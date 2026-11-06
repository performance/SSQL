//! Canonical SSQL Code Formatter (`--fmt` and LSP `textDocument/formatting`)
//!
//! Formats `.सं` source files into idiomatic, readable SSQL while preserving
//! `//` comments and string literals:
//! - Normalizes internal whitespace and ensures clean `।` (Pūrṇadaṇḍa) termination.
//! - Indents DDL table definitions (`पटलम् ... रच्यताम् यत्र ... इति पटलम् ।`)
//!   with 4-space column lines and 2-space closing `इति पटलम् ।`.
//! - Indents Pure UDF definitions (`शुद्धम् कार्यम् रच्यताम् यत्र ... इति कार्यम् ।`).
//! - Formats multi-clause DQL queries (`सह ... इति मेलनेन`, `इति नियमेन`, `इति समूहेन`,
//!   `इति समूहनियमेन`, `इति क्रमेण`, `इति मित्या`, and final verb) with consistent
//!   clause-level indentation.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Format an SSQL file in-place or return the formatted string.
pub fn format_ssql_file(path: &Path, write_in_place: bool) -> Result<String> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read SSQL file {:?}", path))?;
    let formatted = format_ssql_source(&source);
    if write_in_place && formatted != source {
        fs::write(path, &formatted)
            .with_context(|| format!("Failed to write formatted SSQL to {:?}", path))?;
    }
    Ok(formatted)
}

/// Format an SSQL source string into canonical layout.
pub fn format_ssql_source(source: &str) -> String {
    let mut out_lines: Vec<String> = Vec::new();
    let mut in_ddl_block = false;
    let mut in_udf_block = false;
    let mut in_dql_continuation = false;
    let mut prev_was_blank = false;

    for raw_line in source.lines() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            if !prev_was_blank && !out_lines.is_empty() {
                out_lines.push(String::new());
            }
            prev_was_blank = true;
            in_dql_continuation = false;
            continue;
        }
        prev_was_blank = false;

        // Preserve comment-only lines with current block indentation
        if trimmed.starts_with("//") || trimmed.starts_with("--") {
            let indent = if in_ddl_block || in_udf_block || in_dql_continuation {
                "    "
            } else {
                ""
            };
            out_lines.push(format!("{}{}", indent, trimmed));
            continue;
        }

        let norm = normalize_line_tokens(trimmed);

        // Check block transitions
        if in_ddl_block {
            if norm.starts_with("इति पटलम्") {
                out_lines.push(format!("  {}", ensure_space_before_danda(&norm)));
                in_ddl_block = false;
            } else {
                out_lines.push(format!("    {}", ensure_space_before_danda(&norm)));
                if norm.contains("इति पटलम्") && norm.ends_with('।') {
                    in_ddl_block = false;
                }
            }
            continue;
        }

        if in_udf_block {
            if norm.starts_with("इति कार्यम्") {
                out_lines.push(ensure_space_before_danda(&norm));
                in_udf_block = false;
            } else {
                out_lines.push(format!("    {}", ensure_space_before_danda(&norm)));
                if norm.contains("इति कार्यम्") && norm.ends_with('।')
                {
                    in_udf_block = false;
                }
            }
            continue;
        }

        // Entering a multi-line DDL table block
        if norm.contains("पटलम्")
            && norm.contains("रच्यताम्")
            && norm.ends_with("यत्र")
            && !norm.ends_with('।')
        {
            out_lines.push(norm);
            in_ddl_block = true;
            in_dql_continuation = false;
            continue;
        }

        // Entering a multi-line Pure UDF block
        if norm.contains("कार्यम्")
            && norm.contains("रच्यताम्")
            && !norm.ends_with('।')
        {
            out_lines.push(norm);
            in_udf_block = true;
            in_dql_continuation = false;
            continue;
        }

        // DQL multi-line indentation
        let formatted_line = ensure_space_before_danda(&norm);
        if in_dql_continuation {
            out_lines.push(format!("    {}", formatted_line));
            if formatted_line.ends_with('।') {
                in_dql_continuation = false;
            }
        } else {
            out_lines.push(formatted_line.clone());
            if !formatted_line.ends_with('।') {
                in_dql_continuation = true;
            }
        }
    }

    let mut result = out_lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Collapse runs of horizontal whitespace outside string literals while preserving trailing `//` comments.
fn normalize_line_tokens(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut in_quotes = false;
    let mut last_was_space = false;

    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            in_quotes = !in_quotes;
            out.push(c);
            last_was_space = false;
            i += 1;
            continue;
        }
        if !in_quotes && c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
            let comment: String = chars[i..].iter().collect();
            out.push_str(comment.trim_end());
            break;
        }
        if !in_quotes && c.is_whitespace() {
            if !last_was_space && !out.is_empty() {
                out.push(' ');
                last_was_space = true;
            }
            i += 1;
            continue;
        }
        out.push(c);
        last_was_space = false;
        i += 1;
    }

    out.trim().to_string()
}

/// Ensure a single space precedes a statement-final `।` (Pūrṇadaṇḍa).
fn ensure_space_before_danda(line: &str) -> String {
    if let Some(stripped) = line.strip_suffix('।') {
        let trimmed = stripped.trim_end();
        format!("{} ।", trimmed)
    } else {
        line.to_string()
    }
}
