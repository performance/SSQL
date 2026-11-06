//! SQL → SSQL conversion, shared by the CLI (`--from-sql`), the playground
//! and the Python bindings, so every front end converts SQL the same way.
//!
//! `sql_importer` turns a single SELECT/DML statement into SSQL; this module
//! handles what comes around it: statement splitting and comments, CREATE
//! TABLE (naming new tables and columns in Sanskrit), CTEs, set operations,
//! window and CASE select items, comma joins and OFFSET.

use std::collections::{HashMap, HashSet};

use crate::annotator::Annotator;
use crate::ast::{AstNode, Expression, Statement};
use crate::compiler::{Compiler, CompilerConfig};
use crate::lexer::tokenize;
use crate::morphology::MorphologyEngine;
use crate::schema::Schema;
use crate::ssql_parser::convert_vibhakti_tokens;
use crate::types::{Vachana, Vibhakti};

/// Load native SSQL DDL into `schema`.
pub fn load_schema(schema: &mut Schema, schema_source: &str, dialect: &str) -> Result<(), String> {
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

/// Convert a SQL script (one or more statements) to SSQL, one block per
/// statement, against `schema`; a CREATE TABLE extends `schema` for the
/// statements after it.
pub fn convert_sql_to_ssql(sql_source: &str, schema: &mut Schema) -> Result<Vec<String>, String> {
    let cleaned = strip_sql_comments(sql_source);
    let statements = split_sql_statements(&cleaned);
    if statements.is_empty() {
        return Err("SQL input is empty.".to_string());
    }
    statements.iter().map(|stmt| transpile_single_select_or_dml(stmt, schema)).collect()
}

fn split_top_level_commas(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    for ch in input.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                cur.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                cur.push(ch);
            }
            '(' if !in_single && !in_double => {
                depth += 1;
                cur.push(ch);
            }
            ')' if !in_single && !in_double => {
                depth = (depth - 1).max(0);
                cur.push(ch);
            }
            ',' if !in_single && !in_double && depth == 0 => {
                parts.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }
    parts
}

fn find_top_level_keyword_ci(input: &str, keyword: &str) -> Option<usize> {
    let chars: Vec<char> = input.chars().collect();
    let kw_chars: Vec<char> = keyword.chars().collect();
    let mut byte_offsets: Vec<usize> = input.char_indices().map(|(b, _)| b).collect();
    byte_offsets.push(input.len());

    let mut depth = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' && !in_double {
            in_single = !in_single;
            i += 1;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            i += 1;
            continue;
        }
        if !in_single && !in_double {
            if ch == '(' {
                depth += 1;
                i += 1;
                continue;
            }
            if ch == ')' {
                depth = (depth - 1).max(0);
                i += 1;
                continue;
            }
            if depth == 0 && i + kw_chars.len() <= chars.len() {
                let prev_ok = i == 0 || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '_';
                let next_ok = i + kw_chars.len() == chars.len()
                    || (!chars[i + kw_chars.len()].is_alphanumeric()
                        && chars[i + kw_chars.len()] != '_');
                if prev_ok && next_ok {
                    let mut matched = true;
                    for (k, &kc) in kw_chars.iter().enumerate() {
                        if !chars[i + k].eq_ignore_ascii_case(&kc) {
                            matched = false;
                            break;
                        }
                    }
                    if matched {
                        return Some(byte_offsets[i]);
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Expand `ORDER BY <select_alias>` references into their underlying `SELECT` expressions
/// so `import_sql_to_ssql` can resolve aggregate/computed aliases in `ORDER BY`.
fn expand_sql_order_by_aliases(sql: &str) -> String {
    let trimmed = sql.trim();
    let Some(sel_pos) = find_top_level_keyword_ci(trimmed, "SELECT") else {
        return sql.to_string();
    };
    let after_sel = &trimmed[sel_pos + 6..];
    let Some(from_rel) = find_top_level_keyword_ci(after_sel, "FROM") else {
        return sql.to_string();
    };
    let select_list_str = after_sel[..from_rel].trim();
    let select_list_clean = if let Some(rest) = select_list_str.strip_prefix("DISTINCT ") {
        rest.trim()
    } else if let Some(rest) = select_list_str.strip_prefix("distinct ") {
        rest.trim()
    } else {
        select_list_str
    };

    let mut alias_map: HashMap<String, String> = HashMap::new();
    for item in split_top_level_commas(select_list_clean) {
        if let Some(as_pos) = find_top_level_keyword_ci(&item, "AS") {
            let expr_part = item[..as_pos].trim();
            let alias_part = item[as_pos + 2..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            if !expr_part.is_empty()
                && !alias_part.is_empty()
                && alias_part.chars().all(|c| c.is_alphanumeric() || c == '_')
            {
                alias_map.insert(alias_part.to_lowercase(), expr_part.to_string());
            }
        }
    }

    if alias_map.is_empty() {
        return sql.to_string();
    }

    let Some(order_pos) = find_top_level_keyword_ci(trimmed, "ORDER") else {
        return sql.to_string();
    };
    let after_order = &trimmed[order_pos + 5..];
    let Some(by_rel) = find_top_level_keyword_ci(after_order, "BY") else {
        return sql.to_string();
    };
    // Make sure BY immediately follows ORDER
    if !after_order[..by_rel].trim().is_empty() {
        return sql.to_string();
    }
    let order_body_start = order_pos + 5 + by_rel + 2;
    let order_rest = &trimmed[order_body_start..];

    // Find where ORDER BY clause ends (LIMIT, OFFSET, or trailing `;`)
    let end_rel = find_top_level_keyword_ci(order_rest, "LIMIT")
        .or_else(|| find_top_level_keyword_ci(order_rest, "OFFSET"))
        .or_else(|| order_rest.rfind(';'))
        .unwrap_or(order_rest.len());

    let order_clause_str = &order_rest[..end_rel];
    let suffix_str = &order_rest[end_rel..];

    let mut replaced_any = false;
    let mut new_order_items = Vec::new();
    for item in split_top_level_commas(order_clause_str) {
        let tokens: Vec<&str> = item.split_whitespace().collect();
        if !tokens.is_empty() {
            let first = tokens[0].trim_matches('"').trim_matches('\'');
            if let Some(target_expr) = alias_map.get(&first.to_lowercase()) {
                replaced_any = true;
                if tokens.len() > 1 {
                    new_order_items.push(format!("{} {}", target_expr, tokens[1..].join(" ")));
                } else {
                    new_order_items.push(target_expr.clone());
                }
                continue;
            }
        }
        new_order_items.push(item);
    }

    if !replaced_any {
        return sql.to_string();
    }

    let prefix = &trimmed[..order_body_start];
    if suffix_str.is_empty() || suffix_str.starts_with(';') {
        format!("{} {}{}", prefix, new_order_items.join(", "), suffix_str)
    } else {
        format!("{} {} {}", prefix, new_order_items.join(", "), suffix_str)
    }
}

fn to_devanagari_digits_str(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0' => '०',
            '1' => '१',
            '2' => '२',
            '3' => '३',
            '4' => '४',
            '5' => '५',
            '6' => '६',
            '7' => '७',
            '8' => '८',
            '9' => '९',
            _ => c,
        })
        .collect()
}

fn strip_sql_comments(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' && !in_double {
            in_single = !in_single;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
            out.push(c);
            i += 1;
            continue;
        }
        if !in_single && !in_double {
            if c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i = (i + 2).min(chars.len());
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut depth = 0i32;
    for c in sql.chars() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(c);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(c);
            }
            '(' if !in_single && !in_double => {
                depth += 1;
                current.push(c);
            }
            ')' if !in_single && !in_double => {
                depth = (depth - 1).max(0);
                current.push(c);
            }
            ';' if !in_single && !in_double && depth == 0 => {
                let t = current.trim().to_string();
                if !t.is_empty() {
                    out.push(t);
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }
    let tail = current.trim().to_string();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

fn clean_sql_ident(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .trim_matches('`')
        .trim_matches('\'')
        .to_string()
}

fn sanskrit_ordinal_prefix(n: usize) -> &'static str {
    match n {
        1 => "प्रथम",
        2 => "द्वितीय",
        3 => "तृतीय",
        4 => "चतुर्थ",
        5 => "पञ्चम",
        6 => "षष्ठ",
        7 => "सप्तम",
        8 => "अष्टम",
        9 => "नवम",
        10 => "दशम",
        _ => "अपर",
    }
}

fn translate_english_word_to_sanskrit(word: &str) -> Option<&'static str> {
    match word.to_lowercase().as_str() {
        "book" | "books" => Some("पुस्तक"),
        "author" | "authors" => Some("लेखक"),
        "customer" | "customers" | "client" | "clients" => Some("ग्राहक"),
        "user" | "users" | "member" | "members" => Some("उपयोक्तृ"),
        "order" | "orders" => Some("आदेश"),
        "sale" | "sales" => Some("विक्रयण"),
        "item" | "items" => Some("वस्तु"),
        "store" | "stores" | "shop" | "shops" => Some("आपण"),
        "review" | "reviews" => Some("समीक्ष"),
        "employee" | "employees" | "staff" | "worker" | "workers" => Some("कर्मचारिन्"),
        "department" | "departments" | "dept" => Some("विभाग"),
        "student" | "students" | "pupil" | "pupils" => Some("छात्र"),
        "course" | "courses" | "subject" | "subjects" => Some("पाठ्यक्रम"),
        "product" | "products" | "good" | "goods" => Some("उत्पाद"),
        "note" | "notes" | "remark" | "remarks" | "memo" | "memos" => Some("टिप्पणी"),
        "comment" | "comments" => Some("टीका"),
        "tag" | "tags" | "label" | "labels" => Some("चिह्न"),
        "log" | "logs" | "history" => Some("वृत्तान्त"),
        "event" | "events" => Some("घटना"),
        "account" | "accounts" => Some("लेखा"),
        "payment" | "payments" => Some("शोधन"),
        "transaction" | "transactions" | "txn" => Some("व्यवहार"),
        "invoice" | "invoices" | "bill" | "bills" => Some("बीजक"),
        "category" | "categories" | "cat" => Some("वर्ग"),
        "group" | "groups" | "team" | "teams" => Some("समूह"),
        "role" | "roles" => Some("भूमिका"),
        "task" | "tasks" | "job" | "jobs" => Some("कार्य"),
        "project" | "projects" => Some("परियोजना"),
        "patient" | "patients" => Some("रोगिन्"),
        "doctor" | "doctors" | "physician" => Some("वैद्य"),
        "hospital" | "hospitals" | "clinic" => Some("चिकित्सालय"),
        "teacher" | "teachers" | "professor" | "professors" | "instructor" => Some("आचार्य"),
        "class" | "classes" => Some("कक्षा"),
        "school" | "schools" | "college" | "university" => Some("विद्यालय"),
        "city" | "cities" | "town" => Some("नगर"),
        "country" | "countries" | "nation" => Some("देश"),
        "address" | "addresses" | "location" => Some("सङ्केत"),
        "message" | "messages" | "msg" => Some("सन्देश"),
        "file" | "files" | "doc" | "document" | "documents" => Some("सञ्चिका"),
        "image" | "images" | "photo" | "photos" => Some("चित्र"),
        "record" | "records" | "row" | "rows" | "entry" | "entries" => Some("पङ्क्ति"),
        "table" | "tables" | "tbl" | "t" => Some("पटल"),
        "temp" | "tmp" | "test" | "demo" | "sample" => Some("परीक्षा"),
        "id" | "code" | "num" | "number" | "no" | "key" => Some("सङ्ख्या"),
        "title" | "heading" => Some("शीर्षक"),
        "name" | "fullname" => Some("नाम"),
        "fname" | "firstname" => Some("प्रथमनाम"),
        "lname" | "lastname" | "surname" => Some("कुलनाम"),
        "price" | "cost" | "fee" | "rate" => Some("मूल्य"),
        "amount" | "val" | "value" | "balance" => Some("राशि"),
        "total" | "sum" | "subtotal" => Some("सकलमूल्य"),
        "quantity" | "qty" | "count" | "cnt" => Some("परिमाण"),
        "language" | "lang" => Some("भाषा"),
        "genre" | "type" | "kind" => Some("विधा"),
        "rating" => Some("मूल्याङ्कन"),
        "score" | "points" | "marks" => Some("अङ्क"),
        "email" | "mail" => Some("विद्युत्पत्र"),
        "phone" | "mobile" | "tel" => Some("दूरवाणी"),
        "status" | "state" => Some("स्थिति"),
        "age" => Some("वयस्"),
        "salary" | "wage" | "income" | "pay" => Some("वेतन"),
        "year" | "yr" => Some("वर्ष"),
        "month" | "mo" => Some("मास"),
        "day" => Some("दिन"),
        "date" | "dt" | "created" | "updated" => Some("दिनाङ्क"),
        "time" | "timestamp" => Some("समय"),
        "text" | "body" | "content" | "description" | "desc" | "info" | "details" | "detail" => {
            Some("विवरण")
        }
        "band" | "tier" | "level" | "grade" => Some("स्तर"),
        "rank" | "rnk" | "pos" | "position" | "r" => Some("श्रेणी"),
        "ratio" | "pct" | "percent" | "percentage" => Some("अनुपात"),
        "discount" => Some("मूल्यह्रास"),
        "tax" => Some("कर"),
        "active" | "enabled" | "valid" => Some("सक्रिय"),
        "cheap" | "affordable" | "budget" => Some("सुलभ"),
        "expensive" | "costly" | "premium" => Some("बहुमूल्य"),
        "high" | "top" | "upper" | "max" | "best" => Some("उच्च"),
        "low" | "bottom" | "min" => Some("निम्न"),
        "new" | "recent" | "latest" => Some("नूतन"),
        "old" | "archived" | "past" => Some("पुरातन"),
        "big" | "large" => Some("बृहत्"),
        "small" | "short" => Some("लघु"),
        "avg" | "average" | "mean" => Some("माध्य"),
        "summary" | "stats" | "agg" | "cte" | "sub" | "q" => Some("सङ्कलन"),
        _ => None,
    }
}

/// Convert an English SQL identifier into a 100% native Sanskrit stem without
/// ever spelling English words letter-by-letter in Devanagari.
fn ascii_to_sanskrit_stem(ident: &str) -> String {
    let trimmed = ident.trim();
    if trimmed.chars().any(|c| ('\u{0900}'..='\u{097F}').contains(&c)) {
        return trimmed.to_string();
    }

    // Separate trailing digits (e.g. `t2` -> base `t`, num `Some(2)`)
    let num_reversed: String = trimmed
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let trailing_num: Option<usize> = if !num_reversed.is_empty() {
        let num_str: String = num_reversed.chars().rev().collect();
        num_str.parse::<usize>().ok()
    } else {
        None
    };
    let base_alpha = trimmed.trim_end_matches(|c: char| c.is_ascii_digit());

    let mut translated_parts: Vec<&'static str> = Vec::new();
    for raw_part in base_alpha.split('_').filter(|p| !p.is_empty()) {
        if let Some(skt) = translate_english_word_to_sanskrit(raw_part) {
            translated_parts.push(skt);
        }
    }

    let mut out = if translated_parts.is_empty() {
        "दत्त".to_string()
    } else {
        translated_parts.join("")
    };

    if let Some(n) = trailing_num {
        let ord = sanskrit_ordinal_prefix(n);
        if out == "पटल" || out == "दत्त" {
            out = format!("{}{}", ord, out);
        } else {
            out = format!("{}{}", ord, out);
        }
    }

    out
}

fn suggest_sanskrit_table(sql_name: &str) -> (String, String, &'static str) {
    match sql_name.to_lowercase().as_str() {
        "books" | "book" => ("पुस्तकानि".into(), "पुस्तक".into(), "नपुंसकलिङ्गम्"),
        "authors" | "author" => ("लेखकाः".into(), "लेखक".into(), "पुंलिङ्गम्"),
        "customers" | "customer" => ("ग्राहकाः".into(), "ग्राहक".into(), "पुंलिङ्गम्"),
        "users" | "user" => ("उपयोक्तारः".into(), "उपयोक्तृ".into(), "पुंलिङ्गम्"),
        "orders" | "order" => ("आदेशाः".into(), "आदेश".into(), "पुंलिङ्गम्"),
        "sales" | "sale" => ("विक्रयणानि".into(), "विक्रयण".into(), "नपुंसकलिङ्गम्"),
        "sale_items" | "order_items" => ("विक्रयणवस्तूनि".into(), "विक्रयणवस्तु".into(), "नपुंसकलिङ्गम्"),
        "stores" | "store" => ("आपणाः".into(), "आपण".into(), "पुंलिङ्गम्"),
        "reviews" | "review" => ("समीक्षाः".into(), "समीक्ष".into(), "स्त्रीलिङ्गम्"),
        "employees" | "employee" => ("कर्मचारिणः".into(), "कर्मचारिन्".into(), "पुंलिङ्गम्"),
        "departments" | "department" => ("विभागाः".into(), "विभाग".into(), "पुंलिङ्गम्"),
        "students" | "student" => ("छात्राः".into(), "छात्र".into(), "पुंलिङ्गम्"),
        "courses" | "course" => ("पाठ्यक्रमाः".into(), "पाठ्यक्रम".into(), "पुंलिङ्गम्"),
        "products" | "product" => ("उत्पादाः".into(), "उत्पाद".into(), "पुंलिङ्गम्"),
        "notes" | "note" => ("टिप्पण्यः".into(), "टिप्पणी".into(), "स्त्रीलिङ्गम्"),
        "comments" | "comment" => ("टीकाः".into(), "टीका".into(), "स्त्रीलिङ्गम्"),
        "tags" | "tag" => ("चिह्नानि".into(), "चिह्न".into(), "नपुंसकलिङ्गम्"),
        "logs" | "log" => ("वृत्तान्ताः".into(), "वृत्तान्त".into(), "पुंलिङ्गम्"),
        "events" | "event" => ("घटनाः".into(), "घटना".into(), "स्त्रीलिङ्गम्"),
        "accounts" | "account" => ("लेखाः".into(), "लेखा".into(), "स्त्रीलिङ्गम्"),
        "payments" | "payment" => ("शोधनानि".into(), "शोधन".into(), "नपुंसकलिङ्गम्"),
        "transactions" | "transaction" => ("व्यवहाराः".into(), "व्यवहार".into(), "पुंलिङ्गम्"),
        "patients" | "patient" => ("रोगिणः".into(), "रोगिन्".into(), "पुंलिङ्गम्"),
        "doctors" | "doctor" => ("वैद्याः".into(), "वैद्य".into(), "पुंलिङ्गम्"),
        "tasks" | "task" => ("कार्याणि".into(), "कार्य".into(), "नपुंसकलिङ्गम्"),
        "projects" | "project" => ("परियोजनाः".into(), "परियोजना".into(), "स्त्रीलिङ्गम्"),
        _ => {
            let base = ascii_to_sanskrit_stem(sql_name);
            let stem = if base.ends_with("पटल") {
                base
            } else {
                format!("{}पटल", base)
            };
            let nom = format!("{}ानि", stem);
            (nom, stem, "नपुंसकलिङ्गम्")
        }
    }
}

fn suggest_sanskrit_column(sql_name: &str) -> (String, String, &'static str) {
    match sql_name.to_lowercase().as_str() {
        "id" => ("सङ्ख्या".into(), "सङ्ख्या".into(), "स्त्रीलिङ्गम्"),
        "book_id" => ("पुस्तकसङ्ख्या".into(), "पुस्तकसङ्ख्या".into(), "स्त्रीलिङ्गम्"),
        "author_id" => ("लेखकसङ्ख्या".into(), "लेखकसङ्ख्या".into(), "स्त्रीलिङ्गम्"),
        "customer_id" => ("ग्राहकसङ्ख्या".into(), "ग्राहकसङ्ख्या".into(), "स्त्रीलिङ्गम्"),
        "store_id" => ("आपणसङ्ख्या".into(), "आपणसङ्ख्या".into(), "स्त्रीलिङ्गम्"),
        "sale_id" => ("विक्रयणसङ्ख्या".into(), "विक्रयणसङ्ख्या".into(), "स्त्रीलिङ्गम्"),
        "user_id" => ("उपयोक्तृसङ्ख्या".into(), "उपयोक्तृसङ्ख्या".into(), "स्त्रीलिङ्गम्"),
        "title" => ("शीर्षकम्".into(), "शीर्षक".into(), "नपुंसकलिङ्गम्"),
        "name" => ("नाम".into(), "नामन्".into(), "नपुंसकलिङ्गम्"),
        "price" | "amount" | "cost" => ("मूल्यम्".into(), "मूल्य".into(), "नपुंसकलिङ्गम्"),
        "total_amount" | "total" => ("सकलमूल्यम्".into(), "सकलमूल्य".into(), "नपुंसकलिङ्गम्"),
        "quantity" | "qty" => ("परिमाणम्".into(), "परिमाण".into(), "नपुंसकलिङ्गम्"),
        "language" | "lang" => ("भाषा".into(), "भाषा".into(), "स्त्रीलिङ्गम्"),
        "genre" | "category" => ("विधा".into(), "विधा".into(), "स्त्रीलिङ्गम्"),
        "rating" => ("मूल्याङ्कनम्".into(), "मूल्याङ्कन".into(), "नपुंसकलिङ्गम्"),
        "score" => ("अङ्कः".into(), "अङ्क".into(), "पुंलिङ्गम्"),
        "city" => ("नगरम्".into(), "नगर".into(), "नपुंसकलिङ्गम्"),
        "country" => ("देशः".into(), "देश".into(), "पुंलिङ्गम्"),
        "email" => ("विद्युत्पत्रम्".into(), "विद्युत्पत्र".into(), "नपुंसकलिङ्गम्"),
        "status" => ("स्थितिः".into(), "स्थिति".into(), "स्त्रीलिङ्गम्"),
        "age" => ("वयः".into(), "वयस्".into(), "नपुंसकलिङ्गम्"),
        "salary" => ("वेतनम्".into(), "वेतन".into(), "नपुंसकलिङ्गम्"),
        "published_year" | "year" => ("प्रकाशनवर्षम्".into(), "प्रकाशनवर्ष".into(), "नपुंसकलिङ्गम्"),
        "date" | "sale_date" | "created_at" => ("दिनाङ्कः".into(), "दिनाङ्क".into(), "पुंलिङ्गम्"),
        "notes" | "note" | "remark" | "remarks" => ("टिप्पणी".into(), "टिप्पणी".into(), "स्त्रीलिङ्गम्"),
        "comment" | "comments" => ("टीका".into(), "टीका".into(), "स्त्रीलिङ्गम्"),
        "description" | "desc" | "text" | "body" | "content" | "details" | "info" => {
            ("विवरणम्".into(), "विवरण".into(), "नपुंसकलिङ्गम्")
        }
        "band" | "tier" | "level" | "grade" => ("स्तरः".into(), "स्तर".into(), "पुंलिङ्गम्"),
        "rank" | "rnk" | "r" | "position" | "pos" => ("श्रेणी".into(), "श्रेणी".into(), "स्त्रीलिङ्गम्"),
        "count" | "cnt" => ("गणना".into(), "गणना".into(), "स्त्रीलिङ्गम्"),
        _ => {
            let base = ascii_to_sanskrit_stem(sql_name);
            let stem = if base == "दत्त" {
                "दत्तस्तम्भ".to_string()
            } else {
                base
            };
            let nom = format!("{}म्", stem);
            (nom, stem, "नपुंसकलिङ्गम्")
        }
    }
}

fn transpile_create_table_main_thread(stmt: &str, schema: &mut Schema) -> Result<String, String> {
    let open_paren = stmt
        .find('(')
        .ok_or_else(|| "CREATE TABLE missing opening '('".to_string())?;
    let close_paren = stmt
        .rfind(')')
        .ok_or_else(|| "CREATE TABLE missing closing ')'".to_string())?;
    if close_paren <= open_paren {
        return Err("Malformed CREATE TABLE parentheses".to_string());
    }

    let header = stmt[..open_paren].trim();
    let table_raw = header
        .split_whitespace()
        .last()
        .ok_or_else(|| "Missing table name in CREATE TABLE".to_string())?;
    let sql_table_name = clean_sql_ident(table_raw);
    let (nom_plural, table_stem, table_linga_kw) = suggest_sanskrit_table(&sql_table_name);

    let cols_body = &stmt[open_paren + 1..close_paren];
    let col_defs = split_top_level_commas(cols_body);

    // Check for table-level PRIMARY KEY (col)
    let mut table_pk_cols: HashSet<String> = HashSet::new();
    for raw_col in &col_defs {
        let up = raw_col.trim().to_uppercase();
        if up.starts_with("PRIMARY KEY") {
            if let (Some(p1), Some(p2)) = (raw_col.find('('), raw_col.rfind(')')) {
                if p2 > p1 {
                    for c in raw_col[p1 + 1..p2].split(',') {
                        table_pk_cols.insert(clean_sql_ident(c).to_lowercase());
                    }
                }
            }
        }
    }

    let mut ssql_cols = Vec::new();
    let mut used_col_stems: HashSet<String> = HashSet::new();

    for (idx, raw_col) in col_defs.iter().enumerate() {
        let trimmed = raw_col.trim();
        if trimmed.is_empty() {
            continue;
        }
        let up_col = trimmed.to_uppercase();
        if up_col.starts_with("PRIMARY KEY")
            || up_col.starts_with("FOREIGN KEY")
            || up_col.starts_with("CONSTRAINT ")
            || up_col.starts_with("UNIQUE ")
            || up_col.starts_with("UNIQUE(")
            || up_col.starts_with("CHECK ")
            || up_col.starts_with("CHECK(")
        {
            continue;
        }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let col_sql_name = clean_sql_ident(tokens[0]);
        let col_type_tok = tokens
            .get(1)
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "TEXT".to_string());
        let base_type = col_type_tok.split('(').next().unwrap_or("TEXT");

        let ssql_type = match base_type {
            "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "SERIAL" | "BIGSERIAL" => "पूर्णाङ्कः",
            "REAL" | "FLOAT" | "DOUBLE" | "NUMERIC" | "DECIMAL" => "वास्तविकः",
            "BOOL" | "BOOLEAN" => "सत्यासत्यः",
            "DATE" => "दिनाङ्कः",
            "TIMESTAMP" | "DATETIME" => "कालचिह्नम्",
            _ => "वाक्यम्",
        };

        let (mut col_nom, mut col_stem, mut col_linga_kw) = suggest_sanskrit_column(&col_sql_name);
        if used_col_stems.contains(&col_stem) {
            col_stem = format!("{}{}", sanskrit_ordinal_prefix(idx + 1), col_stem);
            col_nom = format!("{}म्", col_stem);
            col_linga_kw = "नपुंसकलिङ्गम्";
        }
        used_col_stems.insert(col_stem.clone());

        let is_pk = up_col.contains("PRIMARY KEY")
            || table_pk_cols.contains(&col_sql_name.to_lowercase());
        let is_serial = up_col.contains("AUTOINCREMENT")
            || up_col.contains("AUTO_INCREMENT")
            || base_type == "SERIAL";
        let is_not_null = up_col.contains("NOT NULL");
        let is_unique = up_col.contains("UNIQUE") && !is_pk;

        let mut modifiers = Vec::new();
        if is_pk {
            let pk_adj = match col_linga_kw {
                "स्त्रीलिङ्गम्" => "प्राथमिकी",
                "पुंलिङ्गम्" => "प्राथमिकः",
                _ => "प्राथमिकम्",
            };
            modifiers.push(pk_adj);
        }
        if is_serial {
            let ser_adj = match col_linga_kw {
                "स्त्रीलिङ्गम्" => "क्रमिका",
                "पुंलिङ्गम्" => "क्रमिकः",
                _ => "क्रमिकम्",
            };
            modifiers.push(ser_adj);
        }
        if is_not_null && !is_pk {
            let nn_adj = match col_linga_kw {
                "स्त्रीलिङ्गम्" => "अनिवार्या",
                "पुंलिङ्गम्" => "अनिवार्यः",
                _ => "अनिवार्यम्",
            };
            modifiers.push(nn_adj);
        }
        if is_unique {
            let uq_adj = match col_linga_kw {
                "स्त्रीलिङ्गम्" => "अद्वितीया",
                "पुंलिङ्गम्" => "अद्वितीयः",
                _ => "अद्वितीयम्",
            };
            modifiers.push(uq_adj);
        }

        let mod_prefix = if modifiers.is_empty() {
            String::new()
        } else {
            format!("{} ", modifiers.join(" "))
        };

        ssql_cols.push(format!(
            "    {}{} इति {} \"{}\" इति आख्या {} इति {} एकवचनम्",
            mod_prefix, col_nom, ssql_type, col_sql_name, col_stem, col_linga_kw
        ));
    }

    if ssql_cols.is_empty() {
        return Err("CREATE TABLE contains no column definitions".to_string());
    }

    let ddl_ssql = format!(
        "{} इति पटलम् \"{}\" इति आख्या {} इति {} बहुवचनम् रच्यताम् यत्र\n{}\nइति पटलम् ।",
        nom_plural,
        sql_table_name,
        table_stem,
        table_linga_kw,
        // One column per line; with several, each takes its own च (L1).
        if ssql_cols.len() > 1 {
            ssql_cols.iter().map(|c| format!("{} च", c)).collect::<Vec<_>>().join("\n")
        } else {
            ssql_cols.join("")
        }
    );

    load_schema(schema, &ddl_ssql, "sqlite")?;
    Ok(ddl_ssql)
}

fn rewrite_sql_comma_join(sql: &str) -> Result<String, String> {
    let trimmed = sql.trim();
    let Some(from_pos) = find_top_level_keyword_ci(trimmed, "FROM") else {
        return Ok(sql.to_string());
    };
    let after_from_start = from_pos + 4;
    let after_from = &trimmed[after_from_start..];
    let from_end_rel = [
        "JOIN", "INNER", "LEFT", "RIGHT", "FULL", "CROSS", "WHERE", "GROUP", "HAVING", "ORDER",
        "LIMIT", "OFFSET",
    ]
    .iter()
    .filter_map(|kw| find_top_level_keyword_ci(after_from, kw))
    .min()
    .unwrap_or(after_from.len());

    let from_body = after_from[..from_end_rel].trim();
    let tables = split_top_level_commas(from_body);
    if tables.len() <= 1 {
        return Ok(sql.to_string());
    }
    if tables.len() > 2 {
        return Err(
            "Comma-joins across 3+ tables should use explicit 'JOIN ... ON ...' syntax."
                .to_string(),
        );
    }

    let rest_after_from = &after_from[from_end_rel..];
    let Some(where_rel) = find_top_level_keyword_ci(rest_after_from, "WHERE") else {
        return Err(
            "Comma-join ('FROM t1, t2') requires a WHERE equality join condition (or use 'JOIN ... ON ...')."
                .to_string(),
        );
    };
    let after_where = &rest_after_from[where_rel + 5..];
    let where_end_rel = ["GROUP", "HAVING", "ORDER", "LIMIT", "OFFSET"]
        .iter()
        .filter_map(|kw| find_top_level_keyword_ci(after_where, kw))
        .min()
        .unwrap_or(after_where.len());
    let where_body = after_where[..where_end_rel].trim();
    let tail_clauses = &after_where[where_end_rel..];

    let (join_cond, rem_where) = if let Some(and_pos) = find_top_level_keyword_ci(where_body, "AND")
    {
        (
            where_body[..and_pos].trim(),
            Some(where_body[and_pos + 3..].trim()),
        )
    } else {
        (where_body, None)
    };

    let prefix = &trimmed[..after_from_start];
    let new_where_str = match rem_where {
        Some(w) if !w.is_empty() => format!(" WHERE {}", w),
        _ => String::new(),
    };
    let tail_str = if tail_clauses.trim().is_empty() {
        String::new()
    } else {
        format!(" {}", tail_clauses.trim())
    };
    Ok(format!(
        "{} {} JOIN {} ON {}{}{}",
        prefix, tables[0], tables[1], join_cond, new_where_str, tail_str
    ))
}

fn extract_sql_offset(sql: &str) -> (String, Option<String>) {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    if let Some(off_pos) = find_top_level_keyword_ci(trimmed, "OFFSET") {
        let before = trimmed[..off_pos].trim();
        let after = trimmed[off_pos + 6..].trim();
        let num_tok = after.split_whitespace().next().unwrap_or("");
        if !num_tok.is_empty() && num_tok.chars().all(|c| c.is_ascii_digit()) {
            return (before.to_string(), Some(to_devanagari_digits_str(num_tok)));
        }
    }
    (trimmed.to_string(), None)
}

fn split_top_level_set_ops(sql: &str) -> Result<Option<(Vec<String>, &'static str)>, String> {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    let ops = [
        ("UNION ALL", "सर्वसङ्गमः"),
        ("UNION", "सङ्गमः"),
        ("INTERSECT", "प्रतिच्छेदः"),
        ("EXCEPT", "वर्जनम्"),
    ];

    let mut chosen_ssql_op: Option<&'static str> = None;
    let mut branches: Vec<String> = Vec::new();
    let mut rest = trimmed;

    loop {
        let mut earliest: Option<(usize, usize, &'static str)> = None;
        for (kw, ssql_kw) in ops {
            if let Some(pos) = find_top_level_keyword_ci(rest, kw) {
                match earliest {
                    None => earliest = Some((pos, kw.len(), ssql_kw)),
                    Some((prev_pos, _, _)) if pos < prev_pos => {
                        earliest = Some((pos, kw.len(), ssql_kw))
                    }
                    _ => {}
                }
            }
        }

        if let Some((pos, kw_len, ssql_kw)) = earliest {
            if let Some(prev_op) = chosen_ssql_op {
                if prev_op != ssql_kw {
                    return Err(
                        "Mixing different set operators (e.g. UNION and INTERSECT) in one statement is not supported."
                            .to_string(),
                    );
                }
            } else {
                chosen_ssql_op = Some(ssql_kw);
            }
            branches.push(rest[..pos].trim().to_string());
            rest = rest[pos + kw_len..].trim();
        } else {
            break;
        }
    }

    if let Some(op_kw) = chosen_ssql_op {
        branches.push(rest.to_string());
        Ok(Some((branches, op_kw)))
    } else {
        Ok(None)
    }
}

fn transpile_window_or_case_select_item(
    expr_str: &str,
    alias_opt: Option<&str>,
    from_table_sql: &str,
    schema: &Schema,
) -> Option<String> {
    let mut morph = MorphologyEngine::new().ok()?;
    let table_stem = schema
        .tables
        .iter()
        .find(|(s, info)| {
            info.sql_name.eq_ignore_ascii_case(from_table_sql)
                || s.eq_ignore_ascii_case(from_table_sql)
        })
        .map(|(s, _)| s.clone())?;
    let tinfo = schema.tables.get(&table_stem)?;

    let resolve_col_form = |col_raw: &str, vib: Vibhakti, morph: &mut MorphologyEngine| -> Option<String> {
        let clean = col_raw
            .trim()
            .rsplit('.')
            .next()
            .unwrap_or(col_raw)
            .trim_matches('"')
            .trim_matches('\'');
        let (_, cinfo) = tinfo.fields.iter().find(|(cs, ci)| {
            ci.sql_name.eq_ignore_ascii_case(clean) || cs.eq_ignore_ascii_case(clean)
        })?;
        // By the stem, not the key: a CTE's fields are keyed by their form.
        morph
            .decline_with_gender(&cinfo.pratipadika, cinfo.gender, cinfo.vachana.unwrap_or(Vachana::Eka))
            .ok()
            .and_then(|t| t.forms.get(&vib).cloned())
    };

    let alias_suffix = alias_opt
        .map(|a| {
            if a.chars().any(|c| ('\u{0900}'..='\u{097F}').contains(&c)) {
                format!(" {} इति आख्या", a)
            } else {
                format!(" \"{}\" इति आख्या", a)
            }
        })
        .unwrap_or_default();

    // 1. Check for `<fn>(...) OVER (...)`
    if let Some(over_pos) = find_top_level_keyword_ci(expr_str, "OVER") {
        let fn_part = expr_str[..over_pos].trim();
        let over_part = expr_str[over_pos + 4..].trim();
        if over_part.starts_with('(') && over_part.ends_with(')') {
            let win_inner = over_part[1..over_part.len() - 1].trim();
            let fn_open = fn_part.find('(')?;
            let fn_close = fn_part.rfind(')')?;
            let fn_name = fn_part[..fn_open].trim().to_uppercase();
            let fn_arg = fn_part[fn_open + 1..fn_close].trim();

            let ssql_fn_head = match fn_name.as_str() {
                "RANK" => "श्रेणी".to_string(),
                "DENSE_RANK" => "सघनश्रेणी".to_string(),
                "ROW_NUMBER" => "पङ्क्तिसङ्ख्या".to_string(),
                "LAG" => {
                    let arg_col = fn_arg.split(',').next().unwrap_or(fn_arg);
                    format!("पूर्वपङ्क्तेः {}", resolve_col_form(arg_col, Vibhakti::Prathama, &mut morph)?)
                }
                "LEAD" => {
                    let arg_col = fn_arg.split(',').next().unwrap_or(fn_arg);
                    format!("अनन्तरपङ्क्तेः {}", resolve_col_form(arg_col, Vibhakti::Prathama, &mut morph)?)
                }
                "NTILE" => match fn_arg.trim().parse::<u32>().ok()? {
                    4 => "चतुर्थांशाङ्कः".to_string(),
                    2 => "२ भागयोः भागाङ्कः".to_string(),
                    n if n >= 3 => format!("{} भागेषु भागाङ्कः", crate::sql_importer::to_devanagari_numerals(&n.to_string())),
                    _ => return None,
                },
                "SUM" => format!("{} योगः", resolve_col_form(fn_arg, Vibhakti::Shashthi, &mut morph)?),
                "AVG" => format!("{} माध्यम्", resolve_col_form(fn_arg, Vibhakti::Shashthi, &mut morph)?),
                "COUNT" => {
                    if fn_arg == "*" || fn_arg.is_empty() {
                        "पङ्क्तीनाम् गणना".to_string()
                    } else {
                        format!("{} गणना", resolve_col_form(fn_arg, Vibhakti::Shashthi, &mut morph)?)
                    }
                }
                "MIN" => format!("{} लघुतमम्", resolve_col_form(fn_arg, Vibhakti::Shashthi, &mut morph)?),
                "MAX" => format!("{} महत्तमम्", resolve_col_form(fn_arg, Vibhakti::Shashthi, &mut morph)?),
                _ => return None,
            };

            let mut win_clauses = Vec::new();
            let order_pos_opt = find_top_level_keyword_ci(win_inner, "ORDER");
            let part_pos_opt = find_top_level_keyword_ci(win_inner, "PARTITION");

            if let Some(ppos) = part_pos_opt {
                let after_part = &win_inner[ppos + 9..];
                if let Some(by_pos) = find_top_level_keyword_ci(after_part, "BY") {
                    let p_end = order_pos_opt.unwrap_or(win_inner.len());
                    let p_body = win_inner[ppos + 9 + by_pos + 2..p_end].trim();
                    let mut p_cols = Vec::new();
                    for pc in split_top_level_commas(p_body) {
                        p_cols.push(resolve_col_form(&pc, Vibhakti::Shashthi, &mut morph)?);
                    }
                    // Keys are genitive; several keys are a च-list, each item
                    // with its own च.
                    if p_cols.len() == 1 {
                        win_clauses.push(format!("{} आधारेण विभाजनम्", p_cols[0]));
                    } else if !p_cols.is_empty() {
                        let keys: Vec<String> = p_cols.iter().map(|c| format!("{} च", c)).collect();
                        win_clauses.push(format!("{} आधारेण विभाजनम्", keys.join(" ")));
                    }
                }
            }

            if let Some(opos) = order_pos_opt {
                let after_ord = &win_inner[opos + 5..];
                if let Some(by_pos) = find_top_level_keyword_ci(after_ord, "BY") {
                    let o_all = after_ord[by_pos + 2..].trim();
                    // A frame (`ROWS BETWEEN …`) follows the keys.
                    let frame_pos = find_top_level_keyword_ci(o_all, "ROWS")
                        .or_else(|| find_top_level_keyword_ci(o_all, "RANGE"));
                    let (o_body, frame_sql) = match frame_pos {
                        Some(f) => (o_all[..f].trim(), Some(o_all[f..].trim())),
                        None => (o_all, None),
                    };
                    // Every key, not only the first; SSQL gives a window one
                    // direction for all its keys, so mixed directions are not read.
                    let mut cols = Vec::new();
                    let mut dirs = Vec::new();
                    for item in split_top_level_commas(o_body) {
                        let toks: Vec<&str> = item.split_whitespace().collect();
                        cols.push(resolve_col_form(toks.first()?, Vibhakti::Shashthi, &mut morph)?);
                        dirs.push(toks.get(1).map(|d| d.eq_ignore_ascii_case("DESC")).unwrap_or(false));
                    }
                    if dirs.windows(2).any(|w| w[0] != w[1]) {
                        return None;
                    }
                    let dir = if dirs.first().copied().unwrap_or(false) { "अवरोहक्रमेण" } else { "आरोहक्रमेण" };
                    let keys = if cols.len() == 1 {
                        cols[0].clone()
                    } else {
                        cols.iter().map(|c| format!("{} च", c)).collect::<Vec<_>>().join(" ")
                    };
                    win_clauses.push(format!("{} अनुसारम् {}", keys, dir));
                    // The frame, which used to be dropped (a moving sum became
                    // a running total): from n or all preceding rows to the
                    // current row. Any other frame is not read.
                    if let Some(frame) = frame_sql {
                        let f = frame.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase();
                        let units = if f.starts_with("ROWS ") { "पङ्क्तयः" } else { "व्याप्तिः" };
                        let bounds = f.splitn(2, ' ').nth(1)?;
                        let start = bounds.strip_prefix("BETWEEN ")?.strip_suffix(" AND CURRENT ROW")?;
                        let start_ssql = if start == "UNBOUNDED PRECEDING" {
                            "अपरिमितपूर्ववर्तितः".to_string()
                        } else {
                            let n = start.strip_suffix(" PRECEDING")?;
                            if !n.chars().all(|c| c.is_ascii_digit()) {
                                return None;
                            }
                            format!("{} पूर्ववर्तितः", to_devanagari_digits_str(n))
                        };
                        win_clauses.push(format!("{} {} आरभ्य वर्तमानपङ्क्तिपर्यन्तम्", units, start_ssql));
                    }
                }
            }

            let yatra_part = if win_clauses.is_empty() {
                String::new()
            } else {
                format!(" यत्र {}", win_clauses.join(" "))
            };
            return Some(format!("{}{}{}", ssql_fn_head, yatra_part, alias_suffix));
        }
    }

    // 2. Check for `CASE WHEN ... THEN ... [WHEN ... THEN ...] [ELSE ...] END`
    let up = expr_str.trim().to_uppercase();
    if up.starts_with("CASE ") && up.ends_with("END") {
        let inner = expr_str.trim()[4..expr_str.trim().len() - 3].trim();
        // Every arm: a WHEN condition and its THEN value; the last may carry
        // the ELSE. A second arm used to be read into the first THEN value.
        let mut arms: Vec<(&str, &str)> = Vec::new();
        let mut rest = &inner[find_top_level_keyword_ci(inner, "WHEN")? + 4..];
        let else_val_sql_opt = loop {
            let then_pos = find_top_level_keyword_ci(rest, "THEN")?;
            let cond = rest[..then_pos].trim();
            let after_then = &rest[then_pos + 4..];
            let next_when = find_top_level_keyword_ci(after_then, "WHEN");
            let next_else = find_top_level_keyword_ci(after_then, "ELSE");
            match (next_when, next_else) {
                (Some(w), e) if e.map_or(true, |e| w < e) => {
                    arms.push((cond, after_then[..w].trim()));
                    rest = &after_then[w + 4..];
                }
                (_, Some(e)) => {
                    arms.push((cond, after_then[..e].trim()));
                    break Some(after_then[e + 4..].trim());
                }
                _ => {
                    arms.push((cond, after_then.trim()));
                    break None;
                }
            }
        };

        // Each condition is transpiled through a probe SELECT from `from_table_sql`.
        let first_col_sql = tinfo.fields.values().next()?.sql_name.as_str();
        let transpile_cond = |cond_sql: &str| -> Option<String> {
            let probe_sql = format!("SELECT {} FROM {} WHERE {}", first_col_sql, from_table_sql, cond_sql);
            let probe_ssql = crate::sql_importer::import_sql_to_ssql(&probe_sql, schema).ok()?;
            // `<cond_ssql>` lies between the table's ablative and `इति नियमेन`.
            let niyamena_idx = probe_ssql.find("इति नियमेन")?;
            let before_niyamena = probe_ssql[..niyamena_idx].trim();
            let first_space = before_niyamena.find(char::is_whitespace)?;
            Some(before_niyamena[first_space..].trim().to_string())
        };

        // A branch value is a literal; SQL's `''` is one quote. Anything else
        // (a column, an expression) is not read here, rather than quoted as
        // if it were text.
        let format_lit = |raw: &str| -> Option<String> {
            let t = raw.trim();
            if t.len() >= 2 && t.starts_with('\'') && t.ends_with('\'') {
                Some(format!("\"{}\"", t[1..t.len() - 1].replace("''", "'")))
            } else if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit() || c == '.') {
                Some(to_devanagari_digits_str(t))
            } else {
                None
            }
        };

        let mut case_ssql = String::from("वरणम्");
        for (cond, val) in &arms {
            case_ssql.push_str(&format!(" यस्मिन् {} तस्मिन् {}", transpile_cond(cond)?, format_lit(val)?));
        }
        if let Some(ev) = else_val_sql_opt {
            case_ssql.push_str(&format!(" अन्यस्मिन् {}", format_lit(ev)?));
        }
        // A whole select item needs no `यत् … इति` grouping (example 25).
        case_ssql.push_str(&alias_suffix);
        return Some(case_ssql);
    }

    None
}

fn postprocess_classical_ssql(ssql: &str, offset_deva: Option<&str>) -> String {
    let mut out = ssql.to_string();
    // Unwrap redundant `यत् <low> च <high> च इति इत्यनयोः मध्ये अस्ति` -> `<low> च <high> च इत्यनयोः मध्ये अस्ति`
    while let Some(madhye_pos) = out.find("इति इत्यनयोः मध्ये अस्ति") {
        let prefix = &out[..madhye_pos];
        if let Some(yat_pos) = prefix.rfind("यत् ") {
            let between_inner = prefix[yat_pos + "यत् ".len()..].trim();
            let after_madhye = &out[madhye_pos + "इति ".len()..];
            out = format!("{}{} {}", &out[..yat_pos], between_inner, after_madhye);
        } else {
            break;
        }
    }
    if let Some(off) = offset_deva {
        out = out.replacen("इति मित्या", &format!("इति मित्या {} अपसारेण", off), 1);
    }
    out
}

fn transpile_single_select_or_dml(sql: &str, schema: &mut Schema) -> Result<String, String> {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    let up = trimmed.to_uppercase();

    // 1. Handle `CREATE TABLE` directly on main thread
    if up.starts_with("CREATE TABLE")
        || up.starts_with("CREATE TEMP TABLE")
        || up.starts_with("CREATE TEMPORARY TABLE")
    {
        return transpile_create_table_main_thread(trimmed, schema);
    }

    // 2. Handle `WITH <cte> AS (SELECT ...) <main_query>`
    if up.starts_with("WITH ") && !up.starts_with("WITH RECURSIVE") {
        let after_with = trimmed[5..].trim();
        if let Some(as_pos) = find_top_level_keyword_ci(after_with, "AS") {
            let cte_sql_name = clean_sql_ident(&after_with[..as_pos]);
            let after_as = after_with[as_pos + 2..].trim();
            if after_as.starts_with('(') {
                let mut depth = 0i32;
                let mut close_idx = None;
                for (b_idx, ch) in after_as.char_indices() {
                    if ch == '(' {
                        depth += 1;
                    } else if ch == ')' {
                        depth -= 1;
                        if depth == 0 {
                            close_idx = Some(b_idx);
                            break;
                        }
                    }
                }
                if let Some(c_idx) = close_idx {
                    let cte_inner_sql = after_as[1..c_idx].trim();
                    let main_sql = after_as[c_idx + 1..].trim();
                    let cte_dql_ssql = transpile_single_select_or_dml(cte_inner_sql, schema)?;
                    let base_cte = ascii_to_sanskrit_stem(&cte_sql_name);
                    let cte_stem = if base_cte.ends_with("सार") {
                        base_cte
                    } else {
                        format!("{}सार", base_cte)
                    };
                    // A CTE names a set of rows, so it is plural like a
                    // table (expert review K2).
                    let cte_nom = format!("{}ाः", cte_stem);
                    let cte_decl = format!(
                        "{} इति आख्या {} इति पुंलिङ्गम् बहुवचनम् {}",
                        cte_nom, cte_stem, cte_dql_ssql
                    );

                    // Register CTE columns as a temporary table in `schema` so `main_sql` can resolve `FROM <cte_sql_name>`
                    // and also register via Pass-1 AST CTE registration
                    if let Ok(lexer_tokens) = tokenize(&cte_decl) {
                        if let Ok(mut annotator) = Annotator::new() {
                            if let Ok(enriched) = annotator.annotate(lexer_tokens, schema) {
                                let p_tokens = convert_vibhakti_tokens(enriched);
                                if let Ok(AstNode::QueryBatch { statements, .. }) =
                                    crate::ssql_parser::parse(p_tokens)
                                {
                                    for st in statements {
                                        if let Statement::VarDecl { name, value, .. } = st {
                                            if let Expression::Subquery(dql) = *value {
                                                let default_ctx =
                                                    dql.source.table.pratipadika.as_str();
                                                let joined_ctxs: Vec<String> = dql
                                                    .joins
                                                    .iter()
                                                    .map(|j| j.table.pratipadika.clone())
                                                    .collect();
                                                let _ = schema.register_cte(
                                                    &name,
                                                    &dql.select.columns,
                                                    Some(default_ctx),
                                                    &joined_ctxs,
                                                );
                                                if let Some(tinfo) =
                                                    schema.tables.get_mut(&cte_stem)
                                                {
                                                    tinfo.sql_name = cte_sql_name.clone();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // The CTE is plural, so it is read in the ablative plural.
                    let main_ssql = transpile_single_select_or_dml(main_sql, schema)?;
                    return Ok(format!("{}\n\n{}", cte_decl, main_ssql));
                }
            }
        }
    }

    // 3. Handle top-level `UNION [ALL]`, `INTERSECT`, `EXCEPT`
    if let Some((branches, op_ssql)) = split_top_level_set_ops(trimmed)? {
        let mut branch_ssqls = Vec::with_capacity(branches.len());
        for b in &branches {
            let b_ssql = transpile_single_select_or_dml(b, schema)?;
            let without_danda = b_ssql.trim().trim_end_matches('।').trim().to_string();
            branch_ssqls.push(without_danda);
        }
        let pronoun = if branch_ssqls.len() == 2 {
            "इत्यनयोः"
        } else {
            "इत्येषाम्"
        };
        return Ok(format!(
            "{} च\n{} {} आह्रियताम् ।",
            branch_ssqls.join(" च\n"),
            pronoun,
            op_ssql
        ));
    }

    // 4. Reject nested `(SELECT ...)` subqueries with a helpful CTE suggestion
    if up.contains("(SELECT ") || up.contains("( SELECT ") {
        return Err(
            "Nested '(SELECT ...)' subqueries are not supported in direct SQL->SSQL conversion; rewrite as a named CTE ('WITH cte AS (SELECT ...) SELECT ...')."
                .to_string(),
        );
    }

    // 5. Rewrite comma-joins (`FROM t1, t2 WHERE t1.a = t2.b`) into explicit `JOIN ... ON ...`
    let join_normalized = rewrite_sql_comma_join(trimmed)?;

    // 6. Extract `OFFSET <M>`
    let (without_offset, offset_deva) = extract_sql_offset(&join_normalized);

    // 7. Expand `ORDER BY <alias>`
    let expanded_sql = expand_sql_order_by_aliases(&without_offset);

    // 8. Pre-extract Window (`OVER (...)`) and `CASE WHEN ... END` items in `SELECT` list
    if let Some(sel_pos) = find_top_level_keyword_ci(&expanded_sql, "SELECT") {
        let after_sel = &expanded_sql[sel_pos + 6..];
        if let Some(from_rel) = find_top_level_keyword_ci(after_sel, "FROM") {
            let select_list_str = after_sel[..from_rel].trim();
            let after_from_str = after_sel[from_rel + 4..].trim();
            let from_tbl_raw = after_from_str.split_whitespace().next().unwrap_or("");
            let from_tbl_sql = clean_sql_ident(from_tbl_raw);

            let items = split_top_level_commas(select_list_str);
            let has_special = items.iter().any(|it| {
                find_top_level_keyword_ci(it, "OVER").is_some()
                    || find_top_level_keyword_ci(it, "CASE").is_some()
            });

            if has_special {
                // Find a simple column from `from_tbl_sql` to act as a placeholder during `import_sql_to_ssql`
                let placeholder_col = schema
                    .tables
                    .iter()
                    .find(|(s, info)| {
                        info.sql_name.eq_ignore_ascii_case(&from_tbl_sql)
                            || s.eq_ignore_ascii_case(&from_tbl_sql)
                    })
                    .and_then(|(_, info)| info.fields.values().next())
                    .map(|c| c.sql_name.clone());

                if let Some(pcol) = placeholder_col {
                    let mut special_replacements: Vec<(String, String, String)> = Vec::new();
                    let mut rewritten_items: Vec<String> = Vec::new();
                    let mut all_ok = true;

                    for (idx, item) in items.iter().enumerate() {
                        let (expr_part, alias_opt) =
                            if let Some(as_pos) = find_top_level_keyword_ci(item, "AS") {
                                (
                                    item[..as_pos].trim(),
                                    Some(clean_sql_ident(&item[as_pos + 2..])),
                                )
                            } else {
                                (item.trim(), None)
                            };

                        if find_top_level_keyword_ci(expr_part, "OVER").is_some()
                            || find_top_level_keyword_ci(expr_part, "CASE").is_some()
                        {
                            if let Some(ssql_special) = transpile_window_or_case_select_item(
                                expr_part,
                                alias_opt.as_deref(),
                                &from_tbl_sql,
                                schema,
                            ) {
                                let marker_alias = format!("zzspecial{}", idx);
                                let (marker_nom, _, _) = suggest_sanskrit_column(&marker_alias);
                                rewritten_items.push(format!("{} AS {}", pcol, marker_alias));
                                special_replacements.push((marker_alias, marker_nom, ssql_special));
                            } else {
                                all_ok = false;
                                break;
                            }
                        } else {
                            rewritten_items.push(item.clone());
                        }
                    }

                    if all_ok && !special_replacements.is_empty() {
                        let rest_from = &after_sel[from_rel..];
                        let rewritten_sql =
                            format!("SELECT {} {}", rewritten_items.join(", "), rest_from);
                        if let Ok(mut ssql) =
                            crate::sql_importer::import_sql_to_ssql(&rewritten_sql, schema)
                        {
                            for (marker_alias, marker_nom, ssql_special) in special_replacements {
                                let candidates = [
                                    format!("\"{}\" इति आख्या", marker_alias),
                                    format!("{} इति आख्या", marker_nom),
                                    format!("{} इति आख्या", marker_alias),
                                ];
                                for pat in &candidates {
                                    if let Some(m_pos) = ssql.find(pat.as_str()) {
                                        let before_m = ssql[..m_pos].trim_end();
                                        let start_tok = before_m
                                            .rfind(char::is_whitespace)
                                            .map(|i| i + 1)
                                            .unwrap_or(0);
                                        let end_pos = m_pos + pat.len();
                                        ssql = format!(
                                            "{}{}{}",
                                            &ssql[..start_tok],
                                            ssql_special,
                                            &ssql[end_pos..]
                                        );
                                        break;
                                    }
                                }
                            }
                            return Ok(postprocess_classical_ssql(&ssql, offset_deva.as_deref()));
                        }
                    }
                }
            }
        }
    }

    let ssql = crate::sql_importer::import_sql_to_ssql(&expanded_sql, schema)
        .map_err(|e| format!("{}", e))?;
    Ok(postprocess_classical_ssql(&ssql, offset_deva.as_deref()))
}

