//! Schema registry, built from native SSQL DDL (`इति आख्या ... रच्यताम्`)
//!
//! Automatically generates all 21 vibhakti forms for each declared table
//! and column using the `MorphologyEngine`.

use anyhow::{bail, Result};
use std::collections::HashMap;

use crate::morphology::MorphologyEngine;
use crate::types::{Linga, Vachana};

/// Characters that would let a `"..." इति आख्या` string break out of the SQL
/// identifier position it's placed in once embedded via `format!` in codegen
/// (statement terminator, string/identifier-quote delimiters, parens,
/// whitespace that would end an unquoted identifier early). A table or
/// column's SQL name is a single plain identifier.
const UNSAFE_IDENTIFIER_CHARS: &[char] =
    &['\'', '"', '`', ';', '(', ')', ' ', '\t', '\n', '\r', '\0'];

/// Rejects a table/column SQL name (or a query-level `इति "..."` alias) that
/// could break out of the plain-identifier position it's emitted into. This
/// is a denylist, not an allowlist, deliberately: table/column names in this
/// project are frequently Devanagari (e.g. `पुस्तकसङ्ख्या`), and Devanagari's
/// combining vowel signs and virama are Unicode "Mark" characters rather than
/// "Letter"/"Number" - a naive `is_alphanumeric()` allowlist would reject
/// real, already-shipped identifiers. Denylisting the specific characters
/// that actually matter for SQL syntax (quotes, parens, whitespace,
/// statement/comment separators) is script-agnostic and matches what's
/// actually dangerous.
pub fn validate_sql_identifier(name: &str, context: &str) -> Result<()> {
    if name.is_empty() {
        bail!("आख्या for {} is empty - a SQL name can't be blank", context);
    }
    if name.contains("--") || name.contains("/*") || name.contains("*/") {
        bail!(
            "आख्या \"{}\" for {} contains a SQL comment sequence (`--`, `/*`, or `*/`), which isn't allowed in a table/column name",
            name, context
        );
    }
    // Names are emitted unquoted, so an SQL reserved word fails in the
    // database (`CREATE TABLE order (…)`); PostgreSQL's reserved words,
    // which cover SQLite's.
    const RESERVED: &[&str] = &[
        "all", "analyse", "analyze", "and", "any", "array", "as", "asc", "asymmetric", "both", "case", "cast",
        "check", "collate", "column", "constraint", "create", "current_catalog", "current_date", "current_role",
        "current_time", "current_timestamp", "current_user", "default", "deferrable", "desc", "distinct", "do",
        "else", "end", "except", "false", "fetch", "for", "foreign", "from", "grant", "group", "having", "in",
        "initially", "intersect", "into", "lateral", "leading", "limit", "localtime", "localtimestamp", "not",
        "null", "offset", "on", "only", "or", "order", "placing", "primary", "references", "returning", "select",
        "session_user", "some", "symmetric", "system_user", "table", "then", "to", "trailing", "true", "union",
        "unique", "user", "using", "variadic", "when", "where", "window", "with",
        // Reserved as a column or table name, though allowed as a function or type.
        "authorization", "binary", "collation", "concurrently", "cross", "current_schema", "freeze", "full",
        "ilike", "inner", "is", "isnull", "join", "left", "like", "natural", "notnull", "outer", "overlaps",
        "right", "similar", "tablesample", "verbose",
        // PostgreSQL's system columns, which every table already has.
        "xmin", "xmax", "cmin", "cmax", "ctid", "tableoid",
    ];
    if RESERVED.contains(&name.to_lowercase().as_str()) {
        bail!("आख्या \"{}\" for {} is an SQL reserved word, which a table or column cannot be named; choose another", name, context);
    }
    // An unquoted name starts with a letter or `_` and holds letters, digits
    // and `_` (any non-ASCII letter, such as Devanagari, is a letter): `2nd`
    // and `a-b` are not names.
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        bail!("आख्या \"{}\" for {} starts with a digit; a table or column name starts with a letter or `_`", name, context);
    }
    if let Some(bad) = name.chars().find(|c| c.is_ascii() && !c.is_ascii_alphanumeric() && *c != '_' && !c.is_ascii_whitespace() && !c.is_ascii_control() && !UNSAFE_IDENTIFIER_CHARS.contains(c)) {
        bail!("आख्या \"{}\" for {} contains {:?}; a table or column name holds only letters, digits and `_`", name, context, bad);
    }
    if let Some(bad) = name.chars().find(|c| UNSAFE_IDENTIFIER_CHARS.contains(c) || c.is_control()) {
        bail!(
            "आख्या \"{}\" for {} contains {:?}, which isn't allowed in a table/column name (no quotes, parentheses, whitespace, or statement separators)",
            name, context, bad
        );
    }
    Ok(())
}

/// Column metadata populated from native SSQL DDL (`इति आख्या ... रच्यताम्`)
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub sql_name: String,
    pub sql_type: String,
    pub gender: Linga,        // For declension & concordance validation
    pub pratipadika: String,  // Base Sanskrit nominal stem
    pub vachana: Option<Vachana>,
}

/// Table metadata populated from native SSQL DDL (`इति आख्या ... रच्यताम्`)
#[derive(Debug, Clone)]
pub struct TableInfo {
    pub sql_name: String,
    pub gender: Linga,
    /// The number the table's word is declared in: plural, or dual for a
    /// word like दम्पती, which is then read in the dual.
    pub number: Vachana,
    pub pratipadika: String,
    pub fields: HashMap<String, ColumnInfo>,
    /// The SQL name of this table's declared primary-key column, if any.
    /// FOREIGN KEY codegen resolves this dynamically rather than assuming
    /// every table's primary key is literally named "id".
    pub primary_key_sql_name: Option<String>,
    /// Map of declensions to the pratipadika (e.g., "विक्रयणेभ्यः" -> "विक्रयण")
    pub form_map: HashMap<String, String>,
    /// Map of declined column names to their pratipadika (e.g., "मूल्येन" -> "मूल्यम्")
    pub column_form_map: HashMap<String, String>,
}

/// Result of a scoped column lookup across a specific set of tables (see
/// `Schema::resolve_column_in_tables`). A bare `Option` can't tell "the
/// column isn't on any table in scope" apart from "the column is on 2+
/// tables in scope and is genuinely ambiguous".
pub enum ScopedColumn<'a> {
    /// Found unambiguously on exactly one table in scope:
    /// (sql_column_name, sql_type, table_pratipadika).
    Found((&'a str, &'a str, &'a str)),
    /// Not present on any table in scope.
    NotFound,
    /// Present on 2+ tables in scope - caller must error rather than guess.
    /// Carries the ambiguous tables' own pratipadikas.
    Ambiguous(Vec<&'a str>),
}

/// Complete schema registry generated from native SSQL DDL statements
#[derive(Debug, Default, Clone)]
pub struct Schema {
    pub dialect: String,
    pub tables: HashMap<String, TableInfo>, // Key matches pratipadika by default
    pub literals: HashMap<String, String>,
    pub aliases: HashMap<String, String>,
    pub functions: HashMap<String, String>,
    pub udf_params: HashMap<String, Vec<String>>,
    /// CTEs and views: their columns are named by their query (often by an
    /// alias), with no declared stem.
    pub derived_tables: std::collections::HashSet<String>,
    /// Required (`अनिवार्य`) and key columns, as (table stem, SQL name):
    /// they cannot be emptied.
    pub required_columns: std::collections::HashSet<(String, String)>,
}

impl Schema {
    /// Parse schema directly from native SSQL DDL statements
    pub fn from_ddl(statements: &[crate::ast::DdlStatement], dialect: &str) -> Result<Self> {
        let mut schema = Self {
            dialect: dialect.to_string(),
            ..Default::default()
        };
        schema.update_from_ddl(statements, dialect)?;
        Ok(schema)
    }

    /// Update the current schema with DDL statements in-place (preserving existing tables)
    pub fn update_from_ddl(&mut self, statements: &[crate::ast::DdlStatement], dialect: &str) -> Result<()> {
        let mut morphology = MorphologyEngine::new()?;
        self.dialect = dialect.to_string();
        
        for stmt in statements {
            if let crate::ast::DdlStatement::CreateUdf(udf_stmt) = stmt {
                let mut param_schema = Self {
                    dialect: dialect.to_string(),
                    functions: self.functions.clone(),
                    udf_params: self.udf_params.clone(),
                    ..Default::default()
                };
                let mut fields = HashMap::new();
                let mut column_form_map = HashMap::new();
                let mut param_stems = Vec::new();

                for (idx, param) in udf_stmt.params.iter().enumerate() {
                    let p_text = &param.name.text;
                    let p_stem = param
                        .name
                        .pratipadika
                        .trim_end_matches("म्")
                        .trim_end_matches('ं')
                        .trim_end_matches('ः')
                        .to_string();
                    param_stems.push(p_stem.clone());

                    let placeholder = format!("({{{}}})", idx);
                    let sql_type = match &param.data_type {
                        crate::ast::DataType::Integer => "integer",
                        crate::ast::DataType::Text => "text",
                        crate::ast::DataType::Decimal => "numeric",
                        crate::ast::DataType::Date => "date",
                        crate::ast::DataType::Boolean => "boolean",
                        crate::ast::DataType::Custom(_) => "text",
                    }
                    .to_string();
                    let p_gender = if p_text.ends_with("म्") || p_text.ends_with('ं') {
                        Linga::Neuter
                    } else if p_text.ends_with('ा') || p_text.ends_with('ी') {
                        Linga::Feminine
                    } else {
                        Linga::Masculine
                    };

                    let col_info = ColumnInfo {
                        sql_name: placeholder,
                        sql_type,
                        gender: p_gender,
                        pratipadika: p_stem.clone(),
                        vachana: Some(Vachana::Eka),
                    };
                    fields.insert(p_stem.clone(), col_info.clone());
                    fields.insert(param.name.pratipadika.clone(), col_info.clone());
                    fields.insert(p_text.clone(), col_info);

                    column_form_map.insert(Self::normalize_sanskrit_word(p_text), p_stem.clone());
                    column_form_map.insert(
                        Self::normalize_sanskrit_word(&param.name.pratipadika),
                        p_stem.clone(),
                    );
                    column_form_map.insert(Self::normalize_sanskrit_word(&p_stem), p_stem.clone());

                    for g in [Linga::Masculine, Linga::Feminine, Linga::Neuter] {
                        for v in [Vachana::Eka, Vachana::Bahu] {
                            if let Ok(forms) = morphology.decline_with_gender(&p_stem, g, v) {
                                for (_, form) in forms.forms {
                                    column_form_map
                                        .insert(Self::normalize_sanskrit_word(&form), p_stem.clone());
                                }
                            }
                        }
                    }
                }

                param_schema.tables.insert(
                    "__udf_params".to_string(),
                    TableInfo {
                        sql_name: "__udf_params".to_string(),
                        gender: Linga::Neuter,
                        number: Vachana::Bahu,
                        pratipadika: "__udf_params".to_string(),
                        fields,
                        primary_key_sql_name: None,
                        form_map: HashMap::new(),
                        column_form_map,
                    },
                );

                let mut body_expr = udf_stmt.body.clone();
                crate::ast::AstNode::lower_expression(&mut body_expr);

                let body_sql = if dialect == "postgres" {
                    let adaptor = crate::codegen::postgres::PostgresAdaptor::new();
                    let mut codegen = crate::ast_codegen::AstCodeGen::new(&adaptor, &param_schema, vec![]);
                    codegen.generate_expression(&body_expr, None, Some("__udf_params"))?
                } else {
                    let adaptor = crate::codegen::sqlite::SQLiteAdaptor::new();
                    let mut codegen = crate::ast_codegen::AstCodeGen::new(&adaptor, &param_schema, vec![]);
                    codegen.generate_expression(&body_expr, None, Some("__udf_params"))?
                };

                let template = format!("({})", body_sql);
                let name_text = udf_stmt.name.text.clone();
                let name_stem = udf_stmt
                    .name
                    .pratipadika
                    .trim_end_matches("म्")
                    .trim_end_matches('ं')
                    .trim_end_matches('ः')
                    .to_string();

                self.functions.insert(name_text.clone(), template.clone());
                self.functions.insert(udf_stmt.name.pratipadika.clone(), template.clone());
                self.functions.insert(name_stem.clone(), template.clone());
                self.udf_params.insert(name_text.clone(), param_stems.clone());
                self.udf_params.insert(udf_stmt.name.pratipadika.clone(), param_stems.clone());
                self.udf_params.insert(name_stem, param_stems.clone());
                if let Some(sql_alias) = &udf_stmt.sql_name {
                    self.functions.insert(sql_alias.clone(), template);
                    self.udf_params.insert(sql_alias.clone(), param_stems);
                }
                continue;
            }

            if let crate::ast::DdlStatement::Create(create_stmt) = stmt {
                let sql_name = create_stmt.sql_name.clone().unwrap_or(create_stmt.name.text.clone());
                validate_sql_identifier(&sql_name, &format!("table \"{}\"", create_stmt.name.text))?;
                // Stem and gender are explicitly declared in the DDL (इति आख्या
                // <stem> इति <gender> <vachana>) - no morphological guessing.
                // Inferring stem or gender from surface forms alone would be
                // ambiguous across -इन् stems, ऊ-stems, and -आः nominative plurals.
                let gender = create_stmt.gender;
                let g_stem = create_stmt.pratipadika.clone();

                let mut form_map = HashMap::new();
                if let Ok(forms_eka) = morphology.decline_with_gender(&g_stem, gender, Vachana::Eka) {
                    for (_, form) in forms_eka.forms {
                        form_map.insert(Self::normalize_sanskrit_word(&form), g_stem.clone());
                    }
                }
                if let Ok(forms_dvi) = morphology.decline_with_gender(&g_stem, gender, Vachana::Dvi) {
                    for (_, form) in forms_dvi.forms {
                        form_map.insert(Self::normalize_sanskrit_word(&form), g_stem.clone());
                    }
                }
                if let Ok(forms_bahu) = morphology.decline_with_gender(&g_stem, gender, Vachana::Bahu) {
                    for (_, form) in forms_bahu.forms {
                        form_map.insert(Self::normalize_sanskrit_word(&form), g_stem.clone());
                    }
                }
                
                let mut fields = HashMap::new();
                let mut column_form_map = HashMap::new();
                let mut primary_key_sql_name = None;

                for col in &create_stmt.columns {
                    let field_name = col.pratipadika.clone();
                    let sql_type = match &col.data_type {
                        crate::ast::DataType::Integer => "integer",
                        crate::ast::DataType::Text => "text",
                        crate::ast::DataType::Decimal => "numeric",
                        crate::ast::DataType::Date => "date",
                        crate::ast::DataType::Boolean => "boolean",
                        crate::ast::DataType::Custom(_) => "text",
                    }.to_string();
                    let col_gender = col.gender;
                    // A column holds one value per row, so its word is
                    // singular (expert review, final): a plural column needs
                    // plural verbs and comparisons SSQL does not have, and its
                    // singular means something else (दृष्टिः "a look").
                    if col.number != Vachana::Eka {
                        anyhow::bail!(
                            "column `{}` of table `{}` is declared {}; a column holds one value per row, so declare it singular (a count as `…सङ्ख्या`, e.g. `दृष्टिसङ्ख्या` for views)",
                            col.name.text, create_stmt.name.text, if col.number == Vachana::Dvi { "dual" } else { "plural" }
                        );
                    }
                    let col_vachana = Some(col.number);
                    let col_sql_name = col.sql_name.clone().unwrap_or_else(|| field_name.clone());
                    validate_sql_identifier(&col_sql_name, &format!("column \"{}\" of table \"{}\"", col.name.text, create_stmt.name.text))?;
                    let col_stem = col.pratipadika.clone();

                    if col.constraints.iter().any(|c| matches!(c, crate::ast::Constraint::PrimaryKey)) {
                        primary_key_sql_name = Some(col_sql_name.clone());
                    }
                    if col.constraints.iter().any(|c| matches!(c, crate::ast::Constraint::PrimaryKey | crate::ast::Constraint::NotNull)) {
                        self.required_columns.insert((g_stem.clone(), col_sql_name.clone()));
                    }

                    fields.insert(field_name.clone(), ColumnInfo {
                        sql_name: col_sql_name,
                        sql_type,
                        gender: col_gender,
                        pratipadika: col_stem.clone(),
                        vachana: col_vachana,
                    });
                    
                    // The column is read in its own (singular) forms, and in the
                    // genitive plural as an aggregate's argument (`मूल्यानाम्
                    // योगः`, "the sum of the prices"); `मूल्यानि` alone is not
                    // the column.
                    if let Ok(forms) = morphology.decline_with_gender(&col_stem, col_gender, Vachana::Eka) {
                        for (_, form) in forms.forms {
                            column_form_map.insert(Self::normalize_sanskrit_word(&form), field_name.clone());
                        }
                    }
                    if let Ok(forms) = morphology.decline_with_gender(&col_stem, col_gender, Vachana::Bahu) {
                        if let Some(form) = forms.forms.get(&crate::types::Vibhakti::Shashthi) {
                            column_form_map.insert(Self::normalize_sanskrit_word(form), field_name.clone());
                        }
                    }
                }
                
                self.tables.insert(g_stem.clone(), TableInfo {
                    sql_name,
                    gender,
                    number: create_stmt.number,
                    pratipadika: g_stem.clone(),
                    fields,
                    primary_key_sql_name,
                    form_map,
                    column_form_map,
                });
            }
        }

        // A foreign key names its table in the locative plural, "tied to the
        // customers" (`ग्राहकेषु बद्धा …`; expert review M1). Checked once every
        // table is known, since a key may name one declared later.
        for stmt in statements {
            let crate::ast::DdlStatement::Create(create) = stmt else { continue };
            for column in &create.columns {
                for constraint in &column.constraints {
                    let crate::ast::Constraint::ForeignKey(target) = constraint else { continue };
                    let Some(table) = self.tables.values().find(|t| t.form_map.contains_key(&target.text)) else { continue };
                    let number = if table.number == Vachana::Dvi { Vachana::Dvi } else { Vachana::Bahu };
                    let locative = morphology
                        .decline_with_gender(&table.pratipadika, table.gender, number)
                        .ok()
                        .and_then(|d| d.forms.get(&crate::types::Vibhakti::Saptami).cloned());
                    if let Some(locative) = locative {
                        if locative != target.text {
                            bail!(
                                "`{}`: a foreign key names its table in the locative {}, `{}` (`{} बद्धा …`)",
                                target.text, if number == Vachana::Dvi { "dual" } else { "plural" }, locative, locative
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Resolve a table name from Sanskrit to SQL
    /// 
    /// Checks:
    /// 1. As a direct pratipadika match
    /// 2. As a declined form in any table's form_map
    pub fn resolve_table(&self, sanskrit: &str) -> Option<&str> {
        // 1. Direct match (Pratipadika)
        if let Some(t) = self.tables.get(sanskrit) {
            return Some(t.sql_name.as_str());
        }

        // 2. Reverse lookup via forms
        for table in self.tables.values() {
            if table.form_map.contains_key(sanskrit) {
                return Some(table.sql_name.as_str());
            }
        }
        
        None
    }

    /// Resolve a table name from Sanskrit to its declared primary-key
    /// column's SQL name. Mirrors resolve_table's lookup order (direct
    /// pratipadika match, then reverse lookup via form_map).
    pub fn resolve_table_primary_key(&self, sanskrit: &str) -> Option<&str> {
        if let Some(t) = self.tables.get(sanskrit) {
            return t.primary_key_sql_name.as_deref();
        }

        for table in self.tables.values() {
            if table.form_map.contains_key(sanskrit) {
                return table.primary_key_sql_name.as_deref();
            }
        }

        None
    }

    pub fn resolve_pratipadika(&self, word: &str) -> Option<&str> {
         let norm_word = Self::normalize_sanskrit_word(word);
         // 1. Is it already a pratipadika?
         if let Some(table) = self.tables.get(&norm_word) {
             return Some(table.pratipadika.as_str());
         }

         // 2. Check form maps
         for table in self.tables.values() {
             if let Some(p) = table.form_map.get(&norm_word) {
                 return Some(p);
             }
             if let Some(p) = table.column_form_map.get(&norm_word) {
                 return Some(p);
             }
         }
         None
    }
    
fn normalize_sanskrit_word(text: &str) -> String {
    let mut normalized = text.to_string();
    if normalized.ends_with('ं') {
        normalized.pop();
        normalized.push_str("म्");
    }
    normalized.replace('ण', "न")
}

    fn lookup_column<'b>(&'b self, table: &'b TableInfo, term: &str) -> Option<(&'b str, &'b str, &'b str)> {
        if let Some(col) = table.fields.get(term) {
            return Some((&col.sql_name, &col.sql_type, &table.pratipadika));
        }
        let norm_term = Self::normalize_sanskrit_word(term);
        if let Some(col_stem) = table.column_form_map.get(&norm_term) {
            if let Some(col) = table.fields.get(col_stem) {
                return Some((&col.sql_name, &col.sql_type, &table.pratipadika));
            }
        }
        // A column can also be referenced by its SQL-level आख्या (sql_name)
        // directly, not just its native Sanskrit name/declensions - e.g.
        // DROP/RENAME COLUMN "some_alias" इति लुप्यताम्/तस्मात्, where
        // "some_alias" was assigned via column_def's optional आख्या clause
        // rather than being the column's pratipadika.
        if let Some(col) = table.fields.values().find(|col| col.sql_name == term) {
            return Some((&col.sql_name, &col.sql_type, &table.pratipadika));
        }
        None
    }

    /// Resolve a column against only the given set of tables (e.g. a
    /// query's actual FROM + JOIN tables), never falling through to an
    /// unscoped whole-schema search the way resolve_column's table_context
    /// parameter does when the column isn't found on that one table.
    /// Used by codegen to re-derive a bare identifier's owning table after
    /// ast_transformer.rs has already rewritten it to a bare SQL name:
    /// resolve_column's own fallthrough can otherwise return an unrelated
    /// table's column, or go needlessly ambiguous, when tables elsewhere in
    /// a large shared schema.सं reuse the same SQL column name even though
    /// they aren't part of this query at all.
    ///
    /// Returns a `ScopedColumn` rather than a bare `Option` so the caller
    /// can distinguish "not present on any table in scope" from "present on
    /// 2+ tables in scope, genuinely ambiguous": collapsing both cases to
    /// `None` would let the caller silently fall through to a different,
    /// unscoped fallback that happened to privilege whichever table it was
    /// handed as the default context (usually the query's primary
    /// FROM-source), masking a real ambiguity instead of erroring on it.
    pub fn resolve_column_in_tables<'a>(&'a self, sanskrit: &str, table_contexts: &[String]) -> ScopedColumn<'a> {
        let mut matches: Vec<(&str, &str, &str)> = Vec::new();
        for table_name in table_contexts {
            let pratipadika = self.resolve_pratipadika(table_name).unwrap_or(table_name.as_str());
            if let Some(table) = self.tables.get(pratipadika) {
                if let Some(res) = self.lookup_column(table, sanskrit) {
                    if !matches.iter().any(|m| m.2 == res.2) {
                        matches.push(res);
                    }
                }
            }
        }
        match matches.len() {
            0 => ScopedColumn::NotFound,
            1 => ScopedColumn::Found(matches[0]),
            _ => ScopedColumn::Ambiguous(matches.into_iter().map(|m| m.2).collect()),
        }
    }

    /// Resolve a column name, optionally scoped to a table
    /// Returns (sql_column_name, sql_type, table_pratipadika)
     pub fn resolve_column(&self, sanskrit: &str, table_context: Option<&str>) -> Option<(&str, &str, &str)> {
        let norm_sanskrit = Self::normalize_sanskrit_word(sanskrit);

        if let Some(table_name) = table_context {
            // Context might be a form (e.g. "पुस्तकेभ्यः"), resolve to pratipadika first
            let pratipadika = self.resolve_pratipadika(table_name).unwrap_or(table_name);
            
            let mut candidate_tables = Vec::new();
            if let Some(table) = self.tables.get(pratipadika) {
                candidate_tables.push(table);
            }
            for table in self.tables.values() {
                if table.sql_name == pratipadika && !candidate_tables.iter().any(|t| t.pratipadika == table.pratipadika) {
                    candidate_tables.push(table);
                }
            }
            
            // 1. Prioritize candidate tables that actually contain the column (either directly or inflected)
            for table in &candidate_tables {
                if let Some(res) = self.lookup_column(table, sanskrit) {
                    return Some(res);
                }
            }
            
            // 2. Fallback: try first matching table anyway
            if let Some(table) = candidate_tables.first() {
                if let Some(res) = self.lookup_column(table, sanskrit) {
                    return Some(res);
                }
            }
        }
        
        // Otherwise, search all tables for unique match
        let mut matches: Vec<(&str, &ColumnInfo)> = Vec::new();
        for (table_name, table) in &self.tables {
            // 1. Direct match
            if let Some(col) = table.fields.get(sanskrit) {
                matches.push((table_name, col));
            } else if let Some(col) = table.fields.get(&norm_sanskrit) {
                matches.push((table_name, col));
            } else if let Some(col_stem) = table.column_form_map.get(&norm_sanskrit) {
                // 2. Declined match
                if let Some(col) = table.fields.get(col_stem) {
                    matches.push((table_name, col));
                }
            } else if let Some(col) = table.fields.values().find(|col| col.sql_name == sanskrit) {
                // 3. Match by SQL-level आख्या (sql_name) directly
                matches.push((table_name, col));
            }
        }
        
        match matches.len() {
            0 => None,
            1 => Some((&matches[0].1.sql_name, &matches[0].1.sql_type, matches[0].0)),
            _ => None, // Ambiguous - caller needs to qualify
        }
    }
    
    /// Get the type of a column if known
    pub fn get_column_type(&self, sanskrit: &str, table_context: Option<&str>) -> Option<&str> {
        self.resolve_column(sanskrit, table_context).map(|(_, t, _)| t)
    }

    /// Get the gender of a column if known. The form map is keyed by the
    /// normalised word (ण as न), so a column with ण was looked up raw and
    /// never found: its agreement went unchecked.
    pub fn get_column_gender(&self, sanskrit: &str, table_context: Option<&str>) -> Option<Linga> {
        if let Some((_, _, table_pratipadika)) = self.resolve_column(sanskrit, table_context) {
            if let Some(table) = self.tables.get(table_pratipadika) {
                let col_stem = table.column_form_map.get(&Self::normalize_sanskrit_word(sanskrit)).map(|s| s.as_str()).unwrap_or(sanskrit);
                if let Some(col) = table.fields.get(col_stem) {
                    return Some(col.gender);
                }
            }
        }
        None
    }

    /// The form of a column in `vibhakti`, declined from its declared stem,
    /// gender and number; None for a name that is not a column.
    pub fn column_case_form(&self, sanskrit: &str, table_context: Option<&str>, vibhakti: crate::types::Vibhakti) -> Option<String> {
        let (_, _, table_pratipadika) = self.resolve_column(sanskrit, table_context)?;
        let table = self.tables.get(table_pratipadika)?;
        let field = table.column_form_map.get(&Self::normalize_sanskrit_word(sanskrit)).map(|s| s.as_str()).unwrap_or(sanskrit);
        let col = table.fields.get(field)?;
        let mut morphology = MorphologyEngine::new().ok()?;
        let forms = morphology.decline_with_gender(&col.pratipadika, col.gender, col.vachana.unwrap_or(Vachana::Eka)).ok()?;
        forms.forms.get(&vibhakti).cloned()
    }

    /// Get the vachana of a column if known
    pub fn get_column_vachana(&self, sanskrit: &str, table_context: Option<&str>) -> Option<Vachana> {
        if let Some((_, _, table_pratipadika)) = self.resolve_column(sanskrit, table_context) {
            if let Some(table) = self.tables.get(table_pratipadika) {
                let col_stem = table.column_form_map.get(&Self::normalize_sanskrit_word(sanskrit)).map(|s| s.as_str()).unwrap_or(sanskrit);
                if let Some(col) = table.fields.get(col_stem) {
                    return col.vachana;
                }
            }
        }
        None
    }
    
    /// Check if a column is numeric (safe for SUM/AVG)
    pub fn is_numeric_column(&self, sanskrit: &str, table_context: Option<&str>) -> bool {
        if let Some((_, col_type, _)) = self.resolve_column(sanskrit, table_context) {
            matches!(col_type, "integer" | "numeric" | "real")
        } else {
            false
        }
    }
    
    /// Resolve a literal value
    pub fn resolve_literal(&self, sanskrit: &str) -> Option<&str> {
        self.literals.get(sanskrit).map(|s| s.as_str())
    }
    
    /// Resolve an alias
    pub fn resolve_alias(&self, sanskrit: &str) -> Option<&str> {
        self.aliases.get(sanskrit).map(|s| s.as_str())
    }
    
    /// Identify the Semantic Type of a symbol
    pub fn identify_symbol(&self, text: &str) -> crate::types::SymbolType {
        use crate::types::SymbolType;

        // 1. Is it a Table?
        if self.resolve_table(text).is_some() {
            return SymbolType::Table;
        }

        // 2. Is it a Column?
        // Note: This is a loose check across all tables. Context-aware check is done during validation.
        if self.resolve_column(text, None).is_some() {
            return SymbolType::Column;
        }

        // 3. Is it a Parameter? (Starts with ?)
        if text.starts_with('?') {
            return SymbolType::Variable;
        }
        
        // 4. Default to Unknown
        SymbolType::Unknown
    }

    /// The SQL type a computed column holds, as far as its expression shows:
    /// a number for an aggregate, a window function or arithmetic; text for
    /// a text function, a text literal or a CASE of texts; "" when it does
    /// not show. Every computed column used to be text, and PostgreSQL
    /// refused the COLLATE "C" or `::date` that text is given on a length,
    /// a month or a CASE of numbers.
    fn computed_type(expr: &crate::ast::Expression) -> &'static str {
        use crate::ast::{DataType, Expression, Literal};
        const TEXT_FUNCTIONS: &[&str] = &[
            "महावर्णरूपम्", "लघुवर्णरूपम्", "आद्यमहावर्णरूपम्", "संयोजनम्", "खण्डः",
            "आदिखण्डः", "अन्त्यखण्डः", "मार्जनम्", "वाममार्जनम्", "दक्षिणमार्जनम्",
            "प्रतिस्थापनम्", "विपरीतम्", "पुनरावृत्तिः", "विभाजनखण्डः",
        ];
        match expr {
            Expression::Aggregate { .. } | Expression::FunctionCall { window: Some(_), .. } | Expression::BinaryOp { .. } => "numeric",
            Expression::FunctionCall { name, .. } if TEXT_FUNCTIONS.contains(&name.as_str()) => "text",
            Expression::Literal(Literal::String(_)) => "text",
            Expression::Literal(Literal::Number(_)) => "numeric",
            Expression::Cast { data_type, .. } => match data_type {
                DataType::Text => "text",
                DataType::Integer | DataType::Decimal => "numeric",
                DataType::Date => "date",
                _ => "",
            },
            Expression::Case { cases } => {
                let results: Vec<&str> = cases.iter()
                    .filter(|c| !matches!(&*c.result, Expression::Literal(Literal::Null)))
                    .map(|c| Self::computed_type(&c.result))
                    .collect();
                match results.first() {
                    Some(&first) if results.iter().all(|&r| r == first) => first,
                    _ => "",
                }
            }
            _ => "",
        }
    }

    /// The gender a name shows in its nominative singular: `-म्` neuter,
    /// `-ः` masculine, `-ा`/`-ी` feminine; None for an ending that does not
    /// tell (a consonant stem's `नाम`).
    fn gender_of_name(name: &str) -> Option<Linga> {
        if name.ends_with("म्") {
            Some(Linga::Neuter)
        } else if name.ends_with('ः') {
            Some(Linga::Masculine)
        } else if name.ends_with('ा') || name.ends_with('ी') {
            Some(Linga::Feminine)
        } else {
            None
        }
    }

    /// Dynamically register a CTE and infer its column types/genders from SELECT items
    ///
    /// `joined_table_contexts` are the pratipadikas of the CTE's own JOINed
    /// tables (source + joins together is the query's full table scope,
    /// same notion as AstCodeGen::active_query_tables) - passed separately
    /// from `default_table_context` (the CTE's primary FROM-source) because
    /// the self-shadow check and the सर्वम्/wildcard column-copy below are
    /// both deliberately source-only, not join-inclusive.
    pub fn register_cte(&mut self, name: &crate::ast::Identifier, fields_to_infer: &[crate::ast::SelectItem], default_table_context: Option<&str>, joined_table_contexts: &[String]) -> Result<()> {
        let mut morphology = MorphologyEngine::new()?;
        let mut fields = HashMap::new();
        let mut column_form_map = HashMap::new();

        // 1. CTE pratipadika and gender are explicitly declared (इति आख्या
        // <stem> इति <gender> <vachana>, see parse_var_decl in cte.rs) - no
        // morphological guessing (same reasoning as update_from_ddl above).
        let stem = name.pratipadika.clone();
        let gender = name.vibhakti_form.linga.unwrap_or(Linga::Masculine);

        // Reject a CTE whose declared name is identical to its own
        // FROM-source table, rather than silently overwriting that
        // table's entry below. self.tables is a single flat map shared
        // between real DDL tables and every query's CTEs, keyed only by
        // stem - an unconditional overwrite here destroys the source
        // table's full column schema, replacing it with this CTE's own
        // (usually much narrower) output-column set, which then breaks
        // resolution of any of the source table's own columns that
        // aren't also surfaced as CTE outputs under the same name. Real
        // SQL allows a CTE to shadow its own source table's name
        // (`WITH sales AS (SELECT * FROM sales ...)
        // ...`), but this schema has no per-query scoping to support
        // that safely yet.
        //
        // Deliberately narrow: only rejects true self-shadowing (CTE
        // name == its own FROM-source), not a CTE reusing a name that
        // happens to already exist elsewhere in the schema for other
        // reasons - some schemas pre-declare a CTE's expected shape
        // as a placeholder table in schema.सं (e.g., physical name
        // "cte_first_student").
        if let Some(ctx) = default_table_context {
            if self.resolve_pratipadika(ctx).unwrap_or(ctx) == stem {
                bail!(
                    "CTE name '{}' collides with its own FROM-source table's name - \
                     rename the CTE to something distinct from the table it selects from \
                     (SSQL does not yet support a CTE shadowing its own source table's name)",
                    stem
                );
            }
        }

        // Generate declensions for the CTE Table Name itself (e.g. "पुस्तकविक्रयणसारः", "पुस्तकविक्रयणसारात्")
        let mut form_map = HashMap::new();
        // Decline in all cases (singular & plural) to populate form_map.
        // Keys must be normalized the same way update_from_ddl normalizes
        // real tables' form_map keys - resolve_pratipadika always
        // normalizes its lookup word before checking form_map, so an
        // un-normalized key here (e.g. one containing retroflex ण) can
        // never match, even though resolve_table's raw/un-normalized
        // lookup succeeds on the exact same data.
        if let Ok(forms) = morphology.decline_with_gender(&stem, gender, Vachana::Eka) {
            for (_, form) in forms.forms {
                form_map.insert(Self::normalize_sanskrit_word(&form), stem.clone());
            }
        }
        if let Ok(forms) = morphology.decline_with_gender(&stem, gender, Vachana::Bahu) {
            for (_, form) in forms.forms {
                form_map.insert(Self::normalize_sanskrit_word(&form), stem.clone());
            }
        }

        // 2. Infer fields from the SELECT items of the CTE subquery
        for item in fields_to_infer {
            // A wildcard SELECT item (सर्वम्, "all") codegens to a literal
            // "*" (see ast_codegen.rs's Expression::Wildcard arm) - at the
            // SQL level this expands to every column of the CTE's own
            // FROM-source. Copy the source table's already-resolved fields
            // and declensions so downstream queries referencing carried-forward
            // columns resolve against the CTE schema.
            if matches!(item.expr, crate::ast::Expression::Wildcard) {
                if let Some(source) = default_table_context.and_then(|ctx| self.tables.get(ctx)) {
                    fields.extend(source.fields.clone());
                    column_form_map.extend(source.column_form_map.clone());
                }
                continue;
            }

            // Determine column alias or fallback name
            let field_name = if let Some(alias) = &item.alias {
                alias.clone()
            } else {
                match &item.expr {
                    crate::ast::Expression::Identifier(id) => id.text.clone(),
                    crate::ast::Expression::FieldAccess { field, .. } => field.text.clone(),
                    _ => "column".to_string(),
                }
            };

            // Infer type and gender: lookup in existing tables, else what
            // the expression computes.
            let mut col_type = Self::computed_type(&item.expr).to_string();
            let mut col_gender = Linga::Neuter;
            // The CTE's exposed SQL name for this column: when the SELECT
            // item carries an explicit alias, codegen emits "... AS <alias>"
            // in the WITH-clause, so callers must reference it by that
            // alias - NOT by the underlying source column's own SQL name
            // (which is only what appears, unaliased, when there is no
            // explicit alias). Without this distinction, two aliased CTE
            // columns that both trace back to identically-named source
            // columns (e.g. two different tables' "name" column) collapse
            // to the same wrong sql_name here, silently producing SQL that
            // references a nonexistent column downstream.
            let has_alias = item.alias.is_some();
            let mut resolved_sql_name = field_name.clone();

            // Simple lookup if expr is a direct identifier or field access
            let lookup_field = match &item.expr {
                crate::ast::Expression::Identifier(id) => Some((id.text.as_str(), None)),
                crate::ast::Expression::FieldAccess { object, field } => {
                    if let crate::ast::Expression::Identifier(obj_id) = &**object {
                        Some((field.text.as_str(), Some(obj_id.pratipadika.as_str())))
                    } else {
                        Some((field.text.as_str(), None))
                    }
                }
                _ => None,
            };

            if let Some((f_text, t_context)) = lookup_field {
                // A bare, unqualified SELECT item (no genitive table-
                // qualifier, e.g. plain "शुल्कम्" rather than "छात्रस्य
                // शुल्कम्") has no explicit t_context - it unambiguously
                // belongs to the CTE's own FROM-source table, not to
                // "whichever table happens to be checked first". Falling
                // through to a schema-wide, unscoped search in that case
                // (the previous behavior) meant that if two or more tables
                // anywhere in the schema share a column with the same
                // pratipadika, which one won depended on HashMap iteration
                // order - genuinely non-deterministic across process runs,
                // not just "wrong in some fixed way" (confirmed empirically:
                // the same query compiled to different, sometimes-invalid
                // SQL referencing a column absent from the CTE's own output,
                // across repeated invocations of the same binary on the
                // same input). Prefer the CTE's declared source table first.
                // Try the scoped context first (explicit qualifier, or the
                // CTE's own default source table for a bare item); only if
                // that fails to find anything, try each of the CTE's own
                // JOINed tables in declaration order: default_table_context
                // alone only covers the primary FROM-source, never a JOINed
                // table, so without this a bare column legitimately owned
                // by a JOINed table would skip straight to the fully
                // unscoped search below even though it belonged to a table
                // already known to be in scope.
                // Only if none of the query's own tables have the column
                // do we fall back to the old, unscoped whole-schema search
                // so already-working CTEs relying on some other match
                // (e.g. a correlated outer-query column) aren't regressed.
                let mut resolved = false;
                let mut ordered_contexts: Vec<Option<&str>> = vec![t_context.or(default_table_context)];
                for joined in joined_table_contexts {
                    ordered_contexts.push(Some(joined.as_str()));
                }
                ordered_contexts.push(None);
                for scoped_context in ordered_contexts {
                    for (table_name, table_info) in &self.tables {
                        if let Some(context) = scoped_context {
                            if table_name != context {
                                continue;
                            }
                        }
                        let resolved_col_opt = if let Some(real_col) = table_info.column_form_map.get(f_text) {
                            table_info.fields.get(real_col)
                        } else {
                            table_info.fields.get(f_text)
                        };
                        if let Some(col_info) = resolved_col_opt {
                            col_type = col_info.sql_type.clone();
                            col_gender = col_info.gender;
                            if !has_alias {
                                resolved_sql_name = col_info.sql_name.clone();
                            }
                            resolved = true;
                            break;
                        }
                    }
                    if resolved {
                        break;
                    }
                }
            }

            // An alias is a word of its own and agrees in its own gender
            // (`पुस्तकगणना` feminine, though a count of books); a computed
            // column used to be neuter whatever its name.
            if has_alias {
                if let Some(g) = Self::gender_of_name(&field_name) {
                    col_gender = g;
                }
            }

            // Decline the inferred field name and populate column_form_map
            let mut col_stem = field_name.clone();
            if col_gender == Linga::Masculine && col_stem.ends_with("ः") {
                col_stem = col_stem.strip_suffix("ः").unwrap_or(&col_stem).to_string();
            } else if col_gender == Linga::Feminine && col_stem.ends_with("ा") {
                col_stem = col_stem.strip_suffix("ा").unwrap_or(&col_stem).to_string();
            } else if col_gender == Linga::Neuter && col_stem.ends_with("म्") {
                col_stem = col_stem.strip_suffix("म्").unwrap_or(&col_stem).to_string();
            }

            if let Ok(forms) = morphology.decline_with_gender(&col_stem, col_gender, Vachana::Eka) {
                for (_, form) in forms.forms {
                    column_form_map.insert(Self::normalize_sanskrit_word(&form), field_name.clone());
                }
            }
            if let Ok(forms) = morphology.decline_with_gender(&col_stem, col_gender, Vachana::Bahu) {
                for (_, form) in forms.forms {
                    column_form_map.insert(Self::normalize_sanskrit_word(&form), field_name.clone());
                }
            }

            fields.insert(field_name.clone(), ColumnInfo {
                sql_name: resolved_sql_name,
                sql_type: col_type,
                gender: col_gender,
                pratipadika: col_stem.clone(),
                vachana: None,
            });
        }

        // 3. Register CTE in the Schema
        self.derived_tables.insert(stem.clone());
        self.tables.insert(stem.clone(), TableInfo {
            sql_name: stem.clone(),
            gender,
            number: Vachana::Bahu,
            pratipadika: stem,
            fields,
            primary_key_sql_name: None,
            form_map,
            column_form_map,
        });

        Ok(())
    }
}
