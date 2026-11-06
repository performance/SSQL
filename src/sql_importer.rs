//! Automated SQL -> SSQL Reverse Transpiler (`--from-sql`)
//!
//! Translates standard SQL DDL (`CREATE TABLE`), DQL (`SELECT ... FROM ... JOIN ...
//! WHERE ... GROUP BY ... HAVING ... ORDER BY ... LIMIT ...`), and DML (`INSERT`,
//! `UPDATE`, `DELETE`) into idiomatic, strict-mode-compliant SSQL (`.सं`) using
//! a target `Schema` and `MorphologyEngine`.
//!
//! Every generated SSQL statement obeys:
//! - Pāṇinian Kāraka case assignments (Panchami Bahuvachana for `FROM`, Tritiya
//!   for `JOIN`/`GROUP BY`, Shashthi for table qualification and aggregate/unary
//!   arguments, Saptami for `INSERT`/`UPDATE` targets).
//! - Gender (`लिङ्ग`) and number (`वचन`) concordance on comparison pronouns
//!   (`इत्यनेन`/`इत्यनया`/`इत्यस्मात्`/`इत्यस्याः`) and adjectives (`समः`/`समा`/`समम्`,
//!   `अधिकः`/`अधिका`/`अधिकम्`, `न्यूनः`/`न्यूना`/`न्यूनम्`, `भिन्नः`/`भिन्ना`/`भिन्नम्`,
//!   `रिक्तः`/`रिक्ता`/`रिक्तम्`).
//! - Subject-verb number agreement (`आह्रियताम्` for 1 column, `आह्रियेताम्` for 2,
//!   `आह्रियन्ताम्` for 3+).
//! - Pure Devanagari numerals (`०-९`) and `यत् ... इति` grouping (zero ASCII
//!   parentheses in generated DQL/DML).

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::morphology::MorphologyEngine;
use crate::schema::{ColumnInfo, Schema, TableInfo};
use crate::types::{Linga, Vachana, Vibhakti};

/// Convert ASCII digits (`0-9`) in a string to Devanagari digits (`०-९`).
pub fn to_devanagari_numerals(input: &str) -> String {
    input
        .chars()
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
            other => other,
        })
        .collect()
}

/// Transpile a SQL file into SSQL using an optional `schema.सं` path.
pub fn import_sql_file(sql_path: &Path, schema_path: Option<&Path>) -> Result<String> {
    let sql_source = fs::read_to_string(sql_path)
        .with_context(|| format!("Failed to read SQL file {:?}", sql_path))?;

    let mut compiler = crate::compiler::Compiler::new(crate::compiler::CompilerConfig::default())?;
    if let Some(sp) = schema_path {
        let schema_src = fs::read_to_string(sp)
            .with_context(|| format!("Failed to read schema file {:?}", sp))?;
        compiler.load_schema_from_ssql_string(&schema_src)?;
    } else if let Some(parent) = sql_path.parent() {
        let candidate = parent.join("schema.सं");
        if candidate.exists() {
            let schema_src = fs::read_to_string(&candidate)?;
            compiler.load_schema_from_ssql_string(&schema_src)?;
        }
    }

    // The same conversion the playground and the Python bindings use.
    let mut schema = compiler.schema;
    let blocks = crate::sql_converter::convert_sql_to_ssql(&sql_source, &mut schema)
        .map_err(|e| anyhow::anyhow!(e))?;
    Ok(format!("{}\n", blocks.join("\n\n")))
}

/// Transpile raw SQL statements into idiomatic, strict-mode SSQL.
pub fn import_sql_to_ssql(sql_source: &str, base_schema: &Schema) -> Result<String> {
    let mut active_schema = Schema {
        dialect: if base_schema.dialect.is_empty() {
            "sqlite".to_string()
        } else {
            base_schema.dialect.clone()
        },
        tables: base_schema.tables.clone(),
        literals: base_schema.literals.clone(),
        aliases: base_schema.aliases.clone(),
        functions: base_schema.functions.clone(),
        udf_params: base_schema.udf_params.clone(),
        derived_tables: base_schema.derived_tables.clone(),
        required_columns: base_schema.required_columns.clone(),
    };
    let mut morphology = MorphologyEngine::new()?;

    let clean_sql = strip_sql_comments(sql_source);
    let statements = split_sql_statements(&clean_sql);

    let mut out_statements = Vec::new();
    for raw_stmt in statements {
        let trimmed = raw_stmt.trim();
        if trimmed.is_empty() {
            continue;
        }
        let upper = trimmed.to_uppercase();
        if upper.starts_with("CREATE TABLE") {
            let ddl_ssql = transpile_create_table(trimmed, &mut active_schema, &mut morphology)?;
            out_statements.push(ddl_ssql);
        } else if upper.starts_with("SELECT") {
            let dql_ssql = transpile_select(trimmed, &active_schema, &mut morphology)?;
            out_statements.push(dql_ssql);
        } else if upper.starts_with("INSERT INTO") {
            let dml_ssql = transpile_insert(trimmed, &active_schema, &mut morphology)?;
            out_statements.push(dml_ssql);
        } else if upper.starts_with("UPDATE ") {
            let dml_ssql = transpile_update(trimmed, &active_schema, &mut morphology)?;
            out_statements.push(dml_ssql);
        } else if upper.starts_with("DELETE FROM") {
            let dml_ssql = transpile_delete(trimmed, &active_schema, &mut morphology)?;
            out_statements.push(dml_ssql);
        } else {
            bail!("Unsupported SQL statement for SSQL reverse transpilation: {}", trimmed);
        }
    }

    Ok(out_statements.join("\n\n"))
}

fn strip_sql_comments(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;

    while i < chars.len() {
        let c = chars[i];
        if c == '\'' && !in_double {
            in_single = !in_single;
            out.push(c);
            i += 1;
        } else if c == '"' && !in_single {
            in_double = !in_double;
            out.push(c);
            i += 1;
        } else if !in_single && !in_double && c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if !in_single && !in_double && c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut stmts = Vec::new();
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
                if !current.trim().is_empty() {
                    stmts.push(current.trim().to_string());
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        stmts.push(current.trim().to_string());
    }
    stmts
}

fn split_top_level_commas(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut depth = 0i32;

    for c in s.chars() {
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
            ',' if !in_single && !in_double && depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

/// Built-in domain vocabulary for synthesizing idiomatic Sanskrit table/column stems
/// when importing raw `CREATE TABLE` DDL that doesn't already have a `schema.सं`.
fn default_sanskrit_table_vocab(sql_table: &str) -> (String, String, Linga) {
    match sql_table.to_lowercase().as_str() {
        "books" | "book" => ("पुस्तकानि".to_string(), "पुस्तक".to_string(), Linga::Neuter),
        "authors" | "author" => ("लेखकाः".to_string(), "लेखक".to_string(), Linga::Masculine),
        "users" | "user" => ("उपयोक्तारः".to_string(), "उपयोक्तृ".to_string(), Linga::Masculine),
        "students" | "student" => ("छात्राः".to_string(), "छात्र".to_string(), Linga::Masculine),
        "courses" | "course" => ("पाठ्यक्रमाः".to_string(), "पाठ्यक्रम".to_string(), Linga::Masculine),
        "orders" | "order" => ("आदेशाः".to_string(), "आदेश".to_string(), Linga::Masculine),
        "order_details" => ("आदेशविवरणानि".to_string(), "आदेशविवरण".to_string(), Linga::Neuter),
        "products" | "product" => ("उत्पादाः".to_string(), "उत्पाद".to_string(), Linga::Masculine),
        "customers" | "customer" => ("ग्राहकाः".to_string(), "ग्राहक".to_string(), Linga::Masculine),
        "employees" | "employee" => ("कर्मचारिणः".to_string(), "कर्मचारिन्".to_string(), Linga::Masculine),
        "departments" | "department" => ("विभागाः".to_string(), "विभाग".to_string(), Linga::Masculine),
        "sales" | "sale" => ("विक्रयाः".to_string(), "विक्रय".to_string(), Linga::Masculine),
        "payments" | "payment" => ("भुगतानानि".to_string(), "भुगतान".to_string(), Linga::Neuter),
        "reviews" | "review" => ("समीक्षाः".to_string(), "समीक्षा".to_string(), Linga::Feminine),
        other => {
            let stem = transliterate_ascii_to_devanagari(other.trim_end_matches('s'));
            (format!("{}ानि", stem), stem, Linga::Neuter)
        }
    }
}

fn default_sanskrit_column_vocab(sql_col: &str) -> (String, String, Linga) {
    match sql_col.to_lowercase().as_str() {
        "id" => ("क्रमाङ्कः".to_string(), "क्रमाङ्क".to_string(), Linga::Masculine),
        "book_id" => ("पुस्तकसङ्ख्या".to_string(), "पुस्तकसङ्ख्या".to_string(), Linga::Feminine),
        "author_id" => ("लेखकसङ्ख्या".to_string(), "लेखकसङ्ख्या".to_string(), Linga::Feminine),
        "user_id" => ("उपयोक्तृसङ्ख्या".to_string(), "उपयोक्तृसङ्ख्या".to_string(), Linga::Feminine),
        "student_id" => ("छात्रसङ्ख्या".to_string(), "छात्रसङ्ख्या".to_string(), Linga::Feminine),
        "course_id" => ("पाठ्यक्रमसङ्ख्या".to_string(), "पाठ्यक्रमसङ्ख्या".to_string(), Linga::Feminine),
        "order_id" => ("आदेशसङ्ख्या".to_string(), "आदेशसङ्ख्या".to_string(), Linga::Feminine),
        "department_id" => ("विभागसङ्ख्या".to_string(), "विभागसङ्ख्या".to_string(), Linga::Feminine),
        "title" => ("शीर्षकम्".to_string(), "शीर्षक".to_string(), Linga::Neuter),
        "name" => ("नाम".to_string(), "नामन्".to_string(), Linga::Neuter),
        "price" => ("मूल्यम्".to_string(), "मूल्य".to_string(), Linga::Neuter),
        "amount" => ("राशिः".to_string(), "राशि".to_string(), Linga::Masculine),
        "quantity" => ("मात्रा".to_string(), "मात्रा".to_string(), Linga::Feminine),
        "publication_year" | "year" => ("प्रकाशनवर्षम्".to_string(), "प्रकाशनवर्ष".to_string(), Linga::Neuter),
        "date" | "order_date" | "created_at" => ("दिनाङ्कः".to_string(), "दिनाङ्क".to_string(), Linga::Masculine),
        "time" => ("समयः".to_string(), "समय".to_string(), Linga::Masculine),
        "status" => ("स्थितिः".to_string(), "स्थिति".to_string(), Linga::Feminine),
        "description" => ("विवरणम्".to_string(), "विवरण".to_string(), Linga::Neuter),
        "category" => ("वर्गः".to_string(), "वर्ग".to_string(), Linga::Masculine),
        "email" => ("विद्युत्पत्रम्".to_string(), "विद्युत्पत्र".to_string(), Linga::Neuter),
        "salary" => ("वेतनम्".to_string(), "वेतन".to_string(), Linga::Neuter),
        "score" | "rating" => ("अङ्कः".to_string(), "अङ्क".to_string(), Linga::Masculine),
        "age" => ("वयः".to_string(), "वयस्".to_string(), Linga::Neuter),
        "city" => ("नगरम्".to_string(), "नगर".to_string(), Linga::Neuter),
        "country" => ("देशः".to_string(), "देश".to_string(), Linga::Masculine),
        other => {
            let stem = transliterate_ascii_to_devanagari(other);
            (format!("{}म्", stem), stem, Linga::Neuter)
        }
    }
}

fn transliterate_ascii_to_devanagari(ascii: &str) -> String {
    let mut out = String::new();
    for c in ascii.to_lowercase().chars() {
        let piece = match c {
            'a' => "अ",
            'b' => "ब",
            'c' => "क",
            'd' => "द",
            'e' => "ए",
            'f' => "फ",
            'g' => "ग",
            'h' => "ह",
            'i' => "इ",
            'j' => "ज",
            'k' => "क",
            'l' => "ल",
            'm' => "म",
            'n' => "न",
            'o' => "ओ",
            'p' => "प",
            'q' => "क",
            'r' => "र",
            's' => "स",
            't' => "त",
            'u' => "उ",
            'v' | 'w' => "व",
            'x' => "क्स",
            'y' => "य",
            'z' => "ज",
            '_' => "_",
            '0'..='9' => "",
            _ => "",
        };
        out.push_str(piece);
    }
    if out.is_empty() {
        "स्तम्भ".to_string()
    } else {
        out
    }
}

fn gender_keyword(linga: Linga) -> &'static str {
    match linga {
        Linga::Masculine => "पुंलिङ्गम्",
        Linga::Feminine => "स्त्रीलिङ्गम्",
        Linga::Neuter => "नपुंसकलिङ्गम्",
    }
}

fn decline_word(
    morphology: &mut MorphologyEngine,
    stem: &str,
    gender: Linga,
    vachana: Vachana,
    vibhakti: Vibhakti,
) -> String {
    if let Ok(table) = morphology.decline_with_gender(stem, gender, vachana) {
        if let Some(form) = table.forms.get(&vibhakti) {
            return form.clone();
        }
    }
    stem.to_string()
}

fn transpile_create_table(
    stmt: &str,
    active_schema: &mut Schema,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let open_paren = stmt
        .find('(')
        .ok_or_else(|| anyhow::anyhow!("Malformed CREATE TABLE statement: {}", stmt))?;
    let close_paren = stmt
        .rfind(')')
        .ok_or_else(|| anyhow::anyhow!("Malformed CREATE TABLE statement: {}", stmt))?;

    let header = stmt[..open_paren].trim();
    let sql_table_name = header
        .split_whitespace()
        .last()
        .ok_or_else(|| anyhow::anyhow!("Missing table name in CREATE TABLE"))?
        .trim_matches(|c| c == '"' || c == '`' || c == '\'');

    let (table_nom_plural, table_stem, table_gender) =
        if let Some(existing) = find_table_by_sql_name(active_schema, sql_table_name) {
            let nom_pl = decline_word(
                morphology,
                &existing.pratipadika,
                existing.gender,
                Vachana::Bahu,
                Vibhakti::Prathama,
            );
            (nom_pl, existing.pratipadika.clone(), existing.gender)
        } else {
            default_sanskrit_table_vocab(sql_table_name)
        };

    let body = &stmt[open_paren + 1..close_paren];
    let col_entries = split_top_level_commas(body);
    let mut ssql_cols = Vec::new();

    for entry in col_entries {
        let tokens: Vec<&str> = entry.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let first_upper = tokens[0].to_uppercase();
        if matches!(
            first_upper.as_str(),
            "PRIMARY" | "FOREIGN" | "UNIQUE" | "CHECK" | "CONSTRAINT"
        ) {
            continue;
        }
        let col_sql_name = tokens[0].trim_matches(|c| c == '"' || c == '`' || c == '\'');
        let col_type_upper = tokens.get(1).map(|s| s.to_uppercase()).unwrap_or_default();
        let ssql_type = if col_type_upper.contains("INT") {
            "पूर्णाङ्कः"
        } else if col_type_upper.contains("REAL")
            || col_type_upper.contains("DEC")
            || col_type_upper.contains("NUM")
            || col_type_upper.contains("FLOAT")
            || col_type_upper.contains("DOUBLE")
        {
            "वास्तविकः"
        } else if col_type_upper.contains("DATE") || col_type_upper.contains("TIME") {
            "दिनाङ्कः"
        } else if col_type_upper.contains("BOOL") {
            "सत्यासत्यः"
        } else {
            "वाक्यम्"
        };

        let (col_nom, col_stem, col_gender) = if let Some(existing_tbl) =
            find_table_by_sql_name(active_schema, sql_table_name)
        {
            if let Some(existing_col) = existing_tbl
                .fields
                .values()
                .find(|c| c.sql_name.eq_ignore_ascii_case(col_sql_name))
            {
                let nom = decline_word(
                    morphology,
                    &existing_col.pratipadika,
                    existing_col.gender,
                    existing_col.vachana.unwrap_or(Vachana::Eka),
                    Vibhakti::Prathama,
                );
                (nom, existing_col.pratipadika.clone(), existing_col.gender)
            } else {
                default_sanskrit_column_vocab(col_sql_name)
            }
        } else {
            default_sanskrit_column_vocab(col_sql_name)
        };

        let entry_upper = entry.to_uppercase();
        let mut prefixes = Vec::new();
        if entry_upper.contains("PRIMARY KEY") {
            prefixes.push(match col_gender {
                Linga::Masculine => "प्राथमिकः",
                Linga::Feminine => "प्राथमिकी",
                Linga::Neuter => "प्राथमिकम्",
            });
        }
        if entry_upper.contains("AUTOINCREMENT") || entry_upper.contains("SERIAL") {
            prefixes.push(match col_gender {
                Linga::Masculine => "क्रमिकः",
                Linga::Feminine => "क्रमिका",
                Linga::Neuter => "क्रमिकम्",
            });
        }
        if entry_upper.contains("NOT NULL") {
            prefixes.push(match col_gender {
                Linga::Masculine => "अनिवार्यः",
                Linga::Feminine => "अनिवार्या",
                Linga::Neuter => "अनिवार्यम्",
            });
        }
        if entry_upper.contains("UNIQUE") && !entry_upper.contains("PRIMARY KEY") {
            prefixes.push(match col_gender {
                Linga::Masculine => "अद्वितीयः",
                Linga::Feminine => "अद्वितीया",
                Linga::Neuter => "अद्वितीयम्",
            });
        }

        let prefix_str = if prefixes.is_empty() {
            String::new()
        } else {
            format!("{} ", prefixes.join(" "))
        };

        ssql_cols.push(format!(
            "    {}{} इति {} \"{}\" इति आख्या {} इति {} एकवचनम्",
            prefix_str,
            col_nom,
            ssql_type,
            col_sql_name,
            col_stem,
            gender_keyword(col_gender)
        ));
    }

    let ddl_ssql = format!(
        "{} इति पटलम् \"{}\" इति आख्या {} इति {} बहुवचनम् रच्यताम् यत्र\n{}\n  इति पटलम् ।",
        table_nom_plural,
        sql_table_name,
        table_stem,
        gender_keyword(table_gender),
        // One column per line; with several, each takes its own च (L1).
        if ssql_cols.len() > 1 {
            ssql_cols.iter().map(|c| format!("{} च", c)).collect::<Vec<_>>().join("\n")
        } else {
            ssql_cols.join("")
        }
    );

    // Register the newly transpiled DDL into active_schema so subsequent queries in the same batch resolve
    let mut temp_compiler = crate::compiler::Compiler::new(crate::compiler::CompilerConfig {
        dialect: active_schema.dialect.clone(),
        ..Default::default()
    })?;
    temp_compiler.schema = Schema {
        dialect: active_schema.dialect.clone(),
        tables: active_schema.tables.clone(),
        literals: active_schema.literals.clone(),
        aliases: active_schema.aliases.clone(),
        functions: active_schema.functions.clone(),
        udf_params: active_schema.udf_params.clone(),
        derived_tables: active_schema.derived_tables.clone(),
        required_columns: active_schema.required_columns.clone(),
    };
    if temp_compiler.load_schema_from_ssql_string(&ddl_ssql).is_ok() {
        active_schema.tables = temp_compiler.schema.tables;
    }

    Ok(ddl_ssql)
}

fn find_table_by_sql_name<'a>(schema: &'a Schema, sql_name: &str) -> Option<&'a TableInfo> {
    let clean = sql_name.trim_matches(|c| c == '"' || c == '`' || c == '\'');
    schema
        .tables
        .values()
        .find(|t| t.sql_name.eq_ignore_ascii_case(clean) || t.pratipadika == clean)
}

fn find_column_in_table<'a>(table: &'a TableInfo, sql_col: &str) -> Option<&'a ColumnInfo> {
    let clean = sql_col.trim_matches(|c| c == '"' || c == '`' || c == '\'');
    table
        .fields
        .values()
        .find(|c| c.sql_name.eq_ignore_ascii_case(clean) || c.pratipadika == clean)
}

#[derive(Default)]
struct QueryScope<'a> {
    primary_table: Option<&'a TableInfo>,
    joined_tables: Vec<&'a TableInfo>,
    alias_to_table: HashMap<String, &'a TableInfo>,
}

impl<'a> QueryScope<'a> {
    fn has_joins(&self) -> bool {
        !self.joined_tables.is_empty()
    }

    fn resolve_col(&self, raw_ref: &str) -> Option<(&'a TableInfo, &'a ColumnInfo, bool)> {
        let trimmed = raw_ref.trim();
        if let Some((prefix, col_part)) = trimmed.split_once('.') {
            let prefix_clean = prefix.trim().to_lowercase();
            let tbl = self
                .alias_to_table
                .get(&prefix_clean)
                .copied()
                .or_else(|| {
                    self.primary_table
                        .into_iter()
                        .chain(self.joined_tables.iter().copied())
                        .find(|t| t.sql_name.eq_ignore_ascii_case(prefix.trim()))
                })?;
            let col = find_column_in_table(tbl, col_part.trim())?;
            return Some((tbl, col, self.has_joins()));
        }

        for tbl in self
            .primary_table
            .into_iter()
            .chain(self.joined_tables.iter().copied())
        {
            if let Some(col) = find_column_in_table(tbl, trimmed) {
                return Some((tbl, col, self.has_joins()));
            }
        }
        None
    }
}

fn format_col_with_vibhakti(
    tbl: &TableInfo,
    col: &ColumnInfo,
    qualify_table: bool,
    vibhakti: Vibhakti,
    morphology: &mut MorphologyEngine,
) -> String {
    let col_vachana = col.vachana.unwrap_or(Vachana::Eka);
    let col_form = decline_word(morphology, &col.pratipadika, col.gender, col_vachana, vibhakti);
    if qualify_table {
        let tbl_genitive = decline_word(
            morphology,
            &tbl.pratipadika,
            tbl.gender,
            Vachana::Eka,
            Vibhakti::Shashthi,
        );
        format!("{} {}", tbl_genitive, col_form)
    } else {
        col_form
    }
}

fn find_top_level_keyword(sql: &str, keyword: &str) -> Option<usize> {
    let upper = sql.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let bytes = sql.as_bytes();
    let mut in_single = false;
    let mut in_double = false;
    let mut depth = 0i32;

    let mut idx = 0;
    while idx + kw_upper.len() <= sql.len() {
        let b = bytes[idx];
        match b {
            b'\'' if !in_double => in_single = !in_single,
            b'"' if !in_single => in_double = !in_double,
            b'(' if !in_single && !in_double => depth += 1,
            b')' if !in_single && !in_double => depth = (depth - 1).max(0),
            _ => {}
        }
        if !in_single && !in_double && depth == 0 && upper[idx..].starts_with(&kw_upper) {
            let before_ok = idx == 0 || bytes[idx - 1].is_ascii_whitespace() || bytes[idx - 1] == b')';
            let after_idx = idx + kw_upper.len();
            let after_ok = after_idx == sql.len()
                || bytes[after_idx].is_ascii_whitespace()
                || bytes[after_idx] == b'(';
            if before_ok && after_ok {
                return Some(idx);
            }
        }
        idx += 1;
    }
    None
}

fn transpile_select(
    stmt: &str,
    schema: &Schema,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let from_pos = find_top_level_keyword(stmt, "FROM")
        .ok_or_else(|| anyhow::anyhow!("SELECT statement missing FROM clause: {}", stmt))?;

    let select_part_raw = stmt["SELECT".len()..from_pos].trim();
    let (is_distinct, select_part) = if select_part_raw.to_uppercase().starts_with("DISTINCT ") {
        (true, select_part_raw["DISTINCT".len()..].trim())
    } else {
        (false, select_part_raw)
    };

    let after_from = stmt[from_pos + "FROM".len()..].trim();

    let where_pos = find_top_level_keyword(after_from, "WHERE");
    let group_pos = find_top_level_keyword(after_from, "GROUP BY");
    let having_pos = find_top_level_keyword(after_from, "HAVING");
    let order_pos = find_top_level_keyword(after_from, "ORDER BY");
    let limit_pos = find_top_level_keyword(after_from, "LIMIT");

    let mut clause_starts: Vec<(&str, usize)> = Vec::new();
    if let Some(p) = where_pos {
        clause_starts.push(("WHERE", p));
    }
    if let Some(p) = group_pos {
        clause_starts.push(("GROUP BY", p));
    }
    if let Some(p) = having_pos {
        clause_starts.push(("HAVING", p));
    }
    if let Some(p) = order_pos {
        clause_starts.push(("ORDER BY", p));
    }
    if let Some(p) = limit_pos {
        clause_starts.push(("LIMIT", p));
    }
    clause_starts.sort_by_key(|(_, p)| *p);

    let from_and_joins_end = clause_starts
        .first()
        .map(|(_, p)| *p)
        .unwrap_or(after_from.len());
    let from_and_joins = after_from[..from_and_joins_end].trim();

    let mut clause_map: HashMap<&str, &str> = HashMap::new();
    for i in 0..clause_starts.len() {
        let (name, start) = clause_starts[i];
        let end = clause_starts
            .get(i + 1)
            .map(|(_, p)| *p)
            .unwrap_or(after_from.len());
        let body = after_from[start + name.len()..end].trim();
        clause_map.insert(name, body);
    }

    // Parse FROM and JOINs
    let (scope, from_ssql, joins_ssql) =
        parse_from_and_joins(from_and_joins, schema, morphology)?;

    let mut ssql_parts = vec![from_ssql];
    ssql_parts.extend(joins_ssql);

    if let Some(where_body) = clause_map.get("WHERE") {
        let cond_ssql = transpile_condition(where_body, &scope, morphology)?;
        ssql_parts.push(format!("{} इति नियमेन", cond_ssql));
    }

    if let Some(group_body) = clause_map.get("GROUP BY") {
        let group_cols = split_top_level_commas(group_body);
        let mut group_ssql = Vec::new();
        for gc in group_cols {
            if let Some((tbl, col, qualify)) = scope.resolve_col(&gc) {
                group_ssql.push(format_col_with_vibhakti(
                    tbl,
                    col,
                    qualify,
                    Vibhakti::Prathama,
                    morphology,
                ));
            } else {
                bail!("Cannot resolve GROUP BY column '{}' in schema", gc);
            }
        }
        ssql_parts.push(format!("{} इति समूहेन", cha_list(&group_ssql)));
    }

    if let Some(having_body) = clause_map.get("HAVING") {
        let having_ssql = transpile_condition(having_body, &scope, morphology)?;
        ssql_parts.push(format!("{} इति समूहनियमेन", having_ssql));
    }

    // SELECT projection items
    let raw_select_items = split_top_level_commas(select_part);
    let mut select_ssql_items = Vec::new();
    for item in &raw_select_items {
        select_ssql_items.push(transpile_select_item(item, &scope, morphology)?);
    }

    // ORDER BY and LIMIT stand before the select list with the other clause
    // markers; every ordering key states its direction (`मूल्यम् अवरोहः च
    // शीर्षकम् आरोहः च इति क्रमेण`).
    if let Some(order_body) = clause_map.get("ORDER BY") {
        let mut order_ssql_items = Vec::new();
        for item in split_top_level_commas(order_body) {
            let tokens: Vec<&str> = item.split_whitespace().collect();
            let (expr_str, direction) = match tokens.last() {
                Some(d) if d.eq_ignore_ascii_case("DESC") => (tokens[..tokens.len() - 1].join(" "), "अवरोहः"),
                Some(d) if d.eq_ignore_ascii_case("ASC") => (tokens[..tokens.len() - 1].join(" "), "आरोहः"),
                _ => (item.clone(), "आरोहः"),
            };
            let order_expr_ssql = transpile_value_expr(&expr_str, &scope, morphology)?;
            order_ssql_items.push(format!("{} {}", order_expr_ssql, direction));
        }
        ssql_parts.push(format!("{} इति क्रमेण", cha_list(&order_ssql_items)));
    }

    if let Some(limit_body) = clause_map.get("LIMIT") {
        let limit_num = limit_body.split_whitespace().next().unwrap_or(limit_body);
        ssql_parts.push(format!("{} इति मित्या", to_devanagari_numerals(limit_num)));
    }

    if is_distinct {
        ssql_parts.push("पुनरुक्तिम् विना".to_string());
    }

    ssql_parts.push(cha_list(&select_ssql_items));

    let verb = match select_ssql_items.len() {
        1 => "आह्रियताम्",
        2 => "आह्रियेताम्",
        _ => "आह्रियन्ताम्",
    };
    ssql_parts.push(format!("{} ।", verb));

    Ok(ssql_parts.join(" "))
}

/// A च-list: one item stands alone; several each carry their own च.
fn cha_list(items: &[String]) -> String {
    if items.len() == 1 {
        items[0].clone()
    } else {
        items.iter().map(|i| format!("{} च", i)).collect::<Vec<_>>().join(" ")
    }
}

fn parse_from_and_joins<'a>(
    from_and_joins: &str,
    schema: &'a Schema,
    morphology: &mut MorphologyEngine,
) -> Result<(QueryScope<'a>, String, Vec<String>)> {
    let mut scope = QueryScope::default();
    let mut joins_ssql = Vec::new();

    // Locate any JOIN keywords
    let upper = from_and_joins.to_uppercase();
    let mut join_positions: Vec<(usize, &'static str, usize)> = Vec::new();

    for (kw, ssql_join_kw) in [
        ("LEFT OUTER JOIN", "वाममेलनेन"),
        ("LEFT JOIN", "वाममेलनेन"),
        ("RIGHT OUTER JOIN", "दक्षिणमेलनेन"),
        ("RIGHT JOIN", "दक्षिणमेलनेन"),
        ("FULL OUTER JOIN", "पूर्णमेलनेन"),
        ("FULL JOIN", "पूर्णमेलनेन"),
        ("CROSS JOIN", "सर्वमेलनेन"),
        ("INNER JOIN", "मेलनेन"),
        ("JOIN", "मेलनेन"),
    ] {
        let mut search_from = 0;
        while let Some(rel) = upper[search_from..].find(kw) {
            let abs = search_from + rel;
            let before_ok = abs == 0 || from_and_joins.as_bytes()[abs - 1].is_ascii_whitespace();
            let after_pos = abs + kw.len();
            let after_ok = after_pos == from_and_joins.len()
                || from_and_joins.as_bytes()[after_pos].is_ascii_whitespace();
            let overlaps = join_positions
                .iter()
                .any(|(s, _, l)| abs >= *s && abs < *s + *l);
            if before_ok && after_ok && !overlaps {
                join_positions.push((abs, ssql_join_kw, kw.len()));
            }
            search_from = after_pos;
        }
    }
    join_positions.sort_by_key(|(pos, _, _)| *pos);

    let from_segment = if let Some((first_join_pos, _, _)) = join_positions.first() {
        from_and_joins[..*first_join_pos].trim()
    } else {
        from_and_joins.trim()
    };

    let (primary_tbl, primary_alias) = parse_table_and_alias(from_segment, schema)?;
    scope.primary_table = Some(primary_tbl);
    scope
        .alias_to_table
        .insert(primary_tbl.sql_name.to_lowercase(), primary_tbl);
    if let Some(a) = primary_alias {
        scope.alias_to_table.insert(a.to_lowercase(), primary_tbl);
    }

    let from_ssql = decline_word(
        morphology,
        &primary_tbl.pratipadika,
        primary_tbl.gender,
        Vachana::Bahu,
        Vibhakti::Panchami,
    );

    // First register all joined tables in scope so ON conditions can resolve both sides
    let mut parsed_joins = Vec::new();
    for i in 0..join_positions.len() {
        let (j_pos, j_kw, j_len) = join_positions[i];
        let seg_end = join_positions
            .get(i + 1)
            .map(|(p, _, _)| *p)
            .unwrap_or(from_and_joins.len());
        let seg = from_and_joins[j_pos + j_len..seg_end].trim();
        let (tbl_part, on_part) = if let Some(on_idx) = find_top_level_keyword(seg, "ON") {
            (seg[..on_idx].trim(), Some(seg[on_idx + 2..].trim()))
        } else {
            (seg, None)
        };
        let (j_tbl, j_alias) = parse_table_and_alias(tbl_part, schema)?;
        scope.joined_tables.push(j_tbl);
        scope
            .alias_to_table
            .insert(j_tbl.sql_name.to_lowercase(), j_tbl);
        if let Some(a) = j_alias {
            scope.alias_to_table.insert(a.to_lowercase(), j_tbl);
        }
        parsed_joins.push((j_tbl, j_kw, on_part));
    }

    for (j_tbl, j_kw, on_part) in parsed_joins {
        let j_tbl_tritiya = decline_word(
            morphology,
            &j_tbl.pratipadika,
            j_tbl.gender,
            Vachana::Bahu,
            Vibhakti::Tritiya,
        );
        if let Some(on_cond) = on_part {
            let on_ssql = transpile_join_on_condition(on_cond, &scope, morphology)?;
            joins_ssql.push(format!("{} सह {} इति {}", j_tbl_tritiya, on_ssql, j_kw));
        } else {
            joins_ssql.push(format!("{} सह इति {}", j_tbl_tritiya, j_kw));
        }
    }

    Ok((scope, from_ssql, joins_ssql))
}

fn parse_table_and_alias<'a>(
    segment: &str,
    schema: &'a Schema,
) -> Result<(&'a TableInfo, Option<String>)> {
    let tokens: Vec<&str> = segment.split_whitespace().collect();
    if tokens.is_empty() {
        bail!("Empty table reference in SQL");
    }
    let tbl_name = tokens[0];
    let tbl = find_table_by_sql_name(schema, tbl_name)
        .ok_or_else(|| anyhow::anyhow!("Table '{}' not found in schema.सं", tbl_name))?;
    let alias = if tokens.len() >= 3 && tokens[1].eq_ignore_ascii_case("AS") {
        Some(tokens[2].to_string())
    } else if tokens.len() >= 2 {
        Some(tokens[1].to_string())
    } else {
        None
    };
    Ok((tbl, alias))
}

fn transpile_join_on_condition(
    on_cond: &str,
    scope: &QueryScope,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    if let Some((lhs, rhs)) = on_cond.split_once('=') {
        let (l_tbl, l_col, _) = scope
            .resolve_col(lhs.trim())
            .ok_or_else(|| anyhow::anyhow!("Cannot resolve JOIN column '{}'", lhs.trim()))?;
        let (r_tbl, r_col, _) = scope
            .resolve_col(rhs.trim())
            .ok_or_else(|| anyhow::anyhow!("Cannot resolve JOIN column '{}'", rhs.trim()))?;

        let l_ssql = format_col_with_vibhakti(l_tbl, l_col, true, Vibhakti::Prathama, morphology);
        let r_ssql = format_col_with_vibhakti(r_tbl, r_col, true, Vibhakti::Prathama, morphology);
        let vachana = l_col.vachana.unwrap_or(Vachana::Eka);
        let pronoun = concordant_pronoun("=", l_col.gender, vachana);
        let adj = concordant_adjective("=", l_col.gender, vachana);
        return Ok(format!("{} {} {} {} अस्ति", l_ssql, r_ssql, pronoun, adj));
    }
    transpile_condition(on_cond, scope, morphology)
}

fn transpile_select_item(
    item: &str,
    scope: &QueryScope,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let trimmed = item.trim();
    if trimmed == "*" {
        return Ok("सर्वम्".to_string());
    }

    // Check for AS alias
    let (expr_part, alias_opt) = if let Some(as_idx) = find_top_level_keyword(trimmed, "AS") {
        let expr_s = trimmed[..as_idx].trim();
        let alias_s = trimmed[as_idx + 2..]
            .trim()
            .trim_matches(|c| c == '"' || c == '\'' || c == '`');
        (expr_s, Some(alias_s.to_string()))
    } else {
        (trimmed, None)
    };

    let expr_ssql = transpile_value_expr(expr_part, scope, morphology)?;
    if let Some(alias) = alias_opt {
        Ok(format!("{} \"{}\" इति आख्या", expr_ssql, alias))
    } else {
        Ok(expr_ssql)
    }
}

fn transpile_value_expr(
    expr: &str,
    scope: &QueryScope,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let trimmed = expr.trim();
    // One pair of parentheses around the whole value (not `(a) + (b)`).
    if strip_outer_parens(trimmed) != trimmed {
        return transpile_value_expr(strip_outer_parens(trimmed), scope, morphology);
    }

    // String literal
    if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
    {
        let inner = &trimmed[1..trimmed.len() - 1];
        let is_date = !inner.is_empty()
            && (inner.contains('-') || inner.contains(':'))
            && inner.chars().any(|c| c.is_ascii_digit() || ('०'..='९').contains(&c))
            && inner.chars().all(|c| {
                c.is_ascii_digit()
                    || ('०'..='९').contains(&c)
                    || matches!(c, '-' | ':' | ' ' | 'T' | 'Z' | '.' | '+')
            });
        let conv = if is_date {
            to_devanagari_numerals(inner)
        } else if trimmed.starts_with('\'') {
            // SQL doubles a quote inside a string (`'978''x'`).
            inner.replace("''", "'")
        } else {
            inner.to_string()
        };
        return Ok(format!("\"{}\"", conv));
    }

    // Numeric literal
    if trimmed
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || ('०'..='९').contains(&c))
        && !trimmed.is_empty()
    {
        return Ok(to_devanagari_numerals(trimmed));
    }

    // Function / Aggregate call: FUNC(...)
    if let Some(open_p) = trimmed.find('(') {
        if trimmed.ends_with(')') {
            let fn_name = trimmed[..open_p].trim().to_uppercase();
            let inner_arg = trimmed[open_p + 1..trimmed.len() - 1].trim();
            match fn_name.as_str() {
                "COUNT" => {
                    if inner_arg == "*" {
                        return Ok("पङ्क्तीनाम् गणना".to_string());
                    }
                    if inner_arg.to_uppercase().starts_with("DISTINCT ") {
                        let col_ref = inner_arg["DISTINCT".len()..].trim();
                        if let Some((tbl, col, qualify)) = scope.resolve_col(col_ref) {
                            let distinct_word = match col.gender {
                                Linga::Feminine => "विशिष्टायाः",
                                _ => "विशिष्टस्य",
                            };
                            let shashthi = format_col_with_vibhakti(
                                tbl,
                                col,
                                qualify,
                                Vibhakti::Shashthi,
                                morphology,
                            );
                            return Ok(format!("{} {} गणना", distinct_word, shashthi));
                        }
                    }
                    if let Some((tbl, col, qualify)) = scope.resolve_col(inner_arg) {
                        let shashthi = format_col_with_vibhakti(
                            tbl,
                            col,
                            qualify,
                            Vibhakti::Shashthi,
                            morphology,
                        );
                        return Ok(format!("{} गणना", shashthi));
                    }
                    if split_arithmetic(strip_outer_parens(inner_arg)).is_some() {
                        return Ok(format!("{} गणना", arithmetic_in(inner_arg, Vibhakti::Shashthi, scope, morphology)?));
                    }
                }
                "SUM" | "AVG" | "MAX" | "MIN" | "UPPER" | "LOWER" | "LENGTH" => {
                    let ssql_fn = match fn_name.as_str() {
                        "SUM" => "योगः",
                        "AVG" => "माध्यम्",
                        "MAX" => "महत्तमम्",
                        "MIN" => "लघुतमम्",
                        "UPPER" => "महावर्णरूपम्",
                        "LOWER" => "लघुवर्णरूपम्",
                        "LENGTH" => "दैर्घ्यम्",
                        _ => unreachable!(),
                    };
                    if let Some((tbl, col, qualify)) = scope.resolve_col(inner_arg) {
                        let shashthi = format_col_with_vibhakti(
                            tbl,
                            col,
                            qualify,
                            Vibhakti::Shashthi,
                            morphology,
                        );
                        return Ok(format!("{} {}", shashthi, ssql_fn));
                    }
                    // The result noun is itself the genitive (`अवशिष्टस्य योगः`).
                    if split_arithmetic(strip_outer_parens(inner_arg)).is_some() {
                        return Ok(format!("{} {}", arithmetic_in(inner_arg, Vibhakti::Shashthi, scope, morphology)?, ssql_fn));
                    }
                }
                _ => {}
            }
        }
    }

    // Binary arithmetic, written in the nominative (see `arithmetic_in`).
    if split_arithmetic(trimmed).is_some() {
        return arithmetic_in(trimmed, Vibhakti::Prathama, scope, morphology);
    }

    // A sign: `-5`, `-price`.
    if let Some(value) = trimmed.strip_prefix('-') {
        return negative_of(value, Vibhakti::Prathama, scope, morphology);
    }

    // Column reference
    if let Some((tbl, col, qualify)) = scope.resolve_col(trimmed) {
        return Ok(format_col_with_vibhakti(
            tbl,
            col,
            qualify,
            Vibhakti::Prathama,
            morphology,
        ));
    }

    bail!("Cannot resolve SQL expression '{}' against schema.सं", trimmed)
}

/// Arithmetic as SSQL in `case`: the nominative for a value, or the case of
/// a gerund's slot (ablative minuend, accusative subtrahend or dividend,
/// instrumental divisor), carried by the result noun (expert review,
/// grouped operands). A sum or product is one list (`a च b च c च इत्येषाम्
/// योगः`); a chain of subtractions is one gerund with several objects
/// (`मूल्यात् प्रकाशनवर्षम् च १ च विशोध्य अवशिष्टम्`), of divisions one divisor
/// after another. A subtraction on the right of a subtraction is grouped.
fn arithmetic_in(
    expr: &str,
    case: Vibhakti,
    scope: &QueryScope,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let trimmed = strip_outer_parens(expr);
    let refuse = || anyhow::anyhow!("`{}`: SSQL cannot yet write this arithmetic in this position", expr.trim());
    let Some((lhs, op, rhs)) = split_arithmetic(trimmed) else {
        return Err(refuse());
    };
    // The operands of a chain of one operator, left to right.
    let chain = |first_op: char| {
        let mut operands = vec![rhs];
        let mut rest = lhs;
        loop {
            let inner = if first_op == '-' || first_op == '/' { strip_outer_parens(rest) } else { rest };
            match split_arithmetic(inner) {
                Some((l, o, r)) if o == first_op => {
                    operands.push(r);
                    rest = l;
                }
                _ => break,
            }
        }
        operands.push(rest);
        operands.reverse();
        operands
    };
    match op {
        '+' | '*' => {
            let mut items = Vec::new();
            for operand in chain(op) {
                let inner = strip_outer_parens(operand);
                let ssql = transpile_value_expr(inner, scope, morphology)?;
                items.push(if split_arithmetic(inner).is_some() { format!("यत् {} इति", ssql) } else { ssql });
            }
            let pronoun = if items.len() == 2 { "इत्यनयोः" } else { "इत्येषाम्" };
            let noun = match (op, case) {
                ('+', Vibhakti::Panchami) => "योगात्",
                ('+', Vibhakti::Dvitiya) => "योगम्",
                ('+', Vibhakti::Tritiya) => "योगेन",
                ('+', Vibhakti::Shashthi) => "योगस्य",
                ('+', _) => "योगः",
                (_, Vibhakti::Panchami) => "गुणनात्",
                (_, Vibhakti::Tritiya) => "गुणनेन",
                (_, Vibhakti::Shashthi) => "गुणनस्य",
                _ => "गुणनम्",
            };
            Ok(format!("{} च {} {}", items.join(" च "), pronoun, noun))
        }
        '-' => {
            let noun = match case {
                Vibhakti::Prathama | Vibhakti::Dvitiya => "अवशिष्टम्",
                Vibhakti::Tritiya => "अवशिष्टेन",
                Vibhakti::Shashthi => "अवशिष्टस्य",
                _ => return Err(refuse()),
            };
            let operands = chain('-');
            let minuend = operand_in(operands[0], Vibhakti::Panchami, scope, morphology)?;
            let mut subtrahends = Vec::new();
            for s in &operands[1..] {
                // A remainder as subtrahend is grouped and carried by the
                // accusative pronoun of its (neuter) result noun.
                if matches!(split_arithmetic(strip_outer_parens(s)), Some((_, '-', _))) {
                    let inner = arithmetic_in(strip_outer_parens(s), Vibhakti::Prathama, scope, morphology)?;
                    subtrahends.push(format!("यत् {} इति इतीदम्", inner));
                    continue;
                }
                subtrahends.push(operand_in(s, Vibhakti::Dvitiya, scope, morphology)?);
            }
            let list = if subtrahends.len() == 1 { subtrahends.remove(0) } else { format!("{} च", subtrahends.join(" च ")) };
            Ok(format!("{} {} विशोध्य {}", minuend, list, noun))
        }
        _ => {
            // The quotient is `लब्धम्` as a value or accusative, `लब्धात्` as
            // a minuend, whose dividend may not itself be a subtraction.
            let last_noun = match case {
                Vibhakti::Prathama | Vibhakti::Dvitiya => "लब्धम्",
                Vibhakti::Panchami => "लब्धात्",
                Vibhakti::Tritiya => "लब्धेन",
                Vibhakti::Shashthi => "लब्धस्य",
                _ => return Err(refuse()),
            };
            let operands = chain('/');
            if case == Vibhakti::Panchami && matches!(split_arithmetic(strip_outer_parens(operands[0])), Some((_, '-', _))) {
                return Err(refuse());
            }
            let mut out = operand_in(operands[0], Vibhakti::Dvitiya, scope, morphology)?;
            for (i, d) in operands[1..].iter().enumerate() {
                let divisor = operand_in(d, Vibhakti::Tritiya, scope, morphology)?;
                let noun = if i + 2 == operands.len() { last_noun } else { "लब्धम्" };
                out = format!("{} {} विभज्य {}", out, divisor, noun);
            }
            Ok(out)
        }
    }
}

/// A negative value in `case`: a constant with its sign (`-५`), or a column
/// after `ऋणात्मक` in the form that agrees with it. SSQL names anything longer
/// first, so it is refused here.
fn negative_of(
    value: &str,
    case: Vibhakti,
    scope: &QueryScope,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let value = value.trim();
    if value.chars().all(|c| c.is_ascii_digit() || c == '.') && !value.is_empty() {
        return Ok(format!("-{}", transpile_value_expr(value, scope, morphology)?));
    }
    if let Some((tbl, col, qualify)) = scope.resolve_col(value) {
        let word = crate::morphology::negative_word(col.gender, Vachana::Eka, case)
            .ok_or_else(|| anyhow::anyhow!("no form of `ऋणात्मक` for column `{}`", value))?;
        return Ok(format!("{} {}", word, format_col_with_vibhakti(tbl, col, qualify, case, morphology)));
    }
    anyhow::bail!("`-{}`: SSQL makes a column or a constant negative; name this value first (a CTE), then make the name negative", value)
}

/// One operand of a gerund in `case`: a column in that case, a number, or
/// arithmetic whose result noun takes the case.
fn operand_in(
    expr: &str,
    case: Vibhakti,
    scope: &QueryScope,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let inner = strip_outer_parens(expr);
    if let Some((tbl, col, qualify)) = scope.resolve_col(inner) {
        return Ok(format_col_with_vibhakti(tbl, col, qualify, case, morphology));
    }
    if split_arithmetic(inner).is_some() {
        return arithmetic_in(inner, case, scope, morphology);
    }
    if let Some(value) = inner.strip_prefix('-') {
        return negative_of(value, case, scope, morphology);
    }
    if inner.chars().all(|c| c.is_ascii_digit() || c == '.') && !inner.is_empty() {
        return transpile_value_expr(inner, scope, morphology);
    }
    anyhow::bail!("`{}`: SSQL cannot yet write this value as an operand of a subtraction or division", inner)
}

/// Split arithmetic at its last top-level `+`/`-` (binary, not a sign), or
/// failing that its last top-level `*`/`/`: the operator applied last, so
/// operators of one precedence associate to the left.
fn split_arithmetic(s: &str) -> Option<(&str, char, &str)> {
    let s = s.trim();
    let mut in_single = false;
    let mut depth = 0i32;
    let (mut additive, mut multiplicative) = (None, None);
    let mut prev: Option<char> = None;
    for (idx, c) in s.char_indices() {
        match c {
            '\'' => in_single = !in_single,
            '(' if !in_single => depth += 1,
            ')' if !in_single => depth = (depth - 1).max(0),
            '+' | '-' if !in_single && depth == 0 => {
                // Binary only after an operand, not after another operator.
                if prev.map_or(false, |p| !matches!(p, '+' | '-' | '*' | '/' | '(')) {
                    additive = Some((idx, c));
                }
            }
            '*' | '/' if !in_single && depth == 0 && idx > 0 => multiplicative = Some((idx, c)),
            _ => {}
        }
        if !c.is_whitespace() {
            prev = Some(c);
        }
    }
    additive.or(multiplicative).map(|(idx, c)| (s[..idx].trim(), c, s[idx + 1..].trim()))
}

/// `s` without one pair of parentheses around the whole of it.
fn strip_outer_parens(s: &str) -> &str {
    let t = s.trim();
    if !(t.starts_with('(') && t.ends_with(')')) {
        return t;
    }
    let mut depth = 0i32;
    for (idx, c) in t.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && idx != t.len() - 1 {
                    return t; // `(a) + (b)`: the first group closes early
                }
            }
            _ => {}
        }
    }
    t[1..t.len() - 1].trim()
}

/// Split a condition at every top-level `kw` (AND or OR). The AND of a
/// `BETWEEN low AND high` belongs to that condition and is not a split point.
fn split_connective(cond: &str, kw: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = cond;
    loop {
        let mut from = 0;
        let found = loop {
            let Some(p) = find_top_level_keyword(&rest[from..], kw) else { break None };
            let at = from + p;
            let before = &rest[..at];
            let between_open = find_top_level_keyword(before, "BETWEEN")
                .map_or(false, |b| find_top_level_keyword(&before[b + "BETWEEN".len()..], "AND").is_none());
            if kw == "AND" && between_open {
                from = at + kw.len();
                continue;
            }
            break Some(at);
        };
        match found {
            Some(at) => {
                parts.push(rest[..at].trim().to_string());
                rest = &rest[at + kw.len()..];
            }
            None => {
                parts.push(rest.trim().to_string());
                return parts;
            }
        }
    }
}

/// A च- or वा-list of conditions: every operand takes its own particle, the
/// last one too (expert review L1). An operand that is itself a list of the
/// other connective is grouped with `यत् … इति`, since one list never mixes
/// च and वा.
fn conjunction(
    operands: &[String],
    particle: &str,
    other_kw: &str,
    scope: &QueryScope,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let mut out = Vec::new();
    for operand in operands {
        let ssql = transpile_condition(operand, scope, morphology)?;
        if split_connective(operand, other_kw).len() > 1 {
            out.push(format!("यत् {} इति {}", ssql, particle));
        } else {
            out.push(format!("{} {}", ssql, particle));
        }
    }
    Ok(out.join(" "))
}

fn transpile_condition(
    cond: &str,
    scope: &QueryScope,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let trimmed = cond.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') && is_wrapped_in_single_paren_pair(trimmed) {
        let inner = transpile_condition(&trimmed[1..trimmed.len() - 1], scope, morphology)?;
        return Ok(format!("यत् {} इति", inner));
    }

    // OR binds looser than AND, so it is split first.
    let ors = split_connective(trimmed, "OR");
    if ors.len() > 1 {
        return conjunction(&ors, "वा", "AND", scope, morphology);
    }
    let ands = split_connective(trimmed, "AND");
    if ands.len() > 1 {
        return conjunction(&ands, "च", "OR", scope, morphology);
    }

    // BETWEEN comes after the splits, which leave its own AND alone: before
    // them it read `a = 1 AND b BETWEEN 2 AND 3` as a BETWEEN whose operand is
    // `a = 1 AND b`.
    if let Some(between_pos) = find_top_level_keyword(trimmed, "BETWEEN") {
        let lhs_str = trimmed[..between_pos].trim();
        let rest = trimmed[between_pos + "BETWEEN".len()..].trim();
        if let Some(and_pos) = find_top_level_keyword(rest, "AND") {
            let low_str = rest[..and_pos].trim();
            let high_str = rest[and_pos + "AND".len()..].trim();
            if find_top_level_keyword(high_str, "AND").is_none()
                && find_top_level_keyword(high_str, "OR").is_none()
            {
                let (lhs_str, verb) = negated(lhs_str);
                let lhs_ssql = transpile_value_expr(lhs_str, scope, morphology)?;
                let low_ssql = transpile_value_expr(low_str, scope, morphology)?;
                let high_ssql = transpile_value_expr(high_str, scope, morphology)?;
                return Ok(format!(
                    "{} {} च {} च इत्यनयोः मध्ये {}",
                    lhs_ssql, low_ssql, high_ssql, verb
                ));
            }
        }
    }

    // IS NOT NULL / IS NULL
    if let Some(is_not_null_pos) = find_top_level_keyword(trimmed, "IS NOT NULL") {
        let lhs_str = trimmed[..is_not_null_pos].trim();
        let (lhs_ssql, gender, vachana) = resolve_lhs_with_metadata(lhs_str, scope, morphology)?;
        let rikta = concordant_adjective("NULL", gender, vachana);
        return Ok(format!("{} {} नास्ति", lhs_ssql, rikta));
    }
    if let Some(is_null_pos) = find_top_level_keyword(trimmed, "IS NULL") {
        let lhs_str = trimmed[..is_null_pos].trim();
        let (lhs_ssql, gender, vachana) = resolve_lhs_with_metadata(lhs_str, scope, morphology)?;
        let rikta = concordant_adjective("NULL", gender, vachana);
        return Ok(format!("{} {} अस्ति", lhs_ssql, rikta));
    }

    // LIKE
    if let Some(like_pos) = find_top_level_keyword(trimmed, "LIKE") {
        let (lhs_str, verb) = negated(trimmed[..like_pos].trim());
        let rhs_str = trimmed[like_pos + "LIKE".len()..].trim();
        let (lhs_ssql, gender, vachana) = resolve_lhs_with_metadata(lhs_str, scope, morphology)?;
        let rhs_ssql = transpile_value_expr(rhs_str, scope, morphology)?;
        let pronoun = concordant_pronoun("=", gender, vachana);
        let adjective = match gender {
            Linga::Masculine => "सदृशः",
            Linga::Feminine => "सदृशी",
            Linga::Neuter => "सदृशम्",
        };
        return Ok(format!("{} {} {} {} {}", lhs_ssql, rhs_ssql, pronoun, adjective, verb));
    }

    // IN (...)
    if let Some(in_pos) = find_top_level_keyword(trimmed, "IN") {
        let lhs_str = trimmed[..in_pos].trim();
        let rhs_str = trimmed[in_pos + "IN".len()..].trim();
        if rhs_str.starts_with('(') && rhs_str.ends_with(')') {
            let items = split_top_level_commas(&rhs_str[1..rhs_str.len() - 1]);
            let mut item_ssqls = Vec::new();
            for it in &items {
                item_ssqls.push(transpile_value_expr(it, scope, morphology)?);
            }
            let (lhs_str, verb) = negated(lhs_str);
            let (lhs_ssql, gender, _) = resolve_lhs_with_metadata(lhs_str, scope, morphology)?;
            let pronoun = match (item_ssqls.len(), gender) {
                (2, _) => "इत्यनयोः",
                (_, Linga::Feminine) => "इत्यासु",
                _ => "इत्येषु",
            };
            return Ok(format!("{} {} {} {}", lhs_ssql, cha_list(&item_ssqls), pronoun, verb));
        }
    }

    // Comparison operators: !=, <>, >=, <=, =, >, <
    for op in ["!=", "<>", ">=", "<=", "=", ">", "<"] {
        if let Some(pos) = trimmed.find(op) {
            let lhs_str = trimmed[..pos].trim();
            let rhs_str = trimmed[pos + op.len()..].trim();
            let (lhs_ssql, gender, vachana) = resolve_lhs_with_metadata(lhs_str, scope, morphology)?;
            let rhs_ssql = transpile_value_expr(rhs_str, scope, morphology)?;
            // `>=` is "not less than", `<=` "not greater than", `!=` "not
            // equal"; NULL behaves the same either way.
            let (norm_op, verb) = match op {
                "<>" | "!=" => ("=", "नास्ति"),
                ">=" => ("<", "नास्ति"),
                "<=" => (">", "नास्ति"),
                ">" => (">", "अस्ति"),
                "<" => ("<", "अस्ति"),
                _ => ("=", "अस्ति"),
            };
            let pronoun = concordant_pronoun(norm_op, gender, vachana);
            let adj = concordant_adjective(norm_op, gender, vachana);
            return Ok(format!("{} {} {} {} {}", lhs_ssql, rhs_ssql, pronoun, adj, verb));
        }
    }

    bail!("Unsupported SQL condition for reverse transpilation: {}", trimmed)
}

/// `x NOT` (before IN, LIKE or BETWEEN) is `x … नास्ति`.
fn negated(lhs: &str) -> (&str, &'static str) {
    let trimmed = lhs.trim_end();
    match trimmed.len().checked_sub(4).filter(|&i| trimmed.is_char_boundary(i) && trimmed[i..].eq_ignore_ascii_case(" NOT")) {
        Some(i) => (trimmed[..i].trim_end(), "नास्ति"),
        None => (trimmed, "अस्ति"),
    }
}

fn is_wrapped_in_single_paren_pair(s: &str) -> bool {
    let mut depth = 0i32;
    for (idx, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && idx + 1 < s.len() {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn resolve_lhs_with_metadata(
    lhs_str: &str,
    scope: &QueryScope,
    morphology: &mut MorphologyEngine,
) -> Result<(String, Linga, Vachana)> {
    let ssql = transpile_value_expr(lhs_str, scope, morphology)?;
    if let Some((_, col, _)) = scope.resolve_col(lhs_str) {
        return Ok((ssql, col.gender, col.vachana.unwrap_or(Vachana::Eka)));
    }
    let upper = lhs_str.trim().to_uppercase();
    if upper.starts_with("COUNT(") {
        return Ok((ssql, Linga::Feminine, Vachana::Eka));
    }
    Ok((ssql, Linga::Neuter, Vachana::Eka))
}

fn concordant_pronoun(op: &str, gender: Linga, vachana: Vachana) -> &'static str {
    match op {
        ">" | "<" => match (gender, vachana) {
            (Linga::Feminine, Vachana::Eka) => "इत्यस्याः",
            (_, Vachana::Dvi) => "इत्याभ्याम्",
            (Linga::Feminine, Vachana::Bahu) => "इत्यासाम्",
            (_, Vachana::Bahu) => "इत्येषाम्",
            _ => "इत्यस्मात्",
        },
        _ => match (gender, vachana) {
            (Linga::Feminine, Vachana::Eka) => "इत्यनया",
            (_, Vachana::Dvi) => "इत्याभ्याम्",
            (Linga::Feminine, Vachana::Bahu) => "इत्येताभिः",
            (_, Vachana::Bahu) => "इत्येभिः",
            _ => "इत्यनेन",
        },
    }
}

fn concordant_adjective(op: &str, gender: Linga, vachana: Vachana) -> &'static str {
    match (op, gender, vachana) {
        ("=", Linga::Masculine, Vachana::Eka) => "समः",
        ("=", Linga::Masculine, Vachana::Dvi) => "समौ",
        ("=", Linga::Masculine, Vachana::Bahu) => "समाः",
        ("=", Linga::Feminine, Vachana::Eka) => "समा",
        ("=", Linga::Feminine, Vachana::Dvi) => "समे",
        ("=", Linga::Feminine, Vachana::Bahu) => "समाः",
        ("=", Linga::Neuter, Vachana::Eka) => "समम्",
        ("=", Linga::Neuter, Vachana::Dvi) => "समे",
        ("=", Linga::Neuter, Vachana::Bahu) => "समानि",

        ("!=", Linga::Masculine, Vachana::Eka) => "भिन्नः",
        ("!=", Linga::Masculine, Vachana::Dvi) => "भिन्नौ",
        ("!=", Linga::Masculine, Vachana::Bahu) => "भिन्नाः",
        ("!=", Linga::Feminine, Vachana::Eka) => "भिन्ना",
        ("!=", Linga::Feminine, Vachana::Dvi) => "भिन्ने",
        ("!=", Linga::Feminine, Vachana::Bahu) => "भिन्नाः",
        ("!=", Linga::Neuter, Vachana::Eka) => "भिन्नम्",
        ("!=", Linga::Neuter, Vachana::Dvi) => "भिन्ने",
        ("!=", Linga::Neuter, Vachana::Bahu) => "भिन्नानि",

        (">", Linga::Masculine, Vachana::Eka) => "अधिकः",
        (">", Linga::Masculine, Vachana::Dvi) => "अधिकौ",
        (">", Linga::Masculine, Vachana::Bahu) => "अधिकाः",
        (">", Linga::Feminine, Vachana::Eka) => "अधिका",
        (">", Linga::Feminine, Vachana::Dvi) => "अधिके",
        (">", Linga::Feminine, Vachana::Bahu) => "अधिकाः",
        (">", Linga::Neuter, Vachana::Eka) => "अधिकम्",
        (">", Linga::Neuter, Vachana::Dvi) => "अधिके",
        (">", Linga::Neuter, Vachana::Bahu) => "अधिकानि",

        ("<", Linga::Masculine, Vachana::Eka) => "न्यूनः",
        ("<", Linga::Masculine, Vachana::Dvi) => "न्यूनौ",
        ("<", Linga::Masculine, Vachana::Bahu) => "न्यूनाः",
        ("<", Linga::Feminine, Vachana::Eka) => "न्यूना",
        ("<", Linga::Feminine, Vachana::Dvi) => "न्यूने",
        ("<", Linga::Feminine, Vachana::Bahu) => "न्यूनाः",
        ("<", Linga::Neuter, Vachana::Eka) => "न्यूनम्",
        ("<", Linga::Neuter, Vachana::Dvi) => "न्यूने",
        ("<", Linga::Neuter, Vachana::Bahu) => "न्यूनानि",

        ("NULL", Linga::Masculine, _) => "रिक्तः",
        ("NULL", Linga::Feminine, _) => "रिक्ता",
        ("NULL", Linga::Neuter, _) => "रिक्तम्",
        _ => "समम्",
    }
}

fn transpile_insert(
    stmt: &str,
    schema: &Schema,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let values_pos = find_top_level_keyword(stmt, "VALUES")
        .ok_or_else(|| anyhow::anyhow!("INSERT statement missing VALUES: {}", stmt))?;
    let before_values = stmt["INSERT INTO".len()..values_pos].trim();
    let open_col_p = before_values
        .find('(')
        .ok_or_else(|| anyhow::anyhow!("INSERT INTO requires explicit column list: {}", stmt))?;
    let close_col_p = before_values
        .rfind(')')
        .ok_or_else(|| anyhow::anyhow!("Malformed INSERT column list: {}", stmt))?;

    let sql_table = before_values[..open_col_p].trim();
    let tbl = find_table_by_sql_name(schema, sql_table)
        .ok_or_else(|| anyhow::anyhow!("Table '{}' not found in schema.सं", sql_table))?;

    let cols = split_top_level_commas(&before_values[open_col_p + 1..close_col_p]);
    let after_values = stmt[values_pos + "VALUES".len()..].trim();
    let open_val_p = after_values
        .find('(')
        .ok_or_else(|| anyhow::anyhow!("Malformed VALUES list: {}", stmt))?;
    let close_val_p = after_values
        .rfind(')')
        .ok_or_else(|| anyhow::anyhow!("Malformed VALUES list: {}", stmt))?;
    let vals = split_top_level_commas(&after_values[open_val_p + 1..close_val_p]);

    if cols.len() != vals.len() {
        bail!("INSERT column count ({}) != value count ({})", cols.len(), vals.len());
    }

    let mut scope = QueryScope::default();
    scope.primary_table = Some(tbl);

    let tbl_saptami = decline_word(
        morphology,
        &tbl.pratipadika,
        tbl.gender,
        Vachana::Bahu,
        Vibhakti::Saptami,
    );

    let mut pairs = Vec::new();
    for (c, v) in cols.iter().zip(vals.iter()) {
        let col = find_column_in_table(tbl, c)
            .ok_or_else(|| anyhow::anyhow!("Column '{}' not found in table '{}'", c, sql_table))?;
        let col_nom = format_col_with_vibhakti(tbl, col, false, Vibhakti::Prathama, morphology);
        let val_ssql = transpile_value_expr(v, &scope, morphology)?;
        pairs.push(format!("{} {}", col_nom, val_ssql));
    }

    // The inserted columns are the verb's subject: its number agrees with
    // how many there are. Every list item carries its च.
    let verb = match pairs.len() {
        1 => "निवेश्यताम्",
        2 => "निवेश्येताम्",
        _ => "निवेश्यन्ताम्",
    };
    Ok(format!("{} {} च {} ।", tbl_saptami, pairs.join(" च "), verb))
}

fn transpile_update(
    stmt: &str,
    schema: &Schema,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let set_pos = find_top_level_keyword(stmt, "SET")
        .ok_or_else(|| anyhow::anyhow!("UPDATE statement missing SET: {}", stmt))?;
    let sql_table = stmt["UPDATE".len()..set_pos].trim();
    let tbl = find_table_by_sql_name(schema, sql_table)
        .ok_or_else(|| anyhow::anyhow!("Table '{}' not found in schema.सं", sql_table))?;

    let mut scope = QueryScope::default();
    scope.primary_table = Some(tbl);

    let after_set = stmt[set_pos + "SET".len()..].trim();
    let (set_body, where_body_opt) = if let Some(w_pos) = find_top_level_keyword(after_set, "WHERE") {
        (after_set[..w_pos].trim(), Some(after_set[w_pos + "WHERE".len()..].trim()))
    } else {
        (after_set, None)
    };

    // Each `col = value` becomes `<col> <value> इति`; several form a च-list
    // whose verb agrees in number with the columns changed.
    let mut assignments = Vec::new();
    for assignment in split_top_level_commas(set_body) {
        let (lhs, rhs) = assignment
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("Malformed SET assignment: {}", assignment))?;
        let col = find_column_in_table(tbl, lhs.trim())
            .ok_or_else(|| anyhow::anyhow!("Column '{}' not found in table '{}'", lhs.trim(), sql_table))?;
        let col_nom = format_col_with_vibhakti(tbl, col, false, Vibhakti::Prathama, morphology);
        // NULL empties the column: the adjective agreeing with it.
        let val_ssql = if rhs.trim().eq_ignore_ascii_case("NULL") {
            match col.gender {
                Linga::Masculine => "रिक्तः".to_string(),
                Linga::Feminine => "रिक्ता".to_string(),
                Linga::Neuter => "रिक्तम्".to_string(),
            }
        } else {
            transpile_value_expr(rhs.trim(), &scope, morphology)?
        };
        assignments.push(format!("{} {} इति", col_nom, val_ssql));
    }
    let set_ssql = if assignments.len() == 1 {
        assignments[0].clone()
    } else {
        assignments.iter().map(|a| format!("{} च", a)).collect::<Vec<_>>().join(" ")
    };
    let verb = match assignments.len() {
        1 => "परिवर्त्यताम्",
        2 => "परिवर्त्येताम्",
        _ => "परिवर्त्यन्ताम्",
    };

    let tbl_saptami = decline_word(
        morphology,
        &tbl.pratipadika,
        tbl.gender,
        Vachana::Bahu,
        Vibhakti::Saptami,
    );

    // A filter, or `सर्वत्र` ("everywhere") to change every row.
    let filter_ssql = match where_body_opt {
        Some(where_body) => format!("{} इति नियमेन", transpile_condition(where_body, &scope, morphology)?),
        None => "सर्वत्र".to_string(),
    };
    Ok(format!("{} {} {} {} ।", tbl_saptami, filter_ssql, set_ssql, verb))
}

fn transpile_delete(
    stmt: &str,
    schema: &Schema,
    morphology: &mut MorphologyEngine,
) -> Result<String> {
    let after_delete = stmt["DELETE FROM".len()..].trim();
    let (sql_table, where_body_opt) =
        if let Some(w_pos) = find_top_level_keyword(after_delete, "WHERE") {
            (
                after_delete[..w_pos].trim(),
                Some(after_delete[w_pos + "WHERE".len()..].trim()),
            )
        } else {
            (after_delete, None)
        };

    let tbl = find_table_by_sql_name(schema, sql_table)
        .ok_or_else(|| anyhow::anyhow!("Table '{}' not found in schema.सं", sql_table))?;

    let mut scope = QueryScope::default();
    scope.primary_table = Some(tbl);

    let tbl_panchami = decline_word(
        morphology,
        &tbl.pratipadika,
        tbl.gender,
        Vachana::Bahu,
        Vibhakti::Panchami,
    );

    if let Some(where_body) = where_body_opt {
        let cond_ssql = transpile_condition(where_body, &scope, morphology)?;
        Ok(format!("{} {} इति नियमेन निष्कास्यताम् ।", tbl_panchami, cond_ssql))
    } else {
        Ok(format!("{} सर्वम् निष्कास्यताम् ।", tbl_panchami))
    }
}
