//! WebAssembly bindings for the SSQL (Samskrtam SQL) compiler pipeline.
//!
//! Runs the entire SSQL compiler pipeline (`lexer`, `annotator`, `ssql_parser`,
//! `schema`, `validator`, `resolution`, `ast_transformer`, `ast_codegen`,
//! `formatter`, `linter`, and `sql_importer`) synchronously on the browser's
//! main/worker thread without spawning OS threads.

use serde::Serialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use ssql_compiler::annotator::{Annotator, LexerToken, TokenType};
use ssql_compiler::ast::{AstNode, Expression, Statement};
use ssql_compiler::compiler::{Compiler, CompilerConfig};
use ssql_compiler::lexer::tokenize;
use ssql_compiler::morphology::MorphologyEngine;
use ssql_compiler::schema::Schema;
use ssql_compiler::ssql_parser::convert_vibhakti_tokens;
use ssql_compiler::types::{Linga, Vachana, Vibhakti};

#[derive(Serialize, Clone)]
pub struct TokenBreakdown {
    pub surface: String,
    pub category: String,
    pub pratipadika: String,
    pub vibhakti: String,
    pub vachana: String,
    pub linga: String,
    pub karaka_role: String,
    pub sql_mapping: String,
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
}

#[derive(Serialize, Clone)]
pub struct LintItem {
    pub code: String,
    pub line: usize,
    pub message: String,
    pub suggestion: String,
}

#[derive(Serialize, Clone)]
pub struct StructuredDiagnostic {
    pub target: String, // "query" | "schema"
    pub line: usize,    // 1-based line number
    pub start_col: usize, // 1-based UTF-16 column (inclusive)
    pub end_col: usize,   // 1-based UTF-16 column (exclusive)
    pub severity: String, // "error" | "warning"
    pub code: String,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct SchemaColumnSummary {
    pub sanskrit_stem: String,
    pub sql_name: String,
    pub data_type: String,
    pub gender: String,
    pub prathama_eka: String,
    pub shashthi_eka: String,
    pub tritiya_eka: String,
    pub panchami_eka: String,
    pub saptami_eka: String,
}

#[derive(Serialize, Clone)]
pub struct SchemaTableSummary {
    pub sanskrit_stem: String,
    pub sql_name: String,
    pub gender: String,
    pub prathama_bahu: String,
    pub panchami_bahu: String,
    pub tritiya_bahu: String,
    pub saptami_bahu: String,
    pub shashthi_eka: String,
    pub columns: Vec<SchemaColumnSummary>,
}

#[derive(Serialize)]
pub struct CompileResponse {
    pub ok: bool,
    pub sql: String,
    pub errors: Vec<String>,
    pub lint_warnings: Vec<LintItem>,
    pub diagnostics: Vec<StructuredDiagnostic>,
    pub tokens: Vec<TokenBreakdown>,
    pub tables: Vec<SchemaTableSummary>,
}

#[derive(Serialize)]
pub struct TranspileResponse {
    pub ok: bool,
    pub ssql: String,
    pub error: Option<String>,
}

/// Whether a statement is an UPDATE of every row (`<table> सर्वत्र …`), the
/// explicit form the grammar requires in place of a filter.
fn has_unfiltered_update_in_lexer_tokens(tokens: &[LexerToken]) -> bool {
    tokens.split(|t| t.text == "।").any(|stmt| {
        stmt.iter().any(|t| matches!(t.text.as_str(), "परिवर्त्यताम्" | "परिवर्त्येताम्" | "परिवर्त्यन्ताम्"))
            && stmt.iter().any(|t| t.text == "सर्वत्र")
    })
}

/// Load `schema.सं` text into `schema` with the core compiler's own schema
/// loader, so the converter resolves tables exactly as the CLI does.
fn load_schema_main_thread(schema: &mut Schema, schema_source: &str, dialect: &str) -> Result<(), String> {
    if schema_source.trim().is_empty() {
        return Ok(());
    }
    let config = CompilerConfig {
        dialect: dialect.to_string(),
        verbose: false,
        dump_ast: false,
        dump_resolved: false,
        dump_symtable: false,
    };
    let mut compiler = Compiler::new(config).map_err(|e| format!("{:#}", e))?;
    compiler.schema = std::mem::take(schema);
    let loaded = compiler
        .load_schema_from_ssql_string(schema_source)
        .map_err(|e| format!("{:#}", e));
    *schema = std::mem::take(&mut compiler.schema);
    loaded
}

fn describe_karaka(v: Vibhakti, vachana: Vachana, surface: &str) -> &'static str {
    match v {
        Vibhakti::Prathama => {
            if surface.ends_with("ताम्") || surface.ends_with("न्ताम्") {
                "क्रियापदम् (Karmaṇi Loṭ Verb)"
            } else {
                "कर्तृ / कर्म (SELECT Projection / Nominal Head)"
            }
        }
        Vibhakti::Dvitiya => "कर्म (Direct Object / Target)",
        Vibhakti::Tritiya => match vachana {
            Vachana::Bahu => "करणम् / सहार्थे तृतीया (JOIN Table / Instrumental Plural)",
            _ => "करणम् (WHERE / GROUP BY / ORDER BY / Multiplier)",
        },
        Vibhakti::Chaturthi => "सम्प्रदानम् (Dative / Recipient)",
        Vibhakti::Panchami => match vachana {
            Vachana::Bahu => "अपादानम् (FROM Source Table — पञ्चमी बहुवचनम्)",
            _ => "अपादानम् (FROM CTE/View or 'Than' Comparison — पञ्चमी)",
        },
        Vibhakti::Shashthi => "सम्बन्धः (Table.Column Qualifier or Function/Aggregate Argument)",
        Vibhakti::Saptami => match vachana {
            Vachana::Bahu => "अधिकरणम् (INSERT INTO / UPDATE Target Table — सप्तमी बहुवचनम्)",
            _ => "अधिकरणम् (Window PARTITION BY / Locative Context)",
        },
    }
}

fn format_linga(l: Option<Linga>) -> &'static str {
    match l {
        Some(Linga::Masculine) => "पुंलिङ्गम् (Masculine)",
        Some(Linga::Feminine) => "स्त्रीलिङ्गम् (Feminine)",
        Some(Linga::Neuter) => "नपुंसकलिङ्गम् (Neuter)",
        None => "—",
    }
}

fn offset_to_line_col_1based(source: &str, byte_offset: usize) -> (usize, usize) {
    let clamped = byte_offset.min(source.len());
    let prefix = &source[..clamped];
    let line = prefix.chars().filter(|&c| c == '\n').count() + 1;
    let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col_str = &prefix[line_start..];
    let col_utf16: usize = col_str.chars().map(|c| c.len_utf16()).sum();
    (line, col_utf16 + 1)
}

fn line_col_len_utf16(source: &str, line_1based: usize) -> usize {
    source
        .lines()
        .nth(line_1based.saturating_sub(1))
        .map(|l| l.chars().map(|c| c.len_utf16()).sum::<usize>().max(1))
        .unwrap_or(1)
}

fn extract_quoted_tokens(msg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = msg.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let q = chars[i];
        if q == '\'' || q == '"' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != q {
                j += 1;
            }
            if j < chars.len() && j > i + 1 {
                let token_str: String = chars[i + 1..j].iter().collect();
                out.push(token_str);
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn parse_number_after(msg: &str, marker: &str) -> Option<usize> {
    let idx = msg.find(marker)?;
    let rest = &msg[idx + marker.len()..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<usize>().ok()
}

fn parse_span_after(msg: &str, marker: &str) -> Option<(usize, usize)> {
    let idx = msg.find(marker)?;
    let rest = &msg[idx + marker.len()..];
    let start_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let start = start_str.parse::<usize>().ok()?;
    let after_start = &rest[start_str.len()..];
    if !after_start.starts_with("..") {
        return None;
    }
    let end_str: String = after_start[2..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let end = end_str.parse::<usize>().ok()?;
    Some((start, end))
}

fn build_error_diagnostic(
    target: &str,
    source: &str,
    tokens: &[LexerToken],
    code: &str,
    msg: &str,
    schema: &Schema,
) -> StructuredDiagnostic {
    let (clean_msg, suggestion) = humanize_error_message(msg, source, tokens, schema);

    // 1. Check for parser token span ("at span N..M" or "at N..M")
    let span_opt = parse_span_after(msg, "at span ").or_else(|| parse_span_after(msg, "at "));
    if let Some((start_idx, end_idx)) = span_opt {
        if !tokens.is_empty() {
            let clamped_start = start_idx.min(tokens.len() - 1);
            let clamped_end = end_idx.max(clamped_start + 1).min(tokens.len());
            let start_tok = &tokens[clamped_start];
            let end_tok = &tokens[clamped_end - 1];
            let (line, start_col) = offset_to_line_col_1based(source, start_tok.position);
            let (_, end_col) =
                offset_to_line_col_1based(source, end_tok.position + end_tok.text.len());
            return StructuredDiagnostic {
                target: target.to_string(),
                line,
                start_col,
                end_col: end_col.max(start_col + 1),
                severity: "error".to_string(),
                code: code.to_string(),
                message: clean_msg,
                suggestion,
            };
        }
    }

    // 2. Check for lexer byte offset ("at byte offset N", "at byte N", or "at position N")
    if let Some(byte_pos) = parse_number_after(msg, "at byte offset ")
        .or_else(|| parse_number_after(msg, "at byte "))
        .or_else(|| parse_number_after(msg, "at position "))
    {
        let (line, start_col) = offset_to_line_col_1based(source, byte_pos);
        let char_len = source[byte_pos.min(source.len())..]
            .chars()
            .next()
            .map(|c| c.len_utf16())
            .unwrap_or(1);
        return StructuredDiagnostic {
            target: target.to_string(),
            line,
            start_col,
            end_col: start_col + char_len.max(1),
            severity: "error".to_string(),
            code: code.to_string(),
            message: clean_msg,
            suggestion,
        };
    }

    // 3. Check for quoted token in message (preferring the last matching token in source, e.g. the offending verb/adjective)
    let quoted = extract_quoted_tokens(msg);
    for needle in quoted.iter().rev() {
        if let Some(tok) = tokens.iter().find(|t| &t.text == needle) {
            let (line, start_col) = offset_to_line_col_1based(source, tok.position);
            let (_, end_col) = offset_to_line_col_1based(source, tok.position + tok.text.len());
            return StructuredDiagnostic {
                target: target.to_string(),
                line,
                start_col,
                end_col: end_col.max(start_col + 1),
                severity: "error".to_string(),
                code: code.to_string(),
                message: clean_msg,
                suggestion,
            };
        }
    }

    // 4. Check for "at line N"
    if let Some(line_num) = parse_number_after(msg, "at line ") {
        let line = line_num.max(1);
        let line_len = line_col_len_utf16(source, line);
        return StructuredDiagnostic {
            target: target.to_string(),
            line,
            start_col: 1,
            end_col: line_len + 1,
            severity: "error".to_string(),
            code: code.to_string(),
            message: clean_msg,
            suggestion,
        };
    }

    // Fallback: line 1 full line
    let line_len = line_col_len_utf16(source, 1);
    StructuredDiagnostic {
        target: target.to_string(),
        line: 1,
        start_col: 1,
        end_col: line_len + 1,
        severity: "error".to_string(),
        code: code.to_string(),
        message: clean_msg,
        suggestion,
    }
}

fn suggest_gender_adjective(msg: &str) -> Option<String> {
    let target_gender = if msg.contains("gender Neuter")
        || msg.contains("Gender::Neuter")
        || msg.contains("नपुंसकलिङ्गम्")
    {
        Some(("नपुंसकलिङ्गम् (Neuter)", "म्", "इत्यनेन"))
    } else if msg.contains("gender Feminine")
        || msg.contains("Gender::Feminine")
        || msg.contains("स्त्रीलिङ्गम्")
    {
        Some(("स्त्रीलिङ्गम् (Feminine)", "ा", "इत्यनया"))
    } else if msg.contains("gender Masculine")
        || msg.contains("Gender::Masculine")
        || msg.contains("पुंलिङ्गम्")
    {
        Some(("पुंलिङ्गम् (Masculine)", "ः", "इत्यनेन"))
    } else {
        None
    }?;

    let quoted = extract_quoted_tokens(msg);
    let adj_surface = quoted.first().map(|s| s.as_str()).unwrap_or("");
    let col_name = quoted.get(1).map(|s| s.as_str()).unwrap_or("the column");

    let stems = [
        ("अधिक", ("अधिकः", "अधिका", "अधिकम्")),
        ("न्यून", ("न्यूनः", "न्यूना", "न्यूनम्")),
        ("सम", ("समः", "समा", "समम्")),
        ("तुल्य", ("तुल्यः", "तुल्या", "तुल्यम्")),
        ("भिन्न", ("भिन्नः", "भिन्ना", "भिन्नम्")),
        ("रिक्त", ("रिक्तः", "रिक्ता", "रिक्तम्")),
    ];

    for (stem, (m_form, f_form, n_form)) in stems {
        if adj_surface.starts_with(stem) {
            let correct_form = match target_gender.1 {
                "म्" => n_form,
                "ा" => f_form,
                _ => m_form,
            };
            if stem == "सम" || stem == "तुल्य" {
                return Some(format!(
                    "Use '{} {}' ({}) instead of '{}' to agree in gender with {}.",
                    target_gender.2, correct_form, target_gender.0, adj_surface, col_name
                ));
            }
            return Some(format!(
                "Use '{}' ({}) instead of '{}' to agree in gender with {}.",
                correct_form, target_gender.0, adj_surface, col_name
            ));
        }
    }

    Some(format!(
        "Inflect the comparison adjective in {} to match {}.",
        target_gender.0, col_name
    ))
}

fn suggest_available_tables(schema: &Schema) -> Option<String> {
    let tables_summary = summarize_schema(schema);
    if tables_summary.is_empty() {
        return None;
    }
    let parts: Vec<String> = tables_summary
        .into_iter()
        .take(6)
        .map(|t| format!("{} (FROM: {} [{}])", t.prathama_bahu, t.panchami_bahu, t.sql_name))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(format!("Valid tables in active schema — {}", parts.join(" • ")))
    }
}

fn suggest_available_columns(msg: &str, schema: &Schema) -> Option<String> {
    let tables_summary = summarize_schema(schema);
    if tables_summary.is_empty() {
        return None;
    }

    // Check if specific table names are mentioned in brackets: "table(s) [\"books\"]"
    let mut matched_tables = Vec::new();
    for t in &tables_summary {
        if msg.contains(&format!("\"{}\"", t.sql_name))
            || msg.contains(&format!("'{}'", t.sql_name))
            || msg.contains(&t.sanskrit_stem)
        {
            matched_tables.push(t);
        }
    }
    if matched_tables.is_empty() {
        matched_tables = tables_summary.iter().collect();
    }

    let mut parts = Vec::new();
    for t in matched_tables.into_iter().take(3) {
        let cols: Vec<String> = t
            .columns
            .iter()
            .map(|c| format!("{} ({})", c.prathama_eka, c.sql_name))
            .collect();
        if !cols.is_empty() {
            parts.push(format!(
                "{} [{}]: {}",
                t.prathama_bahu,
                t.sql_name,
                cols.join(", ")
            ));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(format!("Valid columns — {}", parts.join(" • ")))
    }
}

/// Present a compiler message for the editor: the wording is the core
/// compiler's; only byte offsets become the line/column the user sees.
fn humanize_error_message(
    raw_msg: &str,
    source: &str,
    tokens: &[LexerToken],
    schema: &Schema,
) -> (String, Option<String>) {
    let (msg, suggestion) = humanize_error_message_inner(raw_msg, source, tokens, schema);
    (byte_offsets_to_line_col(&msg, source), suggestion)
}

/// Replace every "byte offset N" with "line L, col C" for `source`.
fn byte_offsets_to_line_col(msg: &str, source: &str) -> String {
    const MARKER: &str = "byte offset ";
    let mut out = String::with_capacity(msg.len());
    let mut rest = msg;
    while let Some(pos) = rest.find(MARKER) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + MARKER.len()..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        match digits.parse::<usize>() {
            Ok(byte_pos) => {
                let (line, col) = offset_to_line_col_1based(source, byte_pos.min(source.len()));
                out.push_str(&format!("line {}, col {}", line, col));
                rest = &after[digits.len()..];
            }
            Err(_) => {
                out.push_str(MARKER);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn humanize_error_message_inner(
    raw_msg: &str,
    source: &str,
    tokens: &[LexerToken],
    schema: &Schema,
) -> (String, Option<String>) {
    // 1. Lexer Errors
    let is_lexer_error = raw_msg.starts_with("Lexer Error:")
        || raw_msg.starts_with("Lexer failed:")
        || raw_msg.starts_with("Schema lexer error:");
    let is_known_lexer_shape = raw_msg.contains("Unterminated string literal")
        || raw_msg.contains("ASCII numeral")
        || raw_msg.contains("Unexpected character");
    if is_lexer_error && is_known_lexer_shape {
        if let Some(byte_pos) = parse_number_after(raw_msg, "at byte offset ")
            .or_else(|| parse_number_after(raw_msg, "at byte "))
            .or_else(|| parse_number_after(raw_msg, "at position "))
        {
            let (line, col) = offset_to_line_col_1based(source, byte_pos);
            let ch = source[byte_pos.min(source.len())..]
                .chars()
                .next()
                .unwrap_or('?');
            if raw_msg.contains("Unterminated string literal") {
                return (
                    format!(
                        "Lexer Error (line {}, col {}): Unterminated string literal starting at character {}.",
                        line, col, col
                    ),
                    Some("Close the string literal with a matching double quote (\"...\").".to_string()),
                );
            }
            if raw_msg.contains("ASCII numeral") {
                let quoted = extract_quoted_tokens(raw_msg);
                let ascii_num = quoted.first().map(|s| s.as_str()).unwrap_or("0");
                let deva_num = quoted.get(1).map(|s| s.as_str()).unwrap_or("०");
                return (
                    format!(
                        "Lexer Error (line {}, col {}): ASCII numeral '{}' is not allowed in SSQL; use Devanagari numeral '{}'.",
                        line, col, ascii_num, deva_num
                    ),
                    Some(format!("Replace '{}' with Devanagari '{}'.", ascii_num, deva_num)),
                );
            }
            let clean = format!(
                "Lexer Error (line {}, col {}): unexpected character '{}'.",
                line, col, ch
            );
            let suggestion = if ch == '(' || ch == ')' {
                Some(
                    "SSQL is 100% parenthesis-free; use 'यत् ... इति' for grouping instead of parentheses."
                        .to_string(),
                )
            } else if ch == ';' {
                Some(
                    "Use pūrṇadaṇḍa '।' instead of ';' to terminate SSQL statements.".to_string(),
                )
            } else if ch == ',' {
                Some(
                    "Use distributive 'च' instead of ',' to conjoin items in SSQL.".to_string(),
                )
            } else {
                Some(
                    "Remove the unsupported character or wrap text literals in double quotes (\"...\")."
                        .to_string(),
                )
            };
            return (clean, suggestion);
        }
    }

    // 2. Parser Errors (`ExpectedFound { ... } at span S..E`)
    if raw_msg.contains("ExpectedFound") || raw_msg.starts_with("Parse Error:") || raw_msg.starts_with("Schema parse error:") {
        let span_opt =
            parse_span_after(raw_msg, "at span ").or_else(|| parse_span_after(raw_msg, "at "));
        if raw_msg.contains("found: None") {
            if let Some(last_tok) = tokens.last() {
                let (line, col) = offset_to_line_col_1based(source, last_tok.position);
                let t = last_tok.text.as_str();
                let is_verb = t.ends_with("ताम्")
                    || t.ends_with("न्ताम्")
                    || t.ends_with("येताम्")
                    || matches!(t, "रच्यताम्" | "निवेश्यताम्" | "परिवर्त्यताम्" | "निष्कास्यताम्" | "अभिधीयताम्");
                if is_verb {
                    return (
                        format!(
                            "Syntax Error (line {}, col {}): missing closing pūrṇadaṇḍa '।' after verb '{}'.",
                            line, col, t
                        ),
                        Some(format!("Append ' ।' after '{}' to complete the statement.", t)),
                    );
                }
                return (
                    format!(
                        "Incomplete SSQL statement after '{}' (line {}, col {}): expected projected columns, clause marker ('इति नियमेन', 'इति क्रमेण', 'इति समूहेन'), or action verb followed by '।'.",
                        t, line, col
                    ),
                    Some(
                        "Finish projecting columns with 'च' and end with a Karmaṇi Loṭ verb ('आह्रियताम् ।' for 1 column, 'आह्रियेताम् ।' for 2, 'आह्रियन्ताम् ।' for 3+)."
                            .to_string(),
                    ),
                );
            }
            return (
                "Incomplete SSQL statement: expected a table source and action verb.".to_string(),
                Some("Start with a table source in पञ्चमी बहुवचनम् (e.g., 'पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।').".to_string()),
            );
        }

        if let Some((start_idx, _)) = span_opt {
            if let Some(tok) = tokens.get(start_idx.min(tokens.len().saturating_sub(1))) {
                let (line, col) = offset_to_line_col_1based(source, tok.position);
                return (
                    format!(
                        "Syntax Error near '{}' (line {}, col {}): unexpected token in SSQL clause.",
                        tok.text, line, col
                    ),
                    Some(format!(
                        "Check the case inflection (विभक्तिः) or clause marker ('इति नियमेन', 'इति मेलनेन', 'इति क्रमेण', 'च') before '{}'.",
                        tok.text
                    )),
                );
            }
        }
        return (
            "Syntax Error: unexpected token sequence in SSQL statement.".to_string(),
            Some("Verify that clauses end with their 'इति ...' marker and the statement ends with a verb and '।'.".to_string()),
        );
    }

    // 3. Grammar Concordance / Validator Errors
    if raw_msg.contains("Grammar error:")
        || raw_msg.contains("Grammar Validation Error:")
        || raw_msg.contains("Grammar Warning:")
        || raw_msg.contains("Validation Error:")
    {
        let mut clean = raw_msg
            .replace(
                "Grammar Validation Error: Grammar Warning: ",
                "Grammar Concordance Error: ",
            )
            .replace("Grammar Validation Error: ", "Grammar Concordance Error: ")
            .replace("Grammar error: ", "Grammar Concordance Error: ")
            .replace("Validation Error: Grammar Error: ", "Grammar Concordance Error: ")
            .replace("Gender::Neuter", "नपुंसकलिङ्गम् (Neuter)")
            .replace("Gender::Masculine", "पुंलिङ्गम् (Masculine)")
            .replace("Gender::Feminine", "स्त्रीलिङ्गम् (Feminine)")
            .replace("Number::Singular", "एकवचनम् (Singular)")
            .replace("Number::Dual", "द्विवचनम् (Dual)")
            .replace("Number::Plural", "बहुवचनम् (Plural)")
            .replace("(requires Eka)", "(requires एकवचनम् / Singular)")
            .replace("(requires Dvi)", "(requires द्विवचनम् / Dual)")
            .replace("(requires Bahu)", "(requires बहुवचनम् / Plural)");

        let suggestion = if raw_msg.contains("Adjective Gender Mismatch")
            || raw_msg.contains("does not match the gender of column")
        {
            suggest_gender_adjective(raw_msg)
        } else if raw_msg.contains("Pronoun Gender Mismatch") {
            if raw_msg.contains("gender Feminine") {
                Some("Use the feminine pronoun 'इत्यनया' (or 'इत्यस्याः' for > / <) to agree with the feminine column.".to_string())
            } else {
                Some("Use 'इत्यनेन' (for =) or 'इत्यस्मात्' (for > / <) to agree with the masculine/neuter column.".to_string())
            }
        } else if raw_msg.contains("requires Eka") || raw_msg.contains("requires Number::Singular") {
            Some("Use the singular verb 'आह्रियताम्' (एकवचनम्) when projecting 1 column.".to_string())
        } else if raw_msg.contains("requires Dvi") || raw_msg.contains("requires Number::Dual") {
            Some("Use the dual verb 'आह्रियेताम्' (द्विवचनम्) when projecting 2 columns.".to_string())
        } else if raw_msg.contains("requires Bahu") || raw_msg.contains("requires Number::Plural") {
            Some("Use the plural verb 'आह्रियन्ताम्' (बहुवचनम्) when projecting 3 or more columns.".to_string())
        } else if raw_msg.contains("requires 'च'") {
            Some("Place 'च' after every projected column before the final verb (e.g., 'शीर्षकम् च मूल्यम् च आह्रियेताम् ।').".to_string())
        } else {
            None
        };

        if let Some(ref sug) = suggestion {
            if !clean.contains("Suggestion:") {
                clean = format!("{} — {}", clean, sug);
            }
        }
        return (clean, suggestion);
    }

    // 4. Schema / Table / Column Resolution Errors
    if raw_msg.contains("Cannot resolve table")
        || raw_msg.contains("not found in schema")
        || raw_msg.contains("is not defined in the schema")
        || (raw_msg.contains("Table '") && raw_msg.contains("schema"))
    {
        let mut clean = raw_msg
            .replace("AST Transform Error: Schema Error: ", "Schema Resolution Error: ")
            .replace("AST Transform Error: ", "Schema Resolution Error: ")
            .replace("Codegen Error: ", "Schema Resolution Error: ");
        let suggestion = suggest_available_tables(schema);
        if let Some(ref sug) = suggestion {
            clean = format!("{} ({})", clean, sug);
        }
        return (clean, suggestion);
    }

    if raw_msg.contains("is not defined in table")
        || raw_msg.contains("Cannot resolve column")
        || raw_msg.contains("does not exist in table")
        || raw_msg.contains("AST Transform Error:")
    {
        let mut clean = raw_msg
            .replace("AST Transform Error: Schema Error: ", "Schema Resolution Error: ")
            .replace("AST Transform Error: ", "Schema Resolution Error: ")
            .replace("Codegen Error: ", "Schema Resolution Error: ");
        let suggestion = if raw_msg.contains("is not defined in table")
            || raw_msg.contains("Cannot resolve column")
            || raw_msg.contains("does not exist in table")
        {
            suggest_available_columns(raw_msg, schema)
        } else {
            None
        };
        if let Some(ref sug) = suggestion {
            clean = format!("{} ({})", clean, sug);
        }
        return (clean, suggestion);
    }

    (raw_msg.to_string(), None)
}

fn build_lint_diagnostics(target: &str, source: &str, lint_items: &[LintItem]) -> Vec<StructuredDiagnostic> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::with_capacity(lint_items.len());
    for item in lint_items {
        let line_idx = item.line.saturating_sub(1);
        let line_str = lines.get(line_idx).copied().unwrap_or("");
        let mut start_col = 1usize;
        let mut end_col = line_str.chars().map(|c| c.len_utf16()).sum::<usize>().max(1) + 1;

        if item.code == "LINT-001" || item.code == "SSQL-L001" {
            // Find first ASCII digit run on this line
            let mut col = 1usize;
            let mut run_start = None;
            let mut run_end = 1usize;
            for ch in line_str.chars() {
                let w = ch.len_utf16();
                if ch.is_ascii_digit() {
                    if run_start.is_none() {
                        run_start = Some(col);
                    }
                    run_end = col + w;
                } else if run_start.is_some() {
                    break;
                }
                col += w;
            }
            if let Some(s) = run_start {
                start_col = s;
                end_col = run_end;
            }
        } else if item.code == "LINT-002" {
            if let Some(byte_idx) = line_str.find(['(', ')']) {
                let prefix_utf16: usize = line_str[..byte_idx].chars().map(|c| c.len_utf16()).sum();
                start_col = prefix_utf16 + 1;
                end_col = start_col + 1;
            }
        } else if item.code == "LINT-003" {
            // Extract word from message: "Word-final anusvāra in '<word>'..."
            if let Some(q1) = item.message.find('\'') {
                if let Some(q2_rel) = item.message[q1 + 1..].find('\'') {
                    let word = &item.message[q1 + 1..q1 + 1 + q2_rel];
                    if let Some(byte_idx) = line_str.find(word) {
                        let prefix_utf16: usize = line_str[..byte_idx].chars().map(|c| c.len_utf16()).sum();
                        let word_utf16: usize = word.chars().map(|c| c.len_utf16()).sum();
                        start_col = prefix_utf16 + 1;
                        end_col = start_col + word_utf16.max(1);
                    }
                }
            }
        } else if item.code == "SSQL-L003" {
            if let Some(byte_idx) = line_str.find(';') {
                let prefix_utf16: usize = line_str[..byte_idx].chars().map(|c| c.len_utf16()).sum();
                start_col = prefix_utf16 + 1;
                end_col = start_col + 1;
            }
        }

        out.push(StructuredDiagnostic {
            target: target.to_string(),
            line: item.line.max(1),
            start_col,
            end_col,
            severity: "warning".to_string(),
            code: item.code.clone(),
            message: item.message.clone(),
            suggestion: Some(item.suggestion.clone()),
        });
    }
    out
}

fn infer_comparison_adjective_gender(surface: &str) -> Option<ssql_compiler::types::Linga> {
    if !is_comparison_adjective(surface) {
        return None;
    }
    if surface.ends_with("म्") || surface.ends_with("ानि") || surface.ends_with("े") {
        Some(ssql_compiler::types::Linga::Neuter)
    } else if surface.ends_with("ा") || surface.ends_with("ी") {
        Some(ssql_compiler::types::Linga::Feminine)
    } else if surface.ends_with("ः") || surface.ends_with("ौ") {
        Some(ssql_compiler::types::Linga::Masculine)
    } else {
        None
    }
}

fn resolve_stem_gender(schema: &Schema, stem: &str, surface: &str) -> Option<ssql_compiler::types::Linga> {
    if let Some(adj_g) = infer_comparison_adjective_gender(surface) {
        return Some(adj_g);
    }
    if let Some(t) = schema.tables.get(stem).or_else(|| schema.tables.get(surface)) {
        return Some(t.gender);
    }
    for t in schema.tables.values() {
        if let Some(c) = t.fields.get(stem).or_else(|| t.fields.get(surface)) {
            return Some(c.gender);
        }
    }
    None
}

fn is_comparison_adjective(surface: &str) -> bool {
    matches!(
        surface,
        "अधिकः" | "अधिका" | "अधिकम्" | "अधिकौ" | "अधिके" | "अधिकाः" | "अधिकानि" | "न्यूनः" | "न्यूना" | "न्यूनम्" | "न्यूनौ" | "न्यूने" | "न्यूनाः" | "न्यूनानि" | "समः" | "समा" | "समम्" | "समौ" | "समे" | "समाः" | "समानि" | "तुल्यः" | "तुल्या" | "तुल्यम्" | "भिन्नः" | "भिन्ना" | "भिन्नम्" | "भिन्नौ" | "भिन्ने" | "भिन्नाः" | "भिन्नानि" | "रिक्तः" | "रिक्ता" | "रिक्तम्" | "रिक्तौ" | "रिक्ते" | "रिक्ताः" | "रिक्तानि" | "सदृशः" | "सदृशी" | "सदृशम्"
    )
}

fn next_clause_marker_in_statement<'a>(lexer_tokens: &'a [LexerToken], idx: usize) -> Option<&'a str> {
    for tok in lexer_tokens.iter().skip(idx + 1) {
        if matches!(tok.token_type, TokenType::Purnadanda) {
            break;
        }
        match tok.text.as_str() {
            "नियमेन" | "समूहनियमेन" | "मेलनेन" | "वाममेलनेन" | "दक्षिणमेलनेन" | "पूर्णमेलनेन" | "समूहेन" | "क्रमेण" | "मित्या" | "रच्यताम्" | "निवेश्यताम्" | "परिवर्त्यताम्" => return Some(tok.text.as_str()),
            _ => {}
        }
    }
    None
}

fn describe_pronoun_token(surface: &str) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    match surface {
        "इत्यनेन" => Some((
            "तृतीया (Instrumental)",
            "एकवचनम् (Singular)",
            "पुंलिङ्गम्/नपुंसकलिङ्गम् (Masc/Neuter)",
            "करणम् / सहार्थे तृतीया (Equality Comparison Connector — 'with this')",
        )),
        "इत्यनया" => Some((
            "तृतीया (Instrumental)",
            "एकवचनम् (Singular)",
            "स्त्रीलिङ्गम् (Feminine)",
            "करणम् / सहार्थे तृतीया (Equality Comparison Connector — 'with this')",
        )),
        "इत्यस्मात्" => Some((
            "पञ्चमी (Ablative)",
            "एकवचनम् (Singular)",
            "पुंलिङ्गम्/नपुंसकलिङ्गम् (Masc/Neuter)",
            "अपादानम् (Inequality Comparison Connector — 'than this')",
        )),
        "इत्यस्याः" => Some((
            "पञ्चमी / षष्ठी (Ablative / Genitive)",
            "एकवचनम् (Singular)",
            "स्त्रीलिङ्गम् (Feminine)",
            "अपादानम् / सम्बन्धः (Comparison / Aggregate Connector)",
        )),
        "इत्यस्य" => Some((
            "षष्ठी (Genitive)",
            "एकवचनम् (Singular)",
            "पुंलिङ्गम्/नपुंसकलिङ्गम् (Masc/Neuter)",
            "सम्बन्धः (Aggregate / Function Argument Connector — 'of this')",
        )),
        "इत्यनयोः" => Some((
            "षष्ठी / सप्तमी (Genitive / Locative)",
            "द्विवचनम् (Dual)",
            "त्रिलिङ्गम् (All Genders)",
            "द्विवचन-सम्बन्धः (Dual Pair Connector — 'of/between these two')",
        )),
        "इत्येषु" => Some((
            "सप्तमी (Locative)",
            "बहुवचनम् (Plural)",
            "पुंलिङ्गम्/नपुंसकलिङ्गम् (Masc/Neuter)",
            "अधिकरणम् (IN-List Connector — 'among these')",
        )),
        "इत्येषाम्" | "इत्यासाम्" => Some((
            "षष्ठी (Genitive)",
            "बहुवचनम् (Plural)",
            "बहुवचनम् (Plural)",
            "सम्बन्धः (Plural Set / Aggregate Connector — 'of these')",
        )),
        _ => None,
    }
}

fn comparison_adjective_stem(surface: &str) -> Option<&'static str> {
    if surface.starts_with("अधिक") {
        Some("अधिक")
    } else if surface.starts_with("न्यून") {
        Some("न्यून")
    } else if surface.starts_with("सम") {
        Some("सम")
    } else if surface.starts_with("तुल्य") {
        Some("तुल्य")
    } else if surface.starts_with("भिन्न") {
        Some("भिन्न")
    } else if surface.starts_with("रिक्त") {
        Some("रिक्त")
    } else if surface.starts_with("सदृश") {
        Some("सदृश")
    } else {
        None
    }
}

fn is_ssql_verb(surface: &str) -> bool {
    matches!(
        surface,
        "आह्रियताम्" | "आह्रियेताम्" | "आह्रियन्ताम्" | "निवेश्यताम्" | "निवेश्येताम्" | "निवेश्यन्ताम्" | "परिवर्त्यताम्" | "परिवर्त्येताम्" | "परिवर्त्यन्ताम्" | "निष्कास्यताम्" | "निष्कास्यताम्" | "रच्यताम्" | "लुप्यताम्" | "योज्यताम्" | "अभिधीयताम्" | "विशदीक्रियताम्" | "प्रारभ्यताम्" | "स्थिरीक्रियताम्" | "स्थिरीक्रियताम्" | "प्रत्याह्रियताम्" | "प्रत्याह्रियताम्" | "अस्ति" | "स्तः" | "सन्ति" | "नास्ति" | "भवति"
    )
}

fn build_token_breakdowns(
    source: &str,
    lexer_tokens: &[LexerToken],
    enriched_tokens: &[ssql_compiler::types::VibhaktiToken],
    schema: &Schema,
) -> Vec<TokenBreakdown> {
    let mut out = Vec::with_capacity(enriched_tokens.len());
    for (idx, vtok) in enriched_tokens.iter().enumerate() {
        let ltok = lexer_tokens.get(idx);
        let is_comp_adj = is_comparison_adjective(&vtok.text);
        let pronoun_info = describe_pronoun_token(&vtok.text);
        let category = if is_comp_adj {
            "Adjective (तुलना-विशेषणम्)"
        } else if pronoun_info.is_some() {
            "Pronoun (तुलना-सर्वनाम)"
        } else if is_ssql_verb(&vtok.text) {
            "Verb (क्रियापदम्)"
        } else {
            match ltok.map(|t| &t.token_type) {
                Some(TokenType::FetchVerb)
                | Some(TokenType::InsertVerb)
                | Some(TokenType::AddVerb)
                | Some(TokenType::ChangeVerb)
                | Some(TokenType::AbhidhiyatamVerb) => "Verb (क्रियापदम्)",
                Some(TokenType::Number(_)) => "Number (सङ्ख्या)",
                Some(TokenType::String(_)) => "Literal (अक्षरमाला)",
                Some(TokenType::Purnadanda) => "Punctuation (पूर्णदण्डः)",
                Some(TokenType::Identifier) => "Identifier (सुबन्तम् / नामपदम्)",
                _ => "Keyword / Particle (निपातः / उपवाक्यचिह्नम्)",
            }
        };

        let (line, start_col, end_col) = if let Some(lt) = ltok {
            let (l, sc) = offset_to_line_col_1based(source, lt.position);
            let (_, ec) = offset_to_line_col_1based(source, lt.position + lt.text.len());
            (l, sc, ec.max(sc + 1))
        } else {
            (1, 1, 2)
        };

        let stem = if let Some(adj_stem) = comparison_adjective_stem(&vtok.text) {
            adj_stem.to_string()
        } else {
            vtok.pratipadika
                .clone()
                .unwrap_or_else(|| vtok.text.clone())
        };

        let next_tok_text = lexer_tokens.get(idx + 1).map(|t| t.text.as_str()).unwrap_or("");
        let is_math_dvitiya = matches!(next_tok_text, "विशोध्य" | "विभज्य");

        // Prefer Panchami over Chaturthi (`-ेभ्यः` plural), and Prathama over Dvitiya for neuter `-म्` nouns
        let chosen_form = vtok
            .possible_forms
            .iter()
            .find(|f| f.vibhakti == Vibhakti::Panchami)
            .or_else(|| {
                if !is_math_dvitiya {
                    vtok.possible_forms
                        .iter()
                        .find(|f| f.vibhakti == Vibhakti::Prathama)
                } else {
                    None
                }
            })
            .or_else(|| vtok.possible_forms.first());

        let (vibhakti_str, vachana_str, linga_str, karaka_str) =
            if let Some((pvib, pvach, plinga, prole)) = pronoun_info {
                (
                    pvib.to_string(),
                    pvach.to_string(),
                    plinga.to_string(),
                    prole.to_string(),
                )
            } else if category.starts_with("Verb") {
                let is_copula = matches!(
                    vtok.text.as_str(),
                    "अस्ति" | "स्तः" | "सन्ति" | "नास्ति" | "भवति"
                );
                let vachana_hint = if vtok.text.ends_with("येताम्") || vtok.text == "स्तः" {
                    "द्विवचनम् (Dual — 2 items)"
                } else if vtok.text.ends_with("न्ताम्") || vtok.text == "सन्ति" {
                    "बहुवचनम् (Plural — 3+ items)"
                } else {
                    "एकवचनम् (Singular — 1 item)"
                };
                let lakara = if is_copula {
                    "लट् लकारः (Present Indicative)"
                } else {
                    "लोट् लकारः (Imperative)"
                };
                let prayoga = if is_copula {
                    "कर्तरि (Active)"
                } else {
                    "कर्मणि (Passive)"
                };
                let verb_role = if is_copula {
                    "सत्ता-क्रियापदम् (Predicate Copula Verb)"
                } else {
                    "क्रियापदम् (Statement Action Verb)"
                };
                (
                    lakara.to_string(),
                    vachana_hint.to_string(),
                    prayoga.to_string(),
                    verb_role.to_string(),
                )
            } else if let Some(first) = chosen_form {
                let resolved_linga = if is_comp_adj {
                    infer_comparison_adjective_gender(&vtok.text).or(first.linga)
                } else {
                    first
                        .linga
                        .or_else(|| resolve_stem_gender(schema, &stem, &vtok.text))
                };
                let role = if is_comp_adj {
                    "विधेय-विशेषणम् (Comparison Predicate Adjective)"
                } else if matches!(first.vibhakti, Vibhakti::Prathama | Vibhakti::Dvitiya)
                    && !is_math_dvitiya
                    && !vtok.text.ends_with("ताम्")
                    && !vtok.text.ends_with("न्ताम्")
                {
                    match next_clause_marker_in_statement(lexer_tokens, idx) {
                        Some("नियमेन") | Some("समूहनियमेन") => {
                            "विशेष्यम् (Filter Condition Subject — प्रथमा)"
                        }
                        Some("मेलनेन") | Some("वाममेलनेन") | Some("दक्षिणमेलनेन")
                        | Some("पूर्णमेलनेन") => {
                            "विशेष्यम् (JOIN Condition Subject — प्रथमा)"
                        }
                        Some("समूहेन") => "समूह-आधारः (GROUP BY Column — प्रथमा)",
                        Some("क्रमेण") => "क्रम-आधारः (ORDER BY Column / Direction — प्रथमा)",
                        Some("रच्यताम्") => "पटल-स्तम्भः (DDL Table / Column Declaration)",
                        Some("निवेश्यताम्") | Some("परिवर्त्यताम्") => {
                            "स्तम्भ-अभिधानम् (DML Column Target)"
                        }
                        _ => describe_karaka(Vibhakti::Prathama, first.vachana, &vtok.text),
                    }
                } else {
                    describe_karaka(first.vibhakti, first.vachana, &vtok.text)
                };
                (
                    format!("{}", first.vibhakti),
                    format!("{}", first.vachana),
                    format_linga(resolved_linga).to_string(),
                    role.to_string(),
                )
            } else if is_comp_adj {
                let adj_linga = infer_comparison_adjective_gender(&vtok.text);
                (
                    "प्रथमा (Nominative)".to_string(),
                    "एकवचनम् (Singular)".to_string(),
                    format_linga(adj_linga).to_string(),
                    "विधेय-विशेषणम् (Comparison Predicate Adjective)".to_string(),
                )
            } else {
                (
                    "—".to_string(),
                    "—".to_string(),
                    "—".to_string(),
                    category.to_string(),
                )
            };

        let sql_map = if let Some(tbl) = schema
            .resolve_table(&vtok.text)
            .or_else(|| schema.resolve_table(&stem))
        {
            format!("table {}", tbl)
        } else if let Some((col, dtype, _)) = schema
            .resolve_column(&vtok.text, None)
            .or_else(|| schema.resolve_column(&stem, None))
        {
            format!("column {} ({})", col, dtype)
        } else {
            "—".to_string()
        };

        out.push(TokenBreakdown {
            surface: vtok.text.clone(),
            category: category.to_string(),
            pratipadika: stem,
            vibhakti: vibhakti_str,
            vachana: vachana_str,
            linga: linga_str,
            karaka_role: karaka_str,
            sql_mapping: sql_map,
            line,
            start_col,
            end_col,
        });
    }
    out
}

fn summarize_schema(schema: &Schema) -> Vec<SchemaTableSummary> {
    let mut morph = MorphologyEngine::new().ok();
    let mut tables = Vec::new();
    for (stem, tinfo) in &schema.tables {
        let mut cols = Vec::new();
        for (cstem, cinfo) in &tinfo.fields {
            let eka_forms = morph
                .as_mut()
                .and_then(|m| m.decline_with_gender(cstem, cinfo.gender, Vachana::Eka).ok());
            let prathama_eka = eka_forms
                .as_ref()
                .and_then(|t| t.forms.get(&Vibhakti::Prathama))
                .cloned()
                .unwrap_or_else(|| cstem.clone());
            let shashthi_eka = eka_forms
                .as_ref()
                .and_then(|t| t.forms.get(&Vibhakti::Shashthi))
                .cloned()
                .unwrap_or_else(|| format!("{}स्य", cstem));
            let tritiya_eka = eka_forms
                .as_ref()
                .and_then(|t| t.forms.get(&Vibhakti::Tritiya))
                .cloned()
                .unwrap_or_else(|| cstem.clone());
            let panchami_eka = eka_forms
                .as_ref()
                .and_then(|t| t.forms.get(&Vibhakti::Panchami))
                .cloned()
                .unwrap_or_else(|| cstem.clone());
            let saptami_eka = eka_forms
                .as_ref()
                .and_then(|t| t.forms.get(&Vibhakti::Saptami))
                .cloned()
                .unwrap_or_else(|| cstem.clone());

            cols.push(SchemaColumnSummary {
                sanskrit_stem: cstem.clone(),
                sql_name: cinfo.sql_name.clone(),
                data_type: cinfo.sql_type.clone(),
                gender: format_linga(Some(cinfo.gender)).to_string(),
                prathama_eka,
                shashthi_eka,
                tritiya_eka,
                panchami_eka,
                saptami_eka,
            });
        }
        cols.sort_by(|a, b| a.sql_name.cmp(&b.sql_name));

        let bahu_forms = morph
            .as_mut()
            .and_then(|m| m.decline_with_gender(stem, tinfo.gender, Vachana::Bahu).ok());
        let eka_forms = morph
            .as_mut()
            .and_then(|m| m.decline_with_gender(stem, tinfo.gender, Vachana::Eka).ok());

        let prathama_bahu = bahu_forms
            .as_ref()
            .and_then(|t| t.forms.get(&Vibhakti::Prathama))
            .cloned()
            .unwrap_or_else(|| stem.clone());
        let panchami_bahu = bahu_forms
            .as_ref()
            .and_then(|t| t.forms.get(&Vibhakti::Panchami))
            .cloned()
            .unwrap_or_else(|| format!("{}ेभ्यः", stem));
        let tritiya_bahu = bahu_forms
            .as_ref()
            .and_then(|t| t.forms.get(&Vibhakti::Tritiya))
            .cloned()
            .unwrap_or_else(|| format!("{}ैः", stem));
        let saptami_bahu = bahu_forms
            .as_ref()
            .and_then(|t| t.forms.get(&Vibhakti::Saptami))
            .cloned()
            .unwrap_or_else(|| format!("{}ेषु", stem));
        let shashthi_eka = eka_forms
            .as_ref()
            .and_then(|t| t.forms.get(&Vibhakti::Shashthi))
            .cloned()
            .unwrap_or_else(|| format!("{}स्य", stem));

        tables.push(SchemaTableSummary {
            sanskrit_stem: stem.clone(),
            sql_name: tinfo.sql_name.clone(),
            gender: format_linga(Some(tinfo.gender)).to_string(),
            prathama_bahu,
            panchami_bahu,
            tritiya_bahu,
            saptami_bahu,
            shashthi_eka,
            columns: cols,
        });
    }
    tables.sort_by(|a, b| a.sql_name.cmp(&b.sql_name));
    tables
}

/// Compile an SSQL query + optional `schema.सं` DDL string to SQL and return
/// a rich JSON response containing generated SQL, errors, lint diagnostics,
/// span-accurate structured diagnostics, Adhyetā token breakdowns, and resolved schema tables.
///
/// Whether the query is accepted, and the SQL it compiles to, come only from
/// the core compiler pipeline the CLI runs (`Compiler::load_schema_from_ssql_string`
/// + `Compiler::compile_string`): the playground has no grammar of its own.
/// Everything else in the response (lint display, token breakdown, schema
/// summary, line/column placement of the compiler's message) is presentation.
#[wasm_bindgen]
pub fn compile_ssql_wasm(source: &str, schema_source: &str, dialect: &str) -> String {
    let norm_dialect = match dialect.to_lowercase().as_str() {
        "postgres" | "postgresql" => "postgres",
        _ => "sqlite",
    };

    let mut lint_warnings: Vec<LintItem> = ssql_compiler::linter::lint_ssql_source(source)
        .into_iter()
        .filter(|d| {
            !(d.code == "LINT-003"
                && (d.suggestion.ends_with("नाम्") || d.suggestion.ends_with("णाम्")))
        })
        .map(|d| LintItem {
            code: d.code.to_string(),
            line: d.line,
            message: d.message,
            suggestion: d.suggestion,
        })
        .collect();
    let mut diagnostics = build_lint_diagnostics("query", source, &lint_warnings);

    let config = CompilerConfig {
        dialect: norm_dialect.to_string(),
        verbose: false,
        dump_ast: false,
        dump_resolved: false,
        dump_symtable: false,
    };
    let mut compiler = match Compiler::new(config) {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("{:#}", e);
            let resp = CompileResponse {
                ok: false,
                sql: String::new(),
                errors: vec![msg],
                lint_warnings,
                diagnostics,
                tokens: Vec::new(),
                tables: Vec::new(),
            };
            return serde_json::to_string(&resp).unwrap_or_default();
        }
    };

    if !schema_source.trim().is_empty() {
        if let Err(e) = compiler.load_schema_from_ssql_string(schema_source) {
            let msg = format!("{:#}", e);
            let schema_tokens = tokenize(schema_source).unwrap_or_default();
            let diag = build_error_diagnostic(
                "schema",
                schema_source,
                &schema_tokens,
                "SSQL-SCHEMA",
                &msg,
                &compiler.schema,
            );
            let clean_err = diag.message.clone();
            diagnostics.insert(0, diag);
            let resp = CompileResponse {
                ok: false,
                sql: String::new(),
                errors: vec![clean_err],
                lint_warnings,
                diagnostics,
                tokens: Vec::new(),
                tables: Vec::new(),
            };
            return serde_json::to_string(&resp).unwrap_or_default();
        }
    }

    let result = compiler.compile_string(source);

    // Presentation: the Adhyetā breakdown of the query's tokens against the
    // schema the compiler ended with (including in-file DDL and CTEs).
    let lexer_tokens = tokenize(source).unwrap_or_default();
    let token_breakdowns = match Annotator::new()
        .ok()
        .and_then(|mut a| a.annotate(lexer_tokens.clone(), &compiler.schema).ok())
    {
        Some(enriched) => build_token_breakdowns(source, &lexer_tokens, &enriched, &compiler.schema),
        None => Vec::new(),
    };

    match result {
        Ok(sql) => {
            if has_unfiltered_update_in_lexer_tokens(&lexer_tokens) {
                let item = LintItem {
                    code: "SSQL-LINT-DML".to_string(),
                    line: 1,
                    message: "This UPDATE changes every row in the table ('सर्वत्र', everywhere).".to_string(),
                    suggestion: "Replace 'सर्वत्र' with an 'इति नियमेन' filter to change only some rows.".to_string(),
                };
                let extra = build_lint_diagnostics("query", source, std::slice::from_ref(&item));
                lint_warnings.push(item);
                diagnostics.extend(extra);
            }
            let resp = CompileResponse {
                ok: true,
                sql,
                errors: Vec::new(),
                lint_warnings,
                diagnostics,
                tokens: token_breakdowns,
                tables: summarize_schema(&compiler.schema),
            };
            serde_json::to_string(&resp).unwrap_or_default()
        }
        Err(e) => {
            let msg = format!("{:#}", e);
            let code = if msg.contains("Lexer") {
                "SSQL-LEXER"
            } else if msg.contains("Parse error") {
                "SSQL-PARSER"
            } else if msg.contains("Grammar error") || msg.contains("Grammar Validation Error") {
                "SSQL-CONCORDANCE"
            } else if msg.contains("Code generation failed") {
                "SSQL-CODEGEN"
            } else {
                "SSQL-COMPILER"
            };
            let diag = build_error_diagnostic("query", source, &lexer_tokens, code, &msg, &compiler.schema);
            let clean_err = diag.message.clone();
            diagnostics.insert(0, diag);
            let resp = CompileResponse {
                ok: false,
                sql: String::new(),
                errors: vec![clean_err],
                lint_warnings,
                diagnostics,
                tokens: token_breakdowns,
                tables: summarize_schema(&compiler.schema),
            };
            serde_json::to_string(&resp).unwrap_or_default()
        }
    }
}

fn expand_single_line_dql(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.contains("रच्यताम्")
        || trimmed.contains("रच्यताम्")
    {
        return trimmed.to_string();
    }

    let clause_markers = [
        "इति मेलनेन",
        "इति वाममेलनेन",
        "इति दक्षिणमेलनेन",
        "इति पूर्णमेलनेन",
        "इति नियमेन",
        "इति समूहेन",
        "इति समूहनियमेन",
        "इति क्रमेण",
        "अपसारेण",
        "इति मित्या",
    ];
    let has_clause_marker = clause_markers.iter().any(|m| trimmed.contains(m));
    let cha_count = trimmed.matches(" च ").count();
    if !has_clause_marker && cha_count < 2 {
        return trimmed.to_string();
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() < 4 {
        return trimmed.to_string();
    }

    let mut lines: Vec<String> = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    let mut i = 0usize;

    // 1. If the first word is a Panchami/Saptami table source (`-ेभ्यः`, `-ात्`, `-ेषु`, `-ासु`), put it on line 1
    let first = words[0];
    if first.ends_with("ेभ्यः")
        || first.ends_with("ात्")
        || first.ends_with("भ्यः")
        || first.ends_with("ेषु")
        || first.ends_with("ासु")
        || first.ends_with("षु")
    {
        lines.push(first.to_string());
        i = 1;
    }

    while i < words.len() {
        let w = words[i];
        // Check if `w` is the final action verb followed by `।`
        let is_final_verb = matches!(
            w,
            "आह्रियताम्" | "आह्रियेताम्" | "आह्रियन्ताम्" | "निवेश्यताम्" | "निवेश्येताम्" | "निवेश्यन्ताम्" | "परिवर्त्यताम्" | "परिवर्त्येताम्" | "परिवर्त्यन्ताम्" | "निष्कास्यताम्" | "निष्कास्यताम्"
        ) && (i + 1 == words.len()
            || (i + 2 == words.len() && words[i + 1] == "।"));

        if is_final_verb {
            if !cur.is_empty() {
                lines.push(format!("    {}", cur.join(" ")));
                cur.clear();
            }
            let verb_line = if i + 1 < words.len() {
                format!("{} ।", w)
            } else {
                w.to_string()
            };
            lines.push(verb_line);
            break;
        }

        cur.push(w);

        // Break after `इति <clause_marker>` or `<N> अपसारेण`
        let prev_is_iti = cur.len() >= 2 && cur[cur.len() - 2] == "इति";
        let is_clause_end = (prev_is_iti
            && matches!(
                w,
                "मेलनेन" | "वाममेलनेन" | "दक्षिणमेलनेन" | "पूर्णमेलनेन" | "नियमेन" | "समूहेन" | "समूहनियमेन" | "क्रमेण"
            ))
            || (prev_is_iti
                && w == "मित्या"
                && !(i + 2 < words.len() && words[i + 2] == "अपसारेण"))
            || w == "अपसारेण";

        if is_clause_end {
            lines.push(format!("    {}", cur.join(" ")));
            cur.clear();
        }
        i += 1;
    }

    if !cur.is_empty() {
        lines.push(format!("    {}", cur.join(" ")));
    }

    lines.join("\n")
}

/// Format SSQL source code using the canonical SSQL formatter (`--fmt`).
#[wasm_bindgen]
pub fn format_ssql_wasm(source: &str) -> String {
    let cleaned = source.to_string();
    let non_empty_lines: Vec<&str> = cleaned
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect();

    let expanded = if non_empty_lines.len() == 1 {
        let mut out_lines = Vec::new();
        for l in cleaned.lines() {
            let t = l.trim();
            if !t.is_empty() && !t.starts_with("//") {
                out_lines.push(expand_single_line_dql(t));
            } else {
                out_lines.push(l.to_string());
            }
        }
        out_lines.join("\n")
    } else {
        cleaned
    };

    let base_formatted = ssql_compiler::formatter::format_ssql_source(&expanded);

    // Ensure standalone final verb lines (`आह्रियताम् ।`, `आह्रियेताम् ।`, `आह्रियन्ताम् ।`, etc.) are flush-left
    let mut final_lines = Vec::new();
    for line in base_formatted.lines() {
        let trimmed = line.trim();
        let is_standalone_verb = matches!(
            trimmed,
            "आह्रियताम् ।" | "आह्रियेताम् ।" | "आह्रियन्ताम् ।" | "निवेश्यताम् ।" | "निवेश्येताम् ।" | "निवेश्यन्ताम् ।" | "परिवर्त्यताम् ।" | "परिवर्त्येताम् ।" | "परिवर्त्यन्ताम् ।" | "निष्कास्यताम् ।" | "निष्कास्यताम् ।"
        );
        if is_standalone_verb {
            final_lines.push(trimmed.to_string());
        } else {
            final_lines.push(line.to_string());
        }
    }
    let mut result = final_lines.join("\n");
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Run the SSQL style and linguistic purity linter (`--lint`) and return JSON diagnostics.
#[wasm_bindgen]
pub fn lint_ssql_wasm(source: &str) -> String {
    let cleaned = source;
    let items: Vec<LintItem> = ssql_compiler::linter::lint_ssql_source(cleaned)
        .into_iter()
        .filter(|d| {
            !(d.code == "LINT-003"
                && (d.suggestion.ends_with("नाम्") || d.suggestion.ends_with("णाम्")))
        })
        .map(|d| LintItem {
            code: d.code.to_string(),
            line: d.line,
            message: d.message,
            suggestion: d.suggestion,
        })
        .collect();
    serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
}

/// Reverse-transpile standard SQL into idiomatic SSQL (`--from-sql`) using the active `schema.सं`.
#[wasm_bindgen]
pub fn transpile_sql_to_ssql_wasm(sql_source: &str, schema_source: &str) -> String {
    let mut schema = Schema::default();
    if let Err(e) = load_schema_main_thread(&mut schema, schema_source, "sqlite") {
        let resp = TranspileResponse {
            ok: false,
            ssql: String::new(),
            error: Some(e),
        };
        return serde_json::to_string(&resp).unwrap_or_default();
    }

    let out_blocks: Vec<String> = match ssql_compiler::sql_converter::convert_sql_to_ssql(sql_source, &mut schema) {
        Ok(blocks) => blocks.iter().map(|b| format_ssql_wasm(b).trim().to_string()).collect(),
        Err(e) => {
            let resp = TranspileResponse {
                ok: false,
                ssql: String::new(),
                error: Some(e),
            };
            return serde_json::to_string(&resp).unwrap_or_default();
        }
    };

    let resp = TranspileResponse {
        ok: true,
        ssql: format!("{}\n", out_blocks.join("\n\n")),
        error: None,
    };
    serde_json::to_string(&resp).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_and_transpile_main_thread() {
        let schema = r#"
पुस्तकानि इति पटलम् "books" इति आख्या पुस्तक इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र  
    प्राथमिकी क्रमिका सङ्ख्या इति पूर्णाङ्कः "id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    अनिवार्यम् शीर्षकम् इति वाक्यम् "title" इति आख्या शीर्षक इति नपुंसकलिङ्गम् एकवचनम् च
    अनिवार्यम् मूल्यम् इति वास्तविकः "price" इति आख्या मूल्य इति नपुंसकलिङ्गम् एकवचनम् च
    भाषा इति वाक्यम् "language" इति आख्या भाषा इति स्त्रीलिङ्गम् एकवचनम् च
  इति पटलम् ।
"#;
        let query = "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् च मूल्यम् च आह्रियेताम् ।";
        let res_json = compile_ssql_wasm(query, schema, "sqlite");
        let parsed: serde_json::Value = serde_json::from_str(&res_json).unwrap();
        assert_eq!(parsed["ok"], true, "Expected ok=true, got: {}", res_json);
        let sql = parsed["sql"].as_str().unwrap();
        assert!(sql.contains("SELECT"), "Expected SELECT in SQL: {}", sql);
        assert!(sql.contains("books"), "Expected books in SQL: {}", sql);
        assert!(parsed["tokens"].as_array().unwrap().len() >= 8);

        let rev_json = transpile_sql_to_ssql_wasm("SELECT title, price FROM books WHERE price > 20;", schema);
        let rev: serde_json::Value = serde_json::from_str(&rev_json).unwrap();
        assert_eq!(rev["ok"], true, "Expected transpile ok=true, got: {}", rev_json);
        let ssql_out = rev["ssql"].as_str().unwrap();
        assert!(ssql_out.contains("पुस्तकेभ्यः"), "Expected पुस्तकेभ्यः in SSQL: {}", ssql_out);
    }

    #[test]
    fn test_udf_and_lint_and_format_wasm() {
        let schema = r#"
पुस्तकानि इति पटलम् "books" इति आख्या पुस्तक इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र  
    प्राथमिकी क्रमिका सङ्ख्या इति पूर्णाङ्कः "id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    अनिवार्यम् शीर्षकम् इति वाक्यम् "title" इति आख्या शीर्षक इति नपुंसकलिङ्गम् एकवचनम् च
    अनिवार्यम् मूल्यम् इति वास्तविकः "price" इति आख्या मूल्य इति नपुंसकलिङ्गम् एकवचनम् च
  इति पटलम् ।
"#;
        let udf_query = r#"
करसहितमूल्यम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
    मूल्यम् इति वास्तविकः च करः इति वास्तविकः च दीयेते
    मूल्यम् च करः च इत्यनयोः योगः इति उत्तरम् प्रतिदीयताम्
इति कार्यम् ।

पुस्तकेभ्यः शीर्षकम् च मूल्यम् च ५ च प्रदाय करसहितमूल्यम् सकलमूल्यम् इति आख्या च आह्रियेताम् ।
"#;
        let res_json = compile_ssql_wasm(udf_query, schema, "postgres");
        let parsed: serde_json::Value = serde_json::from_str(&res_json).unwrap();
        assert_eq!(parsed["ok"], true, "Expected UDF ok=true, got: {}", res_json);
        let sql = parsed["sql"].as_str().unwrap();
        assert!(sql.contains("+ (5)"), "Expected UDF SQL: {}", sql);

        let formatted = format_ssql_wasm(udf_query);
        assert!(formatted.contains("करसहितमूल्यम् इति शुद्धम् कार्यम्"));

        let lint_json = lint_ssql_wasm("पुस्तकेभ्यः मूल्यम् 20 इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम्");
        let lint_arr: serde_json::Value = serde_json::from_str(&lint_json).unwrap();
        assert!(lint_arr.as_array().unwrap().len() >= 2);
    }

    #[test]
    fn test_all_playground_scenarios() {
        let schema = include_str!("../../../examples/university_bookstore/SAMSQL/schema.सं");
        let scenarios = [
            (
                "filter",
                "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् न्यूनम् अस्ति इति नियमेन शीर्षकम् च मूल्यम् च आह्रियेताम् ।",
                true,
            ),
            (
                "join_agg",
                "विक्रयणेभ्यः आपणैः सह विक्रयणस्य आपणसङ्ख्या आपणस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन आपणस्य नाम इति समूहेन आपणस्य नाम च विक्रयणस्य सकलमूल्यस्य योगः च आह्रियेताम् ।",
                true,
            ),
            (
                "left_join",
                "पुस्तकेभ्यः समीक्षाभिः सह पुस्तकस्य सङ्ख्या समीक्षायाः पुस्तकसङ्ख्या इत्यनया समा अस्ति इति वाममेलनेन समीक्षायाः सङ्ख्या रिक्ता अस्ति इति नियमेन पुस्तकस्य सङ्ख्या च पुस्तकस्य शीर्षकम् च आह्रियेताम् ।",
                true,
            ),
            (
                "having_order",
                "आपणेभ्यः विक्रयणैः सह आपणस्य सङ्ख्या विक्रयणस्य आपणसङ्ख्या इत्यनया समा अस्ति इति मेलनेन आपणस्य नाम इति समूहेन विक्रयणस्य सकलमूल्यस्य योगः ३० इत्यस्मात् अधिकः अस्ति इति समूहनियमेन विक्रयणस्य सकलमूल्यस्य योगः अवरोहः इति क्रमेण आपणस्य नाम च विक्रयणस्य सकलमूल्यस्य योगः आपणराजस्वम् इति आख्या च आह्रियेताम् ।",
                true,
            ),
            (
                "window_rank",
                "पुस्तकेभ्यः शीर्षकम् च भाषा च मूल्यम् च श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण मूल्यश्रेणी इति आख्या च आह्रियन्ताम् ।",
                true,
            ),
            (
                "cte_join",
                "पुस्तकसमीक्षासारः इति आख्या पुस्तकसमीक्षासार इति पुंलिङ्गम् एकवचनम् समीक्षाभ्यः पुस्तकसङ्ख्या इति समूहेन पुस्तकसङ्ख्या च सङ्ख्यायाः गणना समीक्षागणना इति आख्या च आह्रियेताम् । पुस्तकेभ्यः पुस्तकसमीक्षासारेण सह पुस्तकस्य सङ्ख्या पुस्तकसमीक्षासारस्य पुस्तकसङ्ख्या इत्यनया समा अस्ति इति मेलनेन पुस्तकस्य शीर्षकम् च पुस्तकसमीक्षासारस्य समीक्षागणना च आह्रियेताम् ।",
                true,
            ),
            (
                "pure_udf",
                "करसहितमूल्यम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च मूल्यम् इति वास्तविकः च करः इति वास्तविकः च दीयेते मूल्यम् च करः च इत्यनयोः योगः इति उत्तरम् प्रतिदीयताम् इति कार्यम् । पुस्तकेभ्यः शीर्षकम् च मूल्यम् च ५ च प्रदाय करसहितमूल्यम् सकलमूल्यम् इति आख्या च आह्रियेताम् ।",
                true,
            ),
            (
                "concordance_err",
                "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकः अस्ति इति नियमेन शीर्षकम् च मूल्यम् च आह्रियताम् ।",
                false,
            ),
        ];

        for (name, query, expected_ok) in scenarios {
            let res_json = compile_ssql_wasm(query, schema, "sqlite");
            let parsed: serde_json::Value = serde_json::from_str(&res_json).unwrap();
            assert_eq!(
                parsed["ok"], expected_ok,
                "Scenario '{}' expected ok={}, got: {}",
                name, expected_ok, res_json
            );
            if !expected_ok {
                let diags = parsed["diagnostics"].as_array().unwrap();
                assert!(!diags.is_empty(), "Expected structured diagnostics on error");
                assert_eq!(diags[0]["line"], 1);
                assert!(diags[0]["start_col"].as_u64().unwrap() >= 1);
            } else {
                let tables = parsed["tables"].as_array().unwrap();
                let books = tables.iter().find(|t| t["sql_name"] == "books").unwrap();
                assert_eq!(books["panchami_bahu"], "पुस्तकेभ्यः");
                assert_eq!(books["tritiya_bahu"], "पुस्तकैः");
                assert_eq!(books["shashthi_eka"], "पुस्तकस्य");
            }
        }
    }

    #[test]
    fn test_humanized_diagnostics_and_order_by_alias() {
        let schema = include_str!("../../../examples/university_bookstore/SAMSQL/schema.सं");

        // 1. Parser incomplete statement -> no ExpectedFound raw debug
        let p_json = compile_ssql_wasm("पुस्तकेभ्यः", schema, "sqlite");
        let p_val: serde_json::Value = serde_json::from_str(&p_json).unwrap();
        let p_err = p_val["errors"][0].as_str().unwrap();
        assert!(!p_err.contains("ExpectedFound"), "Got raw parser error: {}", p_err);
        assert!(p_err.contains("Incomplete statement"), "{}", p_err);

        // 2. Gender mismatch -> suggests 'अधिकम्' and has no duplicate 'Grammar Validation Error: Grammar Warning:'
        let g_json = compile_ssql_wasm(
            "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकः अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।",
            schema,
            "sqlite",
        );
        let g_val: serde_json::Value = serde_json::from_str(&g_json).unwrap();
        let g_err = g_val["errors"][0].as_str().unwrap();
        assert!(!g_err.contains("Grammar Validation Error: Grammar Warning:"));
        assert!(g_err.contains("अधिकम्"), "Expected gender suggestion in: {}", g_err);

        // 3. Unknown column -> lists valid columns of books
        let c_json = compile_ssql_wasm("पुस्तकेभ्यः अज्ञातम् आह्रियताम् ।", schema, "sqlite");
        let c_val: serde_json::Value = serde_json::from_str(&c_json).unwrap();
        let c_err = c_val["errors"][0].as_str().unwrap();
        assert!(c_err.contains("शीर्षकम् (title)"), "Expected valid columns in: {}", c_err);

        // 4. SQL -> SSQL with ORDER BY <alias>
        let rev_json = transpile_sql_to_ssql_wasm(
            "SELECT language, COUNT(*) AS n FROM books GROUP BY language ORDER BY n DESC;",
            schema,
        );
        let rev: serde_json::Value = serde_json::from_str(&rev_json).unwrap();
        assert_eq!(rev["ok"], true, "ORDER BY alias transpile failed: {}", rev_json);
    }

    #[test]
    fn test_second_review_wasm_fixes() {
        let schema = include_str!("../../../examples/university_bookstore/SAMSQL/schema.सं");

        // #1a: UNION & UNION ALL & INTERSECT & EXCEPT preserve all branches
        let u_json = transpile_sql_to_ssql_wasm(
            "SELECT title FROM books WHERE price < 20 UNION SELECT title FROM books WHERE price > 30;",
            schema,
        );
        let u_val: serde_json::Value = serde_json::from_str(&u_json).unwrap();
        assert_eq!(u_val["ok"], true, "UNION transpile failed: {}", u_json);
        let u_ssql = u_val["ssql"].as_str().unwrap();
        assert!(u_ssql.contains("सङ्गमः"), "Expected सङ्गमः in: {}", u_ssql);
        let u_comp: serde_json::Value =
            serde_json::from_str(&compile_ssql_wasm(u_ssql, schema, "sqlite")).unwrap();
        assert_eq!(u_comp["ok"], true, "UNION compile failed: {}", u_comp);
        assert!(u_comp["sql"].as_str().unwrap().contains("UNION"));

        let ua_json = transpile_sql_to_ssql_wasm(
            "SELECT title FROM books WHERE price < 20 UNION ALL SELECT title FROM books WHERE price > 30;",
            schema,
        );
        let ua_val: serde_json::Value = serde_json::from_str(&ua_json).unwrap();
        assert_eq!(ua_val["ok"], true);
        let ua_ssql = ua_val["ssql"].as_str().unwrap();
        let ua_comp: serde_json::Value =
            serde_json::from_str(&compile_ssql_wasm(ua_ssql, schema, "sqlite")).unwrap();
        assert_eq!(ua_comp["ok"], true, "UNION ALL compile failed: {}", ua_comp);
        assert!(ua_comp["sql"].as_str().unwrap().contains("UNION ALL"));

        // #1b: LIMIT 2 OFFSET 2 preserves OFFSET
        let lo_json = transpile_sql_to_ssql_wasm(
            "SELECT title, price FROM books ORDER BY price DESC LIMIT 2 OFFSET 2;",
            schema,
        );
        let lo_val: serde_json::Value = serde_json::from_str(&lo_json).unwrap();
        assert_eq!(lo_val["ok"], true);
        let lo_ssql = lo_val["ssql"].as_str().unwrap();
        assert!(lo_ssql.contains("अपसारेण"), "Expected अपसारेण in: {}", lo_ssql);
        let lo_comp: serde_json::Value =
            serde_json::from_str(&compile_ssql_wasm(lo_ssql, schema, "sqlite")).unwrap();
        assert_eq!(lo_comp["ok"], true, "LIMIT OFFSET compile failed: {}", lo_comp);
        assert!(lo_comp["sql"].as_str().unwrap().contains("OFFSET 2"));

        // #2: CREATE TABLE in converter works on main thread without panicking
        let ct_json = transpile_sql_to_ssql_wasm(
            "CREATE TABLE t2 (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, price REAL);",
            schema,
        );
        let ct_val: serde_json::Value = serde_json::from_str(&ct_json).unwrap();
        assert_eq!(ct_val["ok"], true, "CREATE TABLE transpile failed: {}", ct_json);
        let ct_ssql = ct_val["ssql"].as_str().unwrap();
        let ct_comp: serde_json::Value =
            serde_json::from_str(&compile_ssql_wasm(ct_ssql, schema, "sqlite")).unwrap();
        assert_eq!(ct_comp["ok"], true, "CREATE TABLE compile failed: {}", ct_comp);
        assert!(ct_comp["sql"].as_str().unwrap().contains("CREATE TABLE"));

        // #6: Verb-less query and danda-less query are rejected
        let no_verb: serde_json::Value =
            serde_json::from_str(&compile_ssql_wasm("लेखकेभ्यः नाम ।", schema, "sqlite")).unwrap();
        assert_eq!(no_verb["ok"], false, "Verb-less query should fail");
        assert!(no_verb["errors"][0].as_str().unwrap().contains("आह्रियताम्"));

        let no_danda: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            "लेखकेभ्यः नाम आह्रियताम्",
            schema,
            "sqlite",
        ))
        .unwrap();
        assert_eq!(no_danda["ok"], false, "Danda-less query should fail");
        assert!(no_danda["errors"][0].as_str().unwrap().contains("।"));

        // #7 & #15: `--` comments and zero-width joiners are accepted
        let dash_comment: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            "-- SQL style comment\nपुस्तकेभ्यः\u{200C}\u{200D} शीर्षकम् आह्रियताम् ।",
            schema,
            "sqlite",
        ))
        .unwrap();
        assert_eq!(
            dash_comment["ok"], true,
            "-- comment + ZWJ should succeed: {}",
            dash_comment
        );

        // #10a: COUNT(DISTINCT language) for feminine column 'भाषा' compiles cleanly
        let cd_json = transpile_sql_to_ssql_wasm(
            "SELECT COUNT(DISTINCT language) FROM books;",
            schema,
        );
        let cd_val: serde_json::Value = serde_json::from_str(&cd_json).unwrap();
        assert_eq!(cd_val["ok"], true);
        let cd_ssql = cd_val["ssql"].as_str().unwrap();
        let cd_comp: serde_json::Value =
            serde_json::from_str(&compile_ssql_wasm(cd_ssql, schema, "sqlite")).unwrap();
        assert_eq!(
            cd_comp["ok"], true,
            "COUNT(DISTINCT language) compile failed: {}",
            cd_comp
        );
        assert!(cd_comp["sql"].as_str().unwrap().contains("COUNT(DISTINCT"));

        // #10b: Window function, CTE, CASE, and comma-join in converter
        let win_json = transpile_sql_to_ssql_wasm(
            "SELECT title, price, RANK() OVER (PARTITION BY language ORDER BY price DESC) AS rnk FROM books;",
            schema,
        );
        let win_val: serde_json::Value = serde_json::from_str(&win_json).unwrap();
        assert_eq!(win_val["ok"], true, "Window transpile failed: {}", win_json);
        assert!(
            win_val["ssql"].as_str().unwrap().contains("श्रेणी"),
            "Expected श्रेणी in window SSQL: {}",
            win_val["ssql"]
        );
        let win_comp: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            win_val["ssql"].as_str().unwrap(),
            schema,
            "sqlite",
        ))
        .unwrap();
        assert_eq!(win_comp["ok"], true, "Window compile failed: {}", win_comp);

        let case_json = transpile_sql_to_ssql_wasm(
            "SELECT title, CASE WHEN price > 25 THEN 'High' ELSE 'Low' END AS tier FROM books;",
            schema,
        );
        let case_val: serde_json::Value = serde_json::from_str(&case_json).unwrap();
        assert_eq!(case_val["ok"], true, "CASE transpile failed: {}", case_json);
        assert!(
            case_val["ssql"].as_str().unwrap().contains("वरणम्"),
            "Expected वरणम् in CASE SSQL: {}",
            case_val["ssql"]
        );
        let case_comp: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            case_val["ssql"].as_str().unwrap(),
            schema,
            "sqlite",
        ))
        .unwrap();
        assert_eq!(case_comp["ok"], true, "CASE compile failed: {}", case_comp);

        let cte_json = transpile_sql_to_ssql_wasm(
            "WITH cheap AS (SELECT title, price FROM books WHERE price < 25) SELECT title, price FROM cheap;",
            schema,
        );
        let cte_val: serde_json::Value = serde_json::from_str(&cte_json).unwrap();
        assert_eq!(cte_val["ok"], true, "CTE transpile failed: {}", cte_json);
        let cte_comp: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            cte_val["ssql"].as_str().unwrap(),
            schema,
            "sqlite",
        ))
        .unwrap();
        assert_eq!(cte_comp["ok"], true, "CTE compile failed: {}", cte_comp);

        // #12: Formatter expands single-line multi-clause query and keeps final verb flush-left
        let fmt_out = format_ssql_wasm(
            "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् च मूल्यम् च आह्रियेताम् ।",
        );
        assert!(fmt_out.contains("\n    मूल्यम् २० इत्यस्मात् अधिकम् अस्ति इति नियमेन\n"));
        assert!(fmt_out.ends_with("\nआह्रियेताम् ।\n"));

        // #13: The playground accepts exactly what the CLI accepts: a DELETE
        // with no subject or filter is not a sentence the grammar defines, so
        // it is rejected here too (no hidden `WHERE 1 = 1` injection).
        // An unfiltered UPDATE compiles and carries the SSQL-LINT-DML warning.
        let del_comp: serde_json::Value =
            serde_json::from_str(&compile_ssql_wasm("पुस्तकेभ्यः निष्कास्यताम् ।", schema, "sqlite"))
                .unwrap();
        assert_eq!(del_comp["ok"], false, "Unfiltered DELETE must match the CLI: {}", del_comp);
        // Deleting everything is said explicitly: "let everything be removed from the books".
        let del_all: serde_json::Value =
            serde_json::from_str(&compile_ssql_wasm("पुस्तकेभ्यः सर्वम् निष्कास्यताम् ।", schema, "sqlite"))
                .unwrap();
        assert_eq!(del_all["ok"], true, "Explicit delete-all failed: {}", del_all);
        assert!(del_all["sql"].as_str().unwrap().contains("DELETE FROM books;"));

        let upd_comp: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            "पुस्तकेषु सर्वत्र मूल्यम् २५ इति परिवर्त्यताम् ।",
            schema,
            "sqlite",
        ))
        .unwrap();
        assert_eq!(upd_comp["ok"], true, "UPDATE of every row failed: {}", upd_comp);
        assert!(!upd_comp["lint_warnings"].as_array().unwrap().is_empty());

        // #14: Unknown table lists valid tables in active schema
        let unk_tbl: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            "अज्ञातेभ्यः शीर्षकम् आह्रियताम् ।",
            schema,
            "sqlite",
        ))
        .unwrap();
        assert_eq!(unk_tbl["ok"], false);
        let unk_err = unk_tbl["errors"][0].as_str().unwrap();
        assert!(
            unk_err.contains("Valid tables in active schema") && unk_err.contains("पुस्तकेभ्यः"),
            "Expected valid tables suggestion in: {}",
            unk_err
        );

        // #17: Token breakdown classifies comparison adjective and filter subject without backticks
        let brk_comp: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।",
            schema,
            "sqlite",
        ))
        .unwrap();
        let toks = brk_comp["tokens"].as_array().unwrap();
        let adhika_tok = toks.iter().find(|t| t["surface"] == "अधिकम्").unwrap();
        assert!(adhika_tok["category"].as_str().unwrap().contains("Adjective"));
        let mulya_tok = toks.iter().find(|t| t["surface"] == "मूल्यम्").unwrap();
        assert!(mulya_tok["karaka_role"].as_str().unwrap().contains("Filter Condition"));
        assert!(!mulya_tok["sql_mapping"].as_str().unwrap().contains('`'));

        // #22: Classical माध्यम् for AVG and unwrapped BETWEEN
        let avg_bet_json = transpile_sql_to_ssql_wasm(
            "SELECT AVG(price) FROM books WHERE price BETWEEN 10 AND 30;",
            schema,
        );
        let avg_bet_val: serde_json::Value = serde_json::from_str(&avg_bet_json).unwrap();
        let avg_bet_ssql = avg_bet_val["ssql"].as_str().unwrap();
        assert!(avg_bet_ssql.contains("माध्यम्"), "Expected माध्यम् in: {}", avg_bet_ssql);
        assert!(
            !avg_bet_ssql.contains("इति इत्यनयोः मध्ये"),
            "Unexpected redundant यत्...इति in BETWEEN: {}",
            avg_bet_ssql
        );
        let avg_bet_comp: serde_json::Value =
            serde_json::from_str(&compile_ssql_wasm(avg_bet_ssql, schema, "sqlite")).unwrap();
        assert_eq!(avg_bet_comp["ok"], true, "AVG+BETWEEN compile failed: {}", avg_bet_comp);
    }

    #[test]
    fn test_third_review_wasm_fixes() {
        let schema = include_str!("../../../examples/university_bookstore/SAMSQL/schema.सं");

        // #2a: 28_delete_inactive_customers.सं compiles cleanly without duplicate WHERE injection
        let del_chet_json = compile_ssql_wasm(
            "ग्राहकेभ्यः सङ्ख्या ५ इत्यस्याः अधिका अस्ति इति नियमेन निष्कास्यताम् ।",
            schema,
            "sqlite",
        );
        let del_chet: serde_json::Value = serde_json::from_str(&del_chet_json).unwrap();
        assert_eq!(
            del_chet["ok"], true,
            "a filtered DELETE should compile cleanly: {}",
            del_chet_json
        );
        assert!(del_chet["sql"]
            .as_str()
            .unwrap()
            .contains("DELETE FROM customers WHERE"));
        assert!(del_chet["lint_warnings"].as_array().unwrap().is_empty());

        // #4: Pure Sanskrit stems in CREATE TABLE, WITH CTE, and preserved output aliases (`AS band`, `AS r`)
        let ct2_json = transpile_sql_to_ssql_wasm(
            "CREATE TABLE t2 (id INTEGER, notes TEXT);",
            schema,
        );
        let ct2: serde_json::Value = serde_json::from_str(&ct2_json).unwrap();
        assert_eq!(ct2["ok"], true);
        let ct2_ssql = ct2["ssql"].as_str().unwrap();
        assert!(
            ct2_ssql.contains("द्वितीयपटलानि") && ct2_ssql.contains("टिप्पणी"),
            "Expected pure Sanskrit stems in CREATE TABLE t2: {}",
            ct2_ssql
        );

        let cheap_json = transpile_sql_to_ssql_wasm(
            "WITH cheap AS (SELECT title, price FROM books WHERE price < 25) SELECT title, price FROM cheap;",
            schema,
        );
        let cheap: serde_json::Value = serde_json::from_str(&cheap_json).unwrap();
        let cheap_ssql = cheap["ssql"].as_str().unwrap();
        assert!(
            // A CTE names a set of rows: declared and read in the plural (K2).
            cheap_ssql.contains("सुलभसाराः") && cheap_ssql.contains("सुलभसारेभ्यः") && !cheap_ssql.contains("अपसार"),
            "Expected the plural CTE सुलभसाराः read as सुलभसारेभ्यः, without अपसार, in: {}",
            cheap_ssql
        );

        let band_json = transpile_sql_to_ssql_wasm(
            "SELECT title, CASE WHEN price > 25 THEN 'High' ELSE 'Low' END AS band FROM books;",
            schema,
        );
        let band: serde_json::Value = serde_json::from_str(&band_json).unwrap();
        let band_comp: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            band["ssql"].as_str().unwrap(),
            schema,
            "sqlite",
        ))
        .unwrap();
        assert_eq!(band_comp["ok"], true, "CASE AS band compile failed: {}", band_comp);
        assert!(
            band_comp["sql"].as_str().unwrap().contains("AS band"),
            "Expected AS band in SQL: {}",
            band_comp["sql"]
        );

        let r_json = transpile_sql_to_ssql_wasm(
            "SELECT title, RANK() OVER (ORDER BY price DESC) AS r FROM books;",
            schema,
        );
        let r_val: serde_json::Value = serde_json::from_str(&r_json).unwrap();
        let r_comp: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            r_val["ssql"].as_str().unwrap(),
            schema,
            "sqlite",
        ))
        .unwrap();
        assert_eq!(r_comp["ok"], true, "RANK AS r compile failed: {}", r_comp);
        assert!(
            r_comp["sql"].as_str().unwrap().contains("AS r"),
            "Expected AS r in SQL: {}",
            r_comp["sql"]
        );

        let rn_json = transpile_sql_to_ssql_wasm(
            "SELECT title, ROW_NUMBER() OVER (ORDER BY price) AS r FROM books;",
            schema,
        );
        let rn_val: serde_json::Value = serde_json::from_str(&rn_json).unwrap();
        let rn_comp: serde_json::Value = serde_json::from_str(&compile_ssql_wasm(
            rn_val["ssql"].as_str().unwrap(),
            schema,
            "sqlite",
        ))
        .unwrap();
        assert_eq!(rn_comp["ok"], true, "ROW_NUMBER AS r compile failed: {} (ssql: {})", rn_comp, rn_val["ssql"]);

        // #9: Lexer errors report character line/col instead of raw byte offsets
        let unclosed_json = compile_ssql_wasm("पुस्तकेभ्यः शीर्षकम् \"अ", schema, "sqlite");
        let unclosed: serde_json::Value = serde_json::from_str(&unclosed_json).unwrap();
        assert_eq!(unclosed["ok"], false);
        let unclosed_err = unclosed["errors"][0].as_str().unwrap();
        assert!(
            unclosed_err.contains("line 1, col ") && !unclosed_err.contains("at byte"),
            "Expected line/col without byte offset: {}",
            unclosed_err
        );

        // #10: Comparison adjective `न्यूनम्` reports gender `नपुंसकलिङ्गम् (Neuter)` instead of `—`
        let nyuna_json = compile_ssql_wasm(
            "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् न्यूनम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।",
            schema,
            "sqlite",
        );
        let nyuna_val: serde_json::Value = serde_json::from_str(&nyuna_json).unwrap();
        let nyuna_tok = nyuna_val["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["surface"] == "न्यूनम्")
            .unwrap();
        assert_eq!(nyuna_tok["linga"], "नपुंसकलिङ्गम् (Neuter)");
    }
}
