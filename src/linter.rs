//! SSQL Style & Linguistic Purity Linter (`--lint`)
//!
//! Enforces SSQL's Devanagari and Pāṇinian stylistic standards beyond core
//! compiler semantics:
//! - `LINT-001 (ascii-numerals)`: Detects ASCII digits (`0-9`) outside SQL
//!   identifier aliases (`"..." इति आख्या`) and recommends Devanagari (`०-९`).
//! - `LINT-002 (legacy-parentheses)`: Detects ASCII parentheses `(` or `)` in
//!   SSQL statements and recommends `यत् ... इति` or clause-level markers.
//! - `LINT-003 (anusvara-halanta)`: Detects word-final anusvāra `ं` on keywords
//!   or neuter nominative/accusative forms (e.g., `सर्वं`, `पुस्तकं`, `आह्रियतां`)
//!   where explicit halanta `म्` (`सर्वम्`, `पुस्तकम्`, `आह्रियताम्`) is canonical.
//! - `LINT-004 (missing-purnadanda)`: Detects statements that do not end with
//!   the Devanagari Pūrṇadaṇḍa (`।`).

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintDiagnostic {
    pub code: &'static str,
    pub line: usize,
    pub message: String,
    pub suggestion: String,
}

impl std::fmt::Display for LintDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] line {}: {} (suggestion: {})",
            self.code, self.line, self.message, self.suggestion
        )
    }
}

/// Lint an SSQL file and return all style/purity diagnostics.
pub fn lint_ssql_file(path: &Path) -> Result<Vec<LintDiagnostic>> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read SSQL file {:?}", path))?;
    Ok(lint_ssql_source(&source))
}

/// Lint an SSQL source string and return all style/purity diagnostics.
pub fn lint_ssql_source(source: &str) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut has_non_comment_content = false;
    let mut last_non_comment_line = (0usize, String::new());

    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        let code_part = strip_line_comment(raw_line).trim();
        if code_part.is_empty() {
            continue;
        }
        has_non_comment_content = true;
        last_non_comment_line = (line_no, code_part.to_string());

        // LINT-001: Check for ASCII digits outside quotes
        let unquoted = strip_quoted_strings(code_part);
        if unquoted.chars().any(|c| c.is_ascii_digit()) {
            let fixed = crate::sql_importer::to_devanagari_numerals(code_part);
            diagnostics.push(LintDiagnostic {
                code: "LINT-001",
                line: line_no,
                message: "ASCII numerals (0-9) found in SSQL code; use Devanagari numerals (०-९)."
                    .to_string(),
                suggestion: fixed,
            });
        }

        // LINT-002: Check for legacy ASCII parentheses outside quotes
        if unquoted.contains('(') || unquoted.contains(')') {
            diagnostics.push(LintDiagnostic {
                code: "LINT-002",
                line: line_no,
                message: "Legacy ASCII parentheses '(' or ')' found; use native Devanagari grouping 'यत् ... इति' or clause delimiters.".to_string(),
                suggestion: code_part
                    .replace('(', "यत् ")
                    .replace(')', " इति"),
            });
        }

        // LINT-003: Check for word-final anusvāra 'ं' on tokens outside quotes
        for token in unquoted.split_whitespace() {
            let clean_tok = token.trim_end_matches('।');
            if clean_tok.ends_with('ं') {
                let mut canonical = clean_tok[..clean_tok.len() - 'ं'.len_utf8()].to_string();
                canonical.push_str("म्");
                diagnostics.push(LintDiagnostic {
                    code: "LINT-003",
                    line: line_no,
                    message: format!(
                        "Word-final anusvāra in '{}' should use explicit halanta 'म्' for canonical Pāṇinian padapāṭha style.",
                        clean_tok
                    ),
                    suggestion: canonical,
                });
            }
        }
    }

    // LINT-004: Ensure the final statement terminates with Pūrṇadaṇḍa (।)
    if has_non_comment_content && !last_non_comment_line.1.ends_with('।') {
        diagnostics.push(LintDiagnostic {
            code: "LINT-004",
            line: last_non_comment_line.0,
            message: "SSQL statement does not end with Pūrṇadaṇḍa ('।').".to_string(),
            suggestion: format!("{} ।", last_non_comment_line.1),
        });
    }

    diagnostics
}

fn strip_line_comment(line: &str) -> &str {
    let mut in_quotes = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            in_quotes = !in_quotes;
        } else if !in_quotes && i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            return &line[..i];
        }
        i += 1;
    }
    line
}

fn strip_quoted_strings(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_quotes = false;
    for c in line.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
            out.push(' ');
        } else if !in_quotes {
            out.push(c);
        }
    }
    out
}
