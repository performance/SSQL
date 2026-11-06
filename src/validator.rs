
//! # SSQL Validator
//!
//! Checks the AST for compliance with the schema (built from native SSQL
//! DDL, see `schema.rs`) - specifically, ensures that all defined Aliases
//! and CTEs correspond to a known Pratipadika in the SymbolTable.

use crate::ast::{AstNode, Statement, DqlStatement, Expression, SetOpStatement};
use crate::schema::Schema;
use crate::types::{Linga, Vachana, Vibhakti};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ValidationDiagnostic {
    pub identifier: String,
    pub suggested_pratipadika: Option<String>,
    pub suggested_gender: Option<String>,
    pub context: String, // "CTE" or Scope Name (e.g. "PustakaSankhya")
}

pub struct Validator<'a> {
    schema: &'a Schema,
    diagnostics: Vec<ValidationDiagnostic>,
    current_scope: Option<String>,
    current_table_context: Option<String>,
    pub warnings: Vec<String>,
    cte_columns: HashMap<String, Vec<(Option<Linga>, Option<Vachana>, Option<String>)>>,
    /// The number each CTE is declared in (`… इति पुंलिङ्गम् बहुवचनम्`).
    cte_numbers: HashMap<String, Vachana>,
    // Reserved builtin function names (e.g. स्थानम्,
    // श्रेणी) are already rejected with a clear error when used as a DDL
    // stem declaration (see ssql_parser::ddl::parse_stem_gender_vachana),
    // but a SELECT-item alias colliding with the same reserved words is not
    // caught during parsing and would otherwise surface as a confusing,
    // unrelated downstream parser error. Collected here (rather than returned directly
    // from visit_dql, which would require threading Result through the
    // whole visitor) and surfaced as a hard error at the end of validate().
    reserved_alias_errors: Vec<String>,
}

impl<'a> Validator<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            diagnostics: Vec::new(),
            current_scope: None,
            current_table_context: None,
            warnings: Vec::new(),
            cte_columns: HashMap::new(),
            cte_numbers: HashMap::new(),
            reserved_alias_errors: Vec::new(),
        }
    }

    pub fn validate(&mut self, ast: &AstNode) -> Result<Vec<ValidationDiagnostic>> {
        self.diagnostics.clear();
        self.warnings.clear();
        self.cte_columns.clear();
        self.cte_numbers.clear();
        self.reserved_alias_errors.clear();
        self.visit_ast(ast);
        if let Some(msg) = self.reserved_alias_errors.first() {
            anyhow::bail!("{}", msg);
        }
        Ok(std::mem::take(&mut self.diagnostics))
    }

    /// The columns written are the subject of the passive verb, so its number
    /// agrees with how many there are, exactly as a SELECT's verb does:
    /// `शीर्षकम् "…" च मूल्यम् ३० च निवेश्येताम्` (two columns, dual).
    fn check_dml_verb_agreement(&mut self, dml: &crate::ast::DmlStatement) {
        fn list_len(expr: &Expression) -> usize {
            match expr {
                Expression::LogicalOp { left, op: crate::ast::LogicalOperator::And, right } => list_len(left) + list_len(right),
                _ => 1,
            }
        }
        let (count, verb, forms, action) = match dml {
            crate::ast::DmlStatement::Insert { values, verb, .. } => {
                (list_len(values), verb, ["निवेश्यताम्", "निवेश्येताम्", "निवेश्यन्ताम्"], "inserts")
            }
            crate::ast::DmlStatement::InsertSelect { columns, query, verb, .. } => {
                let n = columns.as_ref().map(|c| c.len()).unwrap_or(query.select.columns.len());
                (n, verb, ["निवेश्यताम्", "निवेश्येताम्", "निवेश्यन्ताम्"], "inserts")
            }
            crate::ast::DmlStatement::Update { set_clause, verb, .. } => {
                (list_len(&set_clause.assignments), verb, ["परिवर्त्यताम्", "परिवर्त्येताम्", "परिवर्त्यन्ताम्"], "changes")
            }
            crate::ast::DmlStatement::Delete { .. } => return,
        };
        let (expected, vachana) = match count {
            1 => (forms[0], "एकवचनम्"),
            2 => (forms[1], "द्विवचनम्"),
            _ => (forms[2], "बहुवचनम्"),
        };
        if verb != expected {
            self.warnings.push(format!(
                "Grammar Warning: Subject-Verb Agreement Mismatch. The statement {} {} {}, which needs '{}' ({}), but you used '{}'",
                action,
                count,
                if count == 1 { "column" } else { "columns" },
                expected,
                vachana,
                verb
            ));
        }
    }

    fn visit_ast(&mut self, ast: &AstNode) {
        match ast {
            AstNode::QueryBatch { statements, .. } => {
                // A CTE belongs to the one statement after it (it is emitted
                // as that statement's WITH); a later statement naming it got
                // SQL with no such table.
                let mut pending: Vec<String> = Vec::new();
                let mut consumed: Vec<String> = Vec::new();
                for stmt in statements {
                    if let Statement::VarDecl { name, .. } = stmt {
                        consumed.retain(|c| c != &name.pratipadika);
                        pending.push(name.pratipadika.clone());
                    } else {
                        let mut tables = Vec::new();
                        Self::statement_tables(stmt, &mut tables);
                        if let Some(t) = tables.iter().find(|t| consumed.contains(t)) {
                            self.warnings.push(format!(
                                "Grammar Warning: `{}` was declared for the statement before; a declaration belongs to the one statement after it, so declare it again here",
                                t
                            ));
                        }
                        consumed.append(&mut pending);
                    }
                    self.check_statement_has_verb(stmt);
                    self.visit_statement(stmt);
                }
            },
            AstNode::Statement(stmt) => {
                self.check_statement_has_verb(stmt);
                self.visit_statement(stmt);
            }
        }
    }

    /// A query is a sentence, and a Sanskrit sentence needs its verb: a
    /// top-level SELECT or CTE body without आह्रियताम्/-येताम्/-न्ताम् is
    /// incomplete, not an abbreviation.
    fn check_statement_has_verb(&mut self, stmt: &Statement) {
        let dql = match stmt {
            Statement::Dql(dql) => Some(dql),
            Statement::VarDecl { value, .. } => match value.as_ref() {
                Expression::Subquery(dql) => Some(dql.as_ref()),
                _ => None,
            },
            _ => None,
        };
        let Some(dql) = dql else { return };
        if dql.verb_token.is_some() {
            return;
        }
        let columns = dql.select.columns.len().max(1);
        let (verb, vachana) = match columns {
            1 => ("आह्रियताम्", "एकवचनम्"),
            2 => ("आह्रियेताम्", "द्विवचनम्"),
            _ => ("आह्रियन्ताम्", "बहुवचनम्"),
        };
        self.warnings.push(format!(
            "Missing action verb: the query projects {} {} and needs '{}' ({}) before '।'.",
            columns,
            if columns == 1 { "column" } else { "columns" },
            verb,
            vachana
        ));
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::VarDecl { name, value, .. } => {
                // CTE Definition
                // A CTE reaches SQL as its stem (`WITH प्रवाह AS`).
                self.check_name_length(&name.pratipadika);
                
                // Enter Scope
                let previous_scope = self.current_scope.clone();
                self.current_scope = Some(name.pratipadika.clone()); 
                
                self.visit_expression(value);
                
                self.cte_numbers.insert(name.pratipadika.clone(), name.vibhakti_form.vachana);
                // Infer and save columns of this CTE
                if let Some(cols) = self.infer_query_columns(value) {
                    self.cte_columns.insert(name.pratipadika.clone(), cols);
                }
                
                // Exit Scope
                self.current_scope = previous_scope;
            },
            Statement::Dql(dql) => self.visit_dql(dql),
            Statement::SetOp(set_op) => {
                self.visit_set_op(set_op);
            },
            Statement::Dml(dml) => {
                match dml {
                    crate::ast::DmlStatement::InsertSelect { query, .. } => self.visit_dql(query),
                    crate::ast::DmlStatement::Update { table, where_clause, .. } => {
                        self.visit_dml_filter(table, where_clause.as_ref())
                    }
                    crate::ast::DmlStatement::Delete { table, where_clause } => {
                        self.visit_dml_filter(table, where_clause.as_ref())
                    }
                    crate::ast::DmlStatement::Insert { .. } => {}
                }
                self.check_dml_verb_agreement(dml);
                self.check_dml_shape(dml);
            },
            Statement::Ddl(ddl) => {
                match ddl {
                    crate::ast::DdlStatement::Create(create_stmt) => {
                        self.check_table_shape(&create_stmt.columns);
                        if let Some(query) = &create_stmt.query {
                            match query.as_ref() {
                                Statement::Dql(dql) => self.visit_dql(dql),
                                Statement::SetOp(_set_op) => self.visit_statement(query),
                                _ => {}
                            }
                        }
                    }
                    crate::ast::DdlStatement::CreateIndex(create_idx) => {
                        let table_name = &create_idx.table_name;
                        let table_p = self.schema.resolve_pratipadika(&table_name.text);
                        if table_p.is_none() {
                            self.warnings.push(format!(
                                "Grammar Warning: Table '{}' is not defined in the schema",
                                table_name.text
                            ));
                        } else {
                            let table_p_str = table_p.unwrap();
                            for col in &create_idx.columns {
                                if self.schema.resolve_column(&col.name.text, Some(table_p_str)).is_none() {
                                    self.warnings.push(format!(
                                        "Grammar Warning: Column '{}' does not exist in table '{}'",
                                        col.name.text, table_name.text
                                    ));
                                }
                            }
                        }
                    }
                    crate::ast::DdlStatement::AlterTable(alter_stmt) => {
                        let table_name = &alter_stmt.table_name;
                        let table_p = self.schema.resolve_pratipadika(&table_name.text);
                        if table_p.is_none() {
                            self.warnings.push(format!(
                                "Grammar Warning: Table '{}' is not defined in the schema",
                                table_name.text
                            ));
                        } else {
                            let table_p_str = table_p.unwrap();
                            match &alter_stmt.action {
                                crate::ast::AlterAction::AddColumn(column_def) => {
                                    if self.schema.dialect == "sqlite" {
                                        for constraint in &column_def.constraints {
                                            match constraint {
                                                crate::ast::Constraint::PrimaryKey => {
                                                    self.warnings.push(
                                                        "Dialect Warning: SQLite does not support adding PRIMARY KEY columns via ALTER TABLE".to_string()
                                                    );
                                                }
                                                crate::ast::Constraint::Unique => {
                                                    self.warnings.push(
                                                        "Dialect Warning: SQLite does not support adding UNIQUE columns via ALTER TABLE".to_string()
                                                    );
                                                }
                                                crate::ast::Constraint::NotNull => {
                                                    self.warnings.push(
                                                        "Dialect Warning: Adding a NOT NULL column in SQLite via ALTER TABLE requires a DEFAULT value or a table copy".to_string()
                                                    );
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                crate::ast::AlterAction::DropColumn(col_name) => {
                                    if self.schema.resolve_column(&col_name.text, Some(table_p_str)).is_none() {
                                        self.warnings.push(format!(
                                            "Grammar Warning: Column '{}' does not exist in table '{}'",
                                            col_name.text, table_name.text
                                        ));
                                    }
                                }
                                crate::ast::AlterAction::RenameColumn { old_name, .. } => {
                                    if self.schema.resolve_column(&old_name.text, Some(table_p_str)).is_none() {
                                        self.warnings.push(format!(
                                            "Grammar Warning: Column '{}' does not exist in table '{}'",
                                            old_name.text, table_name.text
                                        ));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    crate::ast::DdlStatement::CreateUdf(udf_stmt) => {
                        {
                            let param_count = udf_stmt.params.len();
                            let expected_verb = match param_count {
                                1 => "दीयते",
                                2 => "दीयेते",
                                _ => "दीयन्ते",
                            };
                            if udf_stmt.param_verb != expected_verb {
                                self.warnings.push(format!(
                                    "Grammar Warning: UDF parameter verb number mismatch in '{}'. Declared {} parameter(s) (requires '{}'), but you used '{}'",
                                    udf_stmt.name.text, param_count, expected_verb, udf_stmt.param_verb
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            },
            Statement::Tcl(_tcl) => {}
            _ => {}
        }
    }

    fn visit_set_op(&mut self, set_op: &SetOpStatement) {
        let mut expected_arity = None;
        let mut expected_cols: Vec<(Linga, Option<String>)> = Vec::new();
        
        for q in &set_op.queries {
            self.visit_expression(q);
            // A branch is ordered or limited only as the whole result is:
            // `… LIMIT 2 UNION SELECT …` is not SQL.
            if let Expression::Subquery(branch) = q {
                if branch.order_by.is_some() || branch.limit.is_some() {
                    self.warnings.push("Grammar Warning: a branch of a set operation cannot be ordered or limited on its own; order or limit the combined result".to_string());
                }
            }
            
            if let Some(cols) = self.infer_query_columns(q) {
                let current_arity = cols.len();
                if let Some(arity) = expected_arity {
                    if current_arity != arity {
                        self.warnings.push(format!(
                            "Grammar Warning: Set operation arity mismatch. Expected {} columns, but subquery returns {} columns",
                            arity, current_arity
                        ));
                    }
                } else {
                    expected_arity = Some(current_arity);
                }
                
                let current_cols: Vec<(Linga, Option<String>)> = cols
                    .iter()
                    .map(|(g, _, sql_t)| (g.unwrap_or(Linga::Neuter), sql_t.clone()))
                    .collect();
                if expected_cols.is_empty() {
                    expected_cols = current_cols;
                } else {
                    for (i, ((g1, t1), (g2, t2))) in expected_cols.iter().zip(current_cols.iter()).enumerate() {
                        let same_sql_type = match (t1, t2) {
                            (Some(s1), Some(s2)) => s1.eq_ignore_ascii_case(s2),
                            _ => false,
                        };
                        if g1 != g2 && !same_sql_type {
                            self.warnings.push(format!(
                                "Grammar Warning: Set operation column gender mismatch at index {}. First subquery has gender {:?}, but subsequent has gender {:?}",
                                i + 1, g1, g2
                            ));
                        }
                    }
                }
            }
        }
    }

    fn infer_query_columns(&mut self, expr: &Expression) -> Option<Vec<(Option<Linga>, Option<Vachana>, Option<String>)>> {
        match expr {
            Expression::Subquery(dql) => {
                let previous_context = self.current_table_context.clone();
                let table_pratipadika = self.schema.resolve_pratipadika(&dql.source.table.text)
                    .unwrap_or(&dql.source.table.text)
                    .to_string();
                self.current_table_context = Some(table_pratipadika.clone());
                
                let mut cols = Vec::new();
                for col in &dql.select.columns {
                    // An aliased column is that noun (`… स्तरः इति आख्या`), so
                    // its gender is the alias's, read from its nominative
                    // ending; a physical alias (`"level"`) has none.
                    let alias_gender = col.alias.as_deref().and_then(|a| {
                        if a.ends_with("म्") { Some(Linga::Neuter) }
                        else if a.ends_with('ः') { Some(Linga::Masculine) }
                        else if a.ends_with('ा') || a.ends_with('ी') { Some(Linga::Feminine) }
                        else { None }
                    });
                    let (expr_gender, vachana) = self.get_expr_gender_and_vachana(&col.expr);
                    let gender = alias_gender.or(expr_gender);
                    let sql_type = self.get_expr_sql_type(&col.expr);
                    cols.push((gender, vachana, sql_type));
                }
                
                self.current_table_context = previous_context;
                Some(cols)
            }
            Expression::Identifier(id) => {
                if let Some(cols) = self.cte_columns.get(&id.pratipadika) {
                    return Some(cols.clone());
                }
                None
            }
            Expression::SetOp(set_op) => {
                if let Some(first_query) = set_op.queries.first() {
                    return self.infer_query_columns(first_query);
                }
                None
            }
            _ => None,
        }
    }

    /// An UPDATE or DELETE filter is checked like a query's: its comparisons
    /// agree with the columns of the target table.
    fn visit_dml_filter(&mut self, table: &crate::ast::Identifier, where_clause: Option<&crate::ast::WhereClause>) {
        let Some(w) = where_clause else { return };
        if Self::has_group_aggregate(&w.condition) {
            self.warnings.push(Self::AGGREGATE_IN_ROW_FILTER.to_string());
        }
        let previous_context = self.current_table_context.clone();
        let table_pratipadika = self.schema.resolve_pratipadika(&table.text)
            .unwrap_or(&table.text)
            .to_string();
        self.current_table_context = Some(table_pratipadika);
        self.visit_expression(&w.condition);
        self.current_table_context = previous_context;
    }

    fn visit_dql(&mut self, dql: &DqlStatement) {
        self.check_dql_shape(dql);
        let previous_context = self.current_table_context.clone();
        let table_pratipadika = self.schema.resolve_pratipadika(&dql.source.table.text)
            .unwrap_or(&dql.source.table.text)
            .to_string();
        self.current_table_context = Some(table_pratipadika.clone());

        // 1. Ablative source check: a table is read in the ablative plural; a
        // CTE in the ablative of the number it is declared in (a collective
        // singular such as `शिक्षकपटलम्` is read `शिक्षकपटलात्`; expert review K2);
        // a table named by a dual word (`दम्पती`) in the ablative dual.
        let source_table = &dql.source.table;
        let is_ablative = source_table.vibhakti_form.vibhakti == Vibhakti::Panchami;
        let is_plural = source_table.vibhakti_form.vachana == Vachana::Bahu;
        let cte_number = self.cte_numbers.get(&table_pratipadika)
            .or_else(|| self.cte_numbers.get(&source_table.pratipadika))
            .copied();
        if let Some(declared) = cte_number {
            if is_ablative && source_table.vibhakti_form.vachana != declared {
                self.warnings.push(format!(
                    "Grammar Warning: the CTE `{}` is declared {}, so it is read in the ablative {} (you used '{}')",
                    table_pratipadika,
                    if declared == Vachana::Bahu { "plural" } else { "singular" },
                    if declared == Vachana::Bahu { "plural" } else { "singular" },
                    source_table.text
                ));
            }
        }
        let dual_table = cte_number.is_none()
            && self.schema.tables.get(&table_pratipadika).map_or(false, |t| t.number == Vachana::Dvi);
        if dual_table {
            if !is_ablative || source_table.vibhakti_form.vachana != Vachana::Dvi {
                self.warnings.push(format!(
                    "Grammar Warning: the table `{}` is declared dual, so it is read in the ablative dual (you used '{}')",
                    table_pratipadika, source_table.text
                ));
            }
        } else if !is_ablative || (!is_plural && cte_number.is_none()) {
            self.warnings.push(format!(
                "Grammar Warning: FROM source table must be in the Ablative Plural form (Panchami Vibhakti, Bahuvachana), but you used '{}'",
                source_table.text
            ));
        }

        // 2. Subject-Verb Agreement check
        if let Some(verb) = &dql.verb_token {
            let col_count = dql.select.columns.len();
            let text = verb.text.as_str();
            
            let is_singular = text == "आह्रियताम्";
            let is_dual = text == "आह्रियेताम्";
            let is_plural = text == "आह्रियन्ताम्";
            
            let expected_vachana = match col_count {
                1 => Vachana::Eka,
                2 => Vachana::Dvi,
                _ => Vachana::Bahu,
            };
            
            let ok = match expected_vachana {
                Vachana::Eka => is_singular,
                Vachana::Dvi => is_dual,
                Vachana::Bahu => is_plural,
            };
            
            if !ok {
                let expected = match expected_vachana {
                    Vachana::Eka => "आह्रियताम्",
                    Vachana::Dvi => "आह्रियेताम्",
                    Vachana::Bahu => "आह्रियन्ताम्",
                };
                self.warnings.push(format!(
                    "Grammar Warning: Subject-Verb Agreement Mismatch. {} selected, so the verb is `{}`, not `{}`",
                    if col_count == 1 { "1 column is".to_string() } else { format!("{} columns are", col_count) },
                    expected, text
                ));
            }
        }

        // SELECT aliases are new names introduced by the query itself,
        // so we validate that they do not collide with reserved builtin
        // function names, and then recursively validate the underlying expression.
        self.check_grouping(dql);
        self.check_aggregate_placement(dql);
        self.check_distinct_order(dql);
        for col in &dql.select.columns {
             if let Some(alias) = &col.alias {
                 self.check_name_length(alias);
                 if crate::reserved_words::is_reserved_builtin(alias) {
                     self.reserved_alias_errors.push(format!(
                         "'{}' is a reserved builtin function name and cannot be used as an alias",
                         alias
                     ));
                 }
             }
             self.visit_expression(&col.expr);
        }

        // Check Source/Joins
        self.visit_expression(&crate::ast::Expression::Identifier(dql.source.table.clone()));
        for join in &dql.joins {
             self.visit_expression(&crate::ast::Expression::Identifier(join.table.clone()));
             if let Some(cond) = &join.condition {
                  self.visit_expression(cond);
             }
        }
        
        // Where
        if let Some(w) = &dql.where_clause {
            self.visit_expression(&w.condition);
        }
        
         // Group By
        if let Some(g) = &dql.group_by {
            g.expressions.iter().for_each(|e| self.check_not_genitive_plural(e));
            for expr in &g.expressions {
                self.visit_expression(expr);
            }
        }

        // Having: its comparisons agree like a WHERE's (an aggregate takes
        // the gender of the noun naming it: गणना … इत्यस्याः अधिका).
        if let Some(h) = &dql.having {
            self.visit_expression(&h.condition);
        }

        // Order By
        if let Some(o) = &dql.order_by {
            for item in &o.items {
                self.visit_expression(&item.expr);
            }
        }

        self.current_table_context = previous_context;
    }
    
    /// `ऋणात्मक` agrees with its column in gender and case. The column's word
    /// says which cases it can be, since `मूल्यम्` is nominative or accusative.
    fn check_negative_agreement(&mut self, word: &crate::ast::Identifier, value: &Expression) {
        let (noun, table) = match value {
            Expression::Identifier(id) => (id, None),
            Expression::FieldAccess { object, field } => match &**object {
                Expression::Identifier(o) => (field, Some(if o.pratipadika.is_empty() { o.text.clone() } else { o.pratipadika.clone() })),
                _ => return,
            },
            _ => return,
        };
        let (Some(gender), _) = self.get_expr_gender_and_vachana(value) else { return };
        let contexts = [table.as_deref(), self.current_scope.as_deref(), self.current_table_context.as_deref(), None];
        use Vibhakti::*;
        let expected: Vec<String> = [Prathama, Dvitiya, Tritiya, Chaturthi, Panchami, Shashthi, Saptami]
            .into_iter()
            .filter(|&case| {
                let form = contexts.iter().find_map(|c| self.schema.column_case_form(&noun.text, *c, case));
                form.as_deref() == Some(noun.text.as_str())
            })
            .filter_map(|case| crate::morphology::negative_word(gender, Vachana::Eka, case))
            .collect();
        if !expected.is_empty() && !expected.contains(&word.text) {
            self.warnings.push(format!(
                "Grammar Warning: `{}` agrees with `{}`: write `{}`",
                word.text, noun.text, expected[0]
            ));
        }
    }

    fn get_expr_gender_and_vachana(&self, expr: &Expression) -> (Option<Linga>, Option<Vachana>) {
        match expr {
            Expression::Identifier(id) => {
                let mut gender = None;
                let mut vachana = None;
                if let Some(scope) = self.current_scope.as_deref() {
                    gender = self.schema.get_column_gender(&id.text, Some(scope));
                    vachana = self.schema.get_column_vachana(&id.text, Some(scope));
                }
                if gender.is_none() {
                    if let Some(context) = self.current_table_context.as_deref() {
                        gender = self.schema.get_column_gender(&id.text, Some(context));
                        vachana = self.schema.get_column_vachana(&id.text, Some(context));
                    }
                }
                if gender.is_none() {
                    gender = self.schema.get_column_gender(&id.text, None);
                    vachana = self.schema.get_column_vachana(&id.text, None);
                }
                (gender, vachana)
            }
            Expression::FieldAccess { object, field } => {
                let table_context = if let Expression::Identifier(obj_id) = &**object {
                    let pratipadika = if obj_id.pratipadika.is_empty() {
                        &obj_id.text
                    } else {
                        &obj_id.pratipadika
                    };
                    Some(pratipadika.as_str())
                } else {
                    None
                };
                (
                    self.schema.get_column_gender(&field.text, table_context),
                    self.schema.get_column_vachana(&field.text, table_context),
                )
            }
            // An aggregate or function is named by a noun, and has that
            // noun's gender: योगः (SUM) is masculine, गणना (COUNT) feminine,
            // माध्यम् / महत्तमम् / लघुतमम् neuter - so `मूल्यस्य योगः … अधिकः`.
            Expression::Aggregate { func, .. } => {
                let gender = match func {
                    crate::ast::AggFunc::Sum => Linga::Masculine,
                    crate::ast::AggFunc::Count => Linga::Feminine,
                    crate::ast::AggFunc::Avg | crate::ast::AggFunc::Max | crate::ast::AggFunc::Min => Linga::Neuter,
                };
                (Some(gender), Some(Vachana::Eka))
            }
            // `ऋणात्मक` is a prefix, and leaves its value's gender and number.
            Expression::FunctionCall { name, args, .. } if name == "ऋणात्मक" && args.len() == 1 => {
                self.get_expr_gender_and_vachana(&args[0])
            }
            Expression::FunctionCall { name, args, .. } => {
                let gender = match name.trim() {
                    "महावर्णरूपम्" | "UPPER" | "लघुवर्णरूपम्" | "LOWER" | "संयोजनम्" | "CONCAT" | "अवशिष्टम्" | "MOD" | "AVG" | "MAX" | "MIN" => Some(Linga::Neuter),
                    "खण्डः" | "SUBSTR" | "विकल्पः" | "COALESCE" | "आयामः" | "आयामाः" | "LENGTH" | "SUM" | "NTILE" => {
                        Some(Linga::Masculine)
                    }
                    "COUNT" | "RANK" | "DENSE_RANK" | "ROW_NUMBER" => Some(Linga::Feminine),
                    // Date parts: वर्षम्, दिनम्, क्षणम् neuter; मासः, निमेषः,
                    // वारः masculine; होरा feminine.
                    "YEAR" | "DAY" | "SECOND" => Some(Linga::Neuter),
                    "MONTH" | "MINUTE" | "DAYNAME" => Some(Linga::Masculine),
                    "HOUR" => Some(Linga::Feminine),
                    // The previous / next row's value has the column's gender.
                    "LAG" | "LEAD" => return args.first().map(|a| self.get_expr_gender_and_vachana(a)).unwrap_or((None, None)),
                    // Any other function word (a declared binding or pure UDF)
                    // by its nominative ending; unknown otherwise, never guessed.
                    other if other.ends_with("म्") => Some(Linga::Neuter),
                    other if other.ends_with('ः') => Some(Linga::Masculine),
                    other if other.ends_with('ा') || other.ends_with('ी') => Some(Linga::Feminine),
                    _ => None,
                };
                (gender, Some(Vachana::Eka))
            }
            // Arithmetic is named by its result noun: योगः masculine,
            // गुणनम् / अवशिष्टम् / लब्धम् neuter.
            Expression::BinaryOp { op, .. } => match op {
                crate::ast::BinaryOperator::Add => (Some(Linga::Masculine), Some(Vachana::Eka)),
                crate::ast::BinaryOperator::Multiply
                | crate::ast::BinaryOperator::Subtract
                | crate::ast::BinaryOperator::Divide => (Some(Linga::Neuter), Some(Vachana::Eka)),
                _ => (None, None),
            },
            Expression::Literal(crate::ast::Literal::Number(_)) => (Some(Linga::Neuter), Some(Vachana::Eka)),
            Expression::Literal(crate::ast::Literal::String(_)) => (Some(Linga::Neuter), Some(Vachana::Eka)),
            _ => (None, None),
        }
    }

    fn get_expr_sql_type(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier(id) => {
                if let Some(scope) = self.current_scope.as_deref() {
                    if let Some((_, sql_type, _)) = self.schema.resolve_column(&id.text, Some(scope)) {
                        return Some(sql_type.to_string());
                    }
                }
                if let Some(context) = self.current_table_context.as_deref() {
                    if let Some((_, sql_type, _)) = self.schema.resolve_column(&id.text, Some(context)) {
                        return Some(sql_type.to_string());
                    }
                }
                self.schema
                    .resolve_column(&id.text, None)
                    .map(|(_, sql_type, _)| sql_type.to_string())
            }
            Expression::FieldAccess { object, field } => {
                let table_context = if let Expression::Identifier(obj_id) = &**object {
                    let pratipadika = if obj_id.pratipadika.is_empty() {
                        &obj_id.text
                    } else {
                        &obj_id.pratipadika
                    };
                    Some(pratipadika.as_str())
                } else {
                    None
                };
                self.schema
                    .resolve_column(&field.text, table_context)
                    .map(|(_, sql_type, _)| sql_type.to_string())
            }
            _ => None,
        }
    }

    fn count_in_list_items(expr: &Expression) -> usize {
        match expr {
            Expression::LogicalOp { left, op: crate::ast::LogicalOperator::And, right } => {
                Self::count_in_list_items(left) + Self::count_in_list_items(right)
            }
            _ => 1,
        }
    }

    fn pronoun_matches(p: &str, gender: Linga, vachana: Vachana) -> bool {
        match gender {
            Linga::Masculine | Linga::Neuter => match vachana {
                Vachana::Eka => p == "इत्यनेन" || p == "इत्यस्मात्",
                Vachana::Dvi => p == "इत्याभ्याम्",
                Vachana::Bahu => p == "इत्येभिः" || p == "इत्येभ्यः",
            },
            Linga::Feminine => match vachana {
                Vachana::Eka => p == "इत्यनया" || p == "इत्यस्याः",
                Vachana::Dvi => p == "इत्याभ्याम्",
                Vachana::Bahu => p == "इत्याभिः" || p == "इत्याभ्यः",
            },
        }
    }
    
    /// In a grouped query (`… इति समूहेन`, or any aggregate), each value read
    /// once per group (the select list, the group filter, the order, a
    /// window over the groups) is an aggregate, a constant, or made of
    /// grouped columns: SQLite returned one arbitrary row's value per group,
    /// PostgreSQL refuses it.
    fn check_grouping(&mut self, dql: &DqlStatement) {
        let grouped: &[Expression] = dql.group_by.as_ref().map_or(&[], |g| g.expressions.as_slice());
        let per_group: Vec<&Expression> = dql.select.columns.iter().map(|c| &c.expr)
            .chain(dql.having.iter().map(|h| &*h.condition))
            .chain(dql.order_by.iter().flat_map(|o| o.items.iter().map(|i| &*i.expr)))
            .collect();
        let aggregated = per_group.iter().any(|e| Self::has_group_aggregate(e));
        if grouped.is_empty() && !aggregated {
            return;
        }
        let mut keys: Vec<String> = dql.select.columns.iter().filter_map(|c| c.alias.clone()).collect();
        let mut reads = Vec::new();
        for g in grouped {
            self.column_reads(g, &mut reads);
        }
        keys.extend(reads.drain(..).flat_map(|(_, names)| names));
        for e in &per_group {
            self.column_reads(e, &mut reads);
        }
        let mut reported: Vec<String> = Vec::new();
        for (word, names) in reads {
            if names.iter().any(|n| keys.contains(n)) || reported.contains(&word) {
                continue;
            }
            self.warnings.push(format!(
                "Grammar Warning: `{}` is neither grouped (`… इति समूहेन`) nor aggregated, so each group has no one value of it: add it to the grouping or take an aggregate of it",
                word
            ));
            reported.push(word);
        }
    }

    /// An aggregate is computed per group: after the row filter and the
    /// joins, and of a group's rows, never of another aggregate.
    fn check_aggregate_placement(&mut self, dql: &DqlStatement) {
        if dql.where_clause.as_ref().map_or(false, |w| Self::has_group_aggregate(&w.condition)) {
            self.warnings.push(Self::AGGREGATE_IN_ROW_FILTER.to_string());
        }
        if dql.group_by.as_ref().map_or(false, |g| g.expressions.iter().any(Self::has_group_aggregate)) {
            self.warnings.push("Grammar Warning: an aggregate cannot be a grouping key (`… इति समूहेन`): the groups exist before it is computed".to_string());
        }
        if dql.joins.iter().any(|j| j.condition.as_ref().map_or(false, |c| Self::has_group_aggregate(c))) {
            self.warnings.push("Grammar Warning: an aggregate cannot be in a join condition (`… इति मेलनेन`): rows are joined before they are grouped; compute it in a CTE and join that".to_string());
        }
        let nested = dql.select.columns.iter().map(|c| &c.expr)
            .chain(dql.having.iter().map(|h| &*h.condition))
            .chain(dql.order_by.iter().flat_map(|o| o.items.iter().map(|i| &*i.expr)))
            .any(Self::has_nested_aggregate);
        if nested {
            self.warnings.push("Grammar Warning: an aggregate is taken of a group's rows, not of another aggregate; compute the inner one per group in a CTE and aggregate its column".to_string());
        }
    }

    const AGGREGATE_IN_ROW_FILTER: &'static str = "Grammar Warning: an aggregate is computed per group, after the row filter (`… इति नियमेन`) has run, so it cannot be in it; a condition on an aggregate is the group filter `… इति समूहनियमेन`";

    /// The expressions `expr` is made of within its own query (a subquery's
    /// are its own); a window's keys count, as they are read per row.
    fn sub_expressions(expr: &Expression) -> Vec<&Expression> {
        match expr {
            Expression::BinaryOp { left, right, .. }
            | Expression::LogicalOp { left, right, .. }
            | Expression::SanskritComparison { left, right, .. } => vec![&**left, &**right],
            Expression::Aggregate { expr, .. } | Expression::Cast { expr, .. } => vec![&**expr],
            Expression::Between { expr, low, high, .. } => vec![&**expr, &**low, &**high],
            Expression::NamedArg { value, .. } => vec![&**value],
            Expression::Case { cases } => cases.iter()
                .flat_map(|c| c.condition.iter().map(|e| &**e).chain(std::iter::once(&*c.result)))
                .collect(),
            Expression::FunctionCall { args, window, .. } => args.iter()
                .chain(window.iter().flat_map(|w| w.partition_by.iter().chain(w.order_by.iter().map(|o| &*o.expr))))
                .collect(),
            Expression::Identifier(_) | Expression::FieldAccess { .. } | Expression::Literal(_)
            | Expression::Subquery(_) | Expression::SetOp(_) | Expression::Exists { .. } | Expression::Wildcard => vec![],
        }
    }

    /// Whether `expr` holds an aggregate of its own query's groups (a
    /// windowed function is computed per row; a subquery groups its own).
    fn has_group_aggregate(expr: &Expression) -> bool {
        matches!(expr, Expression::Aggregate { .. })
            || Self::sub_expressions(expr).into_iter().any(Self::has_group_aggregate)
    }

    fn has_nested_aggregate(expr: &Expression) -> bool {
        match expr {
            Expression::Aggregate { expr, .. } => Self::has_group_aggregate(expr),
            _ => Self::sub_expressions(expr).into_iter().any(Self::has_nested_aggregate),
        }
    }

    /// The column words `expr` reads outside an aggregate, each with the
    /// names it answers to (its form, its stem, its SQL name). A word that
    /// is not a column (a parameter, a pronoun) is not read.
    fn column_reads(&self, expr: &Expression, out: &mut Vec<(String, Vec<String>)>) {
        match expr {
            Expression::Aggregate { .. } => {}
            Expression::Identifier(id) => {
                if let Some(names) = self.column_names(id, None) {
                    out.push((id.text.clone(), names));
                }
            }
            Expression::FieldAccess { object, field } => {
                let table = match &**object {
                    Expression::Identifier(t) => Some(t.text.as_str()),
                    _ => None,
                };
                if let Some(names) = self.column_names(field, table) {
                    out.push((field.text.clone(), names));
                }
            }
            Expression::Subquery(sub) | Expression::Exists { subquery: sub, .. } => self.outer_reads(sub, out),
            _ => {
                for e in Self::sub_expressions(expr) {
                    self.column_reads(e, out);
                }
            }
        }
    }

    /// The outer query's columns a correlated subquery reads: a qualified
    /// column of a table the subquery does not read itself.
    fn outer_reads(&self, sub: &DqlStatement, out: &mut Vec<(String, Vec<String>)>) {
        fn qualified<'e>(e: &'e Expression, found: &mut Vec<(&'e crate::ast::Identifier, &'e crate::ast::Identifier)>) {
            if let Expression::FieldAccess { object, field } = e {
                if let Expression::Identifier(t) = &**object {
                    found.push((t, field));
                }
            }
            for s in Validator::sub_expressions(e) {
                qualified(s, found);
            }
        }
        let own: Vec<&str> = std::iter::once(&sub.source.table)
            .chain(sub.joins.iter().map(|j| &j.table))
            .map(|t| t.pratipadika.as_str())
            .collect();
        let mut found = Vec::new();
        let exprs = sub.select.columns.iter().map(|c| &c.expr)
            .chain(sub.where_clause.iter().map(|w| &*w.condition))
            .chain(sub.having.iter().map(|h| &*h.condition))
            .chain(sub.joins.iter().filter_map(|j| j.condition.as_deref()));
        for e in exprs {
            qualified(e, &mut found);
        }
        for (table, field) in found {
            if own.contains(&table.pratipadika.as_str()) {
                continue;
            }
            if let Some(names) = self.column_names(field, Some(&table.text)) {
                out.push((field.text.clone(), names));
            }
        }
    }

    /// Distinct rows (`पुनरुक्तिम् विना`) are ordered by what they hold: an
    /// order key outside the select list has no one value per distinct row
    /// (PostgreSQL refuses it; SQLite ordered by an arbitrary row's).
    fn check_distinct_order(&mut self, dql: &DqlStatement) {
        let Some(order) = dql.order_by.as_ref().filter(|_| dql.select.distinct) else { return };
        let mut selected: Vec<String> = dql.select.columns.iter().filter_map(|c| c.alias.clone()).collect();
        let mut reads = Vec::new();
        for c in &dql.select.columns {
            self.column_reads(&c.expr, &mut reads);
        }
        selected.extend(reads.drain(..).flat_map(|(_, names)| names));
        for item in &order.items {
            self.column_reads(&item.expr, &mut reads);
        }
        if let Some((word, _)) = reads.iter().find(|(_, names)| !names.iter().any(|n| selected.contains(n))) {
            self.warnings.push(format!(
                "Grammar Warning: distinct rows (`पुनरुक्तिम् विना`) are ordered by the values they hold; `{}` is not selected, so it has no one value per row: select it or order by a selected value",
                word
            ));
        }
    }

    fn column_names(&self, id: &crate::ast::Identifier, table: Option<&str>) -> Option<Vec<String>> {
        let context = table.or(self.current_table_context.as_deref());
        let sql = self.schema.resolve_column(&id.text, context).map(|(c, _, _)| c.to_string());
        if sql.is_none() && id.symbol_type != crate::types::SymbolType::Column {
            return None;
        }
        Some([Some(id.text.clone()), Some(id.pratipadika.clone()), sql].into_iter().flatten().filter(|n| !n.is_empty()).collect())
    }

    /// A frame runs forward from its start to its end, and an offset counts
    /// rows: a whole number, not a column or text.
    fn check_frame(&mut self, f: &crate::ast::WindowFrame) {
        use crate::ast::FrameBound::*;
        fn rank(b: &crate::ast::FrameBound) -> u8 {
            match b { UnboundedPreceding => 0, Preceding(_) => 1, CurrentRow => 2, Following(_) => 3, UnboundedFollowing => 4 }
        }
        for b in std::iter::once(&f.start).chain(f.end.iter()) {
            if let Preceding(e) | Following(e) = b {
                let whole = matches!(&**e, Expression::Literal(crate::ast::Literal::Number(n)) if !n.contains('.'));
                if !whole {
                    self.warnings.push("Grammar Warning: a frame's offset (`… पूर्ववर्तितः` / `… अनुवर्तितः`) is a whole number of rows".to_string());
                }
            }
        }
        let bad_start = matches!(f.start, UnboundedFollowing);
        let bad_end = matches!(f.end, Some(UnboundedPreceding));
        let reversed = f.end.as_ref().map_or(false, |e| rank(e) < rank(&f.start)
            || (matches!(f.start, Following(_)) && !matches!(e, Following(_) | UnboundedFollowing)));
        if bad_start || bad_end || reversed {
            self.warnings.push("Grammar Warning: a frame runs from an earlier row to a later one (`… पूर्ववर्तितः आरभ्य वर्तमानपङ्क्तिपर्यन्तम्`); this one starts after it ends, or starts after every row".to_string());
        }
    }

    /// The tables and CTEs a statement reads, by stem.
    fn statement_tables(stmt: &Statement, out: &mut Vec<String>) {
        fn dql_tables(dql: &DqlStatement, out: &mut Vec<String>) {
            out.push(dql.source.table.pratipadika.clone());
            out.extend(dql.joins.iter().map(|j| j.table.pratipadika.clone()));
        }
        match stmt {
            Statement::Dql(dql) => dql_tables(dql, out),
            Statement::SetOp(op) => {
                for q in &op.queries {
                    if let Expression::Subquery(dql) = q { dql_tables(dql, out); }
                    if let Expression::Identifier(id) = q { out.push(id.pratipadika.clone()); }
                }
            }
            Statement::Dml(crate::ast::DmlStatement::InsertSelect { query, .. }) => dql_tables(query, out),
            _ => {}
        }
    }

    /// Clauses that compile to SQL no database accepts.
    fn check_dql_shape(&mut self, dql: &DqlStatement) {
        // A window function is computed after WHERE, GROUP BY and HAVING.
        let mut windowed = dql.where_clause.as_ref().map_or(false, |w| Self::has_window(&w.condition));
        windowed |= dql.having.as_ref().map_or(false, |h| Self::has_window(&h.condition));
        windowed |= dql.group_by.as_ref().map_or(false, |g| g.expressions.iter().any(Self::has_window));
        if windowed {
            self.warnings.push("Grammar Warning: a window function (`… यत्र …`) is computed after the filter, the grouping and the group filter, so it cannot be used in them; compute it in a CTE and filter that".to_string());
        }
        // LIMIT and OFFSET count rows: a whole number.
        if let Some(limit) = &dql.limit {
            for (what, e) in std::iter::once(("मित्या", &limit.count)).chain(limit.offset.iter().map(|o| ("अपसारेण", o))) {
                let whole = matches!(&**e, Expression::Literal(crate::ast::Literal::Number(n)) if !n.contains('.') && !n.starts_with('-'));
                if !whole {
                    self.warnings.push(format!("Grammar Warning: `{}` takes a whole number of rows", what));
                }
            }
        }
        // A subquery used as one value has one column.
        for col in &dql.select.columns {
            self.check_one_column(&col.expr);
        }
    }

    fn check_one_column(&mut self, expr: &Expression) {
        if let Expression::Subquery(sub) = expr {
            let wildcard = sub.select.columns.iter().any(|c| matches!(c.expr, Expression::Wildcard));
            if sub.select.columns.len() != 1 || wildcard {
                self.warnings.push(format!(
                    "Grammar Warning: a subquery used as a value, or as the list of an IN, returns one column; this one returns {}",
                    if wildcard { "every column".to_string() } else { sub.select.columns.len().to_string() }
                ));
            }
        }
    }

    fn has_window(expr: &Expression) -> bool {
        match expr {
            Expression::FunctionCall { window: Some(_), .. } => true,
            Expression::FunctionCall { args, .. } => args.iter().any(Self::has_window),
            Expression::BinaryOp { left, right, .. } | Expression::LogicalOp { left, right, .. }
            | Expression::SanskritComparison { left, right, .. } => Self::has_window(left) || Self::has_window(right),
            Expression::Between { expr, low, high, .. } => Self::has_window(expr) || Self::has_window(low) || Self::has_window(high),
            Expression::Aggregate { expr, .. } | Expression::Cast { expr, .. } => Self::has_window(expr),
            _ => false,
        }
    }

    /// The `<column> <value>` pairs of an INSERT or UPDATE list.
    fn assigned_columns(expr: &Expression) -> Vec<String> {
        match expr {
            Expression::LogicalOp { left, op: crate::ast::LogicalOperator::And, right } => {
                let mut v = Self::assigned_columns(left);
                v.extend(Self::assigned_columns(right));
                v
            }
            Expression::BinaryOp { left, .. } | Expression::SanskritComparison { left, .. } => match &**left {
                Expression::Identifier(id) => vec![id.text.clone()],
                Expression::FieldAccess { field, .. } => vec![field.text.clone()],
                _ => vec![],
            },
            _ => vec![],
        }
    }

    fn check_dml_shape(&mut self, dml: &crate::ast::DmlStatement) {
        use crate::ast::DmlStatement::*;
        let (columns, what) = match dml {
            Insert { values, .. } => (Self::assigned_columns(values), "inserted"),
            Update { set_clause, .. } => (Self::assigned_columns(&set_clause.assignments), "changed"),
            InsertSelect { columns: Some(cols), query, .. } => {
                let wildcard = query.select.columns.iter().any(|c| matches!(c.expr, Expression::Wildcard));
                if !wildcard && cols.len() != query.select.columns.len() {
                    let n = |k: usize| if k == 1 { "1 column".to_string() } else { format!("{} columns", k) };
                    self.warnings.push(format!(
                        "Grammar Warning: {} named but the query selects {}",
                        if cols.len() == 1 { "1 column is".to_string() } else { format!("{} columns are", cols.len()) },
                        n(query.select.columns.len())
                    ));
                }
                (cols.iter().map(|c| c.text.clone()).collect(), "inserted")
            }
            _ => return,
        };
        let mut seen = std::collections::HashSet::new();
        if let Some(dup) = columns.iter().find(|c| !seen.insert(c.as_str())) {
            self.warnings.push(format!("Grammar Warning: `{}` is {} twice in one statement", dup, what));
        }
    }

    fn check_table_shape(&mut self, columns: &[crate::ast::ColumnDef]) {
        use crate::ast::Constraint;
        let keys = columns.iter().filter(|c| c.constraints.contains(&Constraint::PrimaryKey)).count();
        if keys > 1 {
            self.warnings.push("Grammar Warning: a table has one primary key (`प्राथमिक`); this one marks several columns".to_string());
        }
        let mut physical = std::collections::HashSet::new();
        for col in columns {
            let name = col.sql_name.clone().unwrap_or_else(|| col.name.text.clone());
            if !physical.insert(name.clone()) {
                self.warnings.push(format!("Grammar Warning: two columns are both named `{}`", name));
            }
            let c = &col.constraints;
            if c.contains(&Constraint::AutoIncrement)
                && (!c.contains(&Constraint::PrimaryKey) || col.data_type != crate::ast::DataType::Integer)
            {
                self.warnings.push(format!(
                    "Grammar Warning: `{}`: only an integer primary key counts by itself (`प्राथमिकी क्रमिका … पूर्णाङ्कः`)",
                    col.name.text
                ));
            }
            if c.contains(&Constraint::NotNull) && c.contains(&Constraint::Null) {
                self.warnings.push(format!("Grammar Warning: `{}` is both required and optional", col.name.text));
            }
        }
    }

    /// A name reaches SQL as written. PostgreSQL keeps 63 bytes of a name,
    /// about 21 Devanagari letters, cutting mid-akshara, and two long names
    /// can then collide; so a longer one is an error on both dialects, and
    /// one source compiles on both.
    fn check_name_length(&mut self, name: &str) {
        if name.len() > 63 {
            self.warnings.push(format!(
                "Grammar Warning: `{}` is {} bytes, and PostgreSQL keeps only 63 bytes of a name (about 21 Devanagari letters): use a shorter name",
                name, name.len()
            ));
        }
    }

    fn is_genitive_plural(expr: &Expression) -> bool {
        let id = match expr {
            Expression::Identifier(id) => id,
            Expression::FieldAccess { field, .. } => field,
            _ => return false,
        };
        id.vibhakti_form.vibhakti == crate::types::Vibhakti::Shashthi && id.vibhakti_form.vachana == Vachana::Bahu
    }

    /// A column is read in the singular; its genitive plural is only an
    /// aggregate's argument (`मूल्यानाम् योगः`, "the sum of the prices").
    fn check_not_genitive_plural(&mut self, expr: &Expression) {
        if Self::is_genitive_plural(expr) {
            let word = match expr {
                Expression::Identifier(id) => &id.text,
                Expression::FieldAccess { field, .. } => &field.text,
                _ => return,
            };
            self.warnings.push(format!(
                "Grammar Warning: `{}` is a genitive plural, which is only an aggregate's argument (`… योगः`); here the column is in the singular",
                word
            ));
        }
    }

    fn check_gerund_object(&mut self, operand: &Expression, gerund: &str) {
        let (word, context) = match operand {
            Expression::Identifier(id) => (
                &id.text,
                self.current_scope.clone().or_else(|| self.current_table_context.clone()),
            ),
            Expression::FieldAccess { object, field } => match &**object {
                Expression::Identifier(obj) => (&field.text, Some(if obj.pratipadika.is_empty() { obj.text.clone() } else { obj.pratipadika.clone() })),
                _ => return,
            },
            _ => return,
        };
        if let Some(accusative) = self.schema.column_case_form(word, context.as_deref(), crate::types::Vibhakti::Dvitiya) {
            if &accusative != word {
                self.warnings.push(format!(
                    "Grammar Warning: the object of `{}` is in the accusative: write `{}`, not `{}`",
                    gerund, accusative, word
                ));
            }
        }
    }

    fn visit_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Subquery(dql) => self.visit_dql(dql),
            Expression::FunctionCall { name, args, window, pronoun_token } => {
                for arg in args { self.visit_expression(arg); }
                if name == "ऋणात्मक" {
                    let numeric = args.first().and_then(|a| self.get_expr_sql_type(a)).map_or(true, |t| {
                        let t = t.to_ascii_lowercase();
                        !(t.contains("text") || t.contains("char") || t.contains("date") || t.contains("time"))
                    });
                    if !numeric {
                        self.warnings.push("Grammar Warning: `ऋणात्मक` makes a number negative; this column does not hold numbers".to_string());
                    }
                    if let (Some(word), Some(value)) = (pronoun_token, args.first()) {
                        self.check_negative_agreement(word, value);
                    }
                    return;
                }
                if !matches!(name.as_str(), "SUM" | "COUNT" | "AVG" | "MAX" | "MIN") {
                    args.iter().for_each(|arg| self.check_not_genitive_plural(arg));
                }
                if let Some(w) = window {
                    w.partition_by.iter().for_each(|p| self.check_not_genitive_plural(p));
                    w.order_by.iter().for_each(|o| self.check_not_genitive_plural(&o.expr));
                    for p in &w.partition_by { self.visit_expression(p); }
                    for o in &w.order_by { self.visit_expression(&o.expr); }
                    if let Some(ref f) = w.frame {
                        self.check_frame(f);
                        match &f.start {
                            crate::ast::FrameBound::Preceding(e) | crate::ast::FrameBound::Following(e) => {
                                self.visit_expression(e);
                            }
                            _ => {}
                        }
                        if let Some(ref end) = f.end {
                            match end {
                                crate::ast::FrameBound::Preceding(e) | crate::ast::FrameBound::Following(e) => {
                                    self.visit_expression(e);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // The pronoun of a further operand agrees with the first:
                // `मूल्यस्य २ इत्यनेन शेषः`, `सङ्ख्यायाः २ इत्यनया शेषः`.
                if let (true, Some(p_id)) = (args.len() > 1, pronoun_token) {
                    if let (Some(gender), _) = self.get_expr_gender_and_vachana(&args[0]) {
                        let expected = if gender == Linga::Feminine { "इत्यनया" } else { "इत्यनेन" };
                        if p_id.text != expected {
                            self.warnings.push(format!(
                                "Grammar Warning: the pronoun agrees with the function's first operand, which is {:?}: write `{}`, not `{}`",
                                gender, expected, p_id.text
                            ));
                        }
                    }
                }
                if args.len() == 1 {
                    let (arg_gender, arg_vachana) = self.get_expr_gender_and_vachana(&args[0]);
                    if let Some(gender) = arg_gender {
                        let vachana = arg_vachana.unwrap_or(Vachana::Eka);
                        if let Some(p_id) = pronoun_token {
                            let p_text = &p_id.text;
                            let ok = match gender {
                                Linga::Masculine | Linga::Neuter => match vachana {
                                    Vachana::Eka => p_text == "इत्यस्य",
                                    Vachana::Dvi => p_text == "इत्यनयोः",
                                    Vachana::Bahu => p_text == "इत्येषाम्",
                                },
                                Linga::Feminine => match vachana {
                                    Vachana::Eka => p_text == "इत्यस्याः",
                                    Vachana::Dvi => p_text == "इत्यनयोः",
                                    Vachana::Bahu => p_text == "इत्यासाम्",
                                },
                            };
                            if !ok {
                                self.warnings.push(format!(
                                    "Grammar Warning: Function pronoun gender/number mismatch. Argument has gender {:?} and number {:?}, but you used pronoun '{}'",
                                    gender, vachana, p_text
                                ));
                            }
                        }
                    }
                }
            },
            // `१.२.३` is not a number.
            Expression::Literal(crate::ast::Literal::Number(n)) if n.matches('.').count() > 1 => {
                self.warnings.push(format!("Grammar Warning: `{}` is not a number: it has more than one decimal point", n));
            }
            Expression::SanskritComparison { left, op, right, pronoun, adjective } => {
                self.visit_expression(left);
                self.visit_expression(right);
                self.check_one_column(right);

                let (col_gender, col_vachana) = self.get_expr_gender_and_vachana(left);

                if let Some(gender) = col_gender {
                    let vachana = col_vachana.unwrap_or(Vachana::Eka);
                    // Pronoun check (Tritiya / Panchami / Saptami for IN)
                    if let Some(p_id) = pronoun {
                        let p = &p_id.text;
                        let is_in = matches!(op, crate::ast::BinaryOperator::In | crate::ast::BinaryOperator::NotIn);
                        // The list's pronoun is a locative of इदम् for the
                        // number of values: dual इत्यनयोः, plural इत्येषु,
                        // feminine इत्यासु.
                        let item_count = Self::count_in_list_items(right);
                        let in_pronoun = match item_count {
                            2 => "इत्यनयोः",
                            _ if gender == Linga::Feminine => "इत्यासु",
                            _ => "इत्येषु",
                        };
                        let ok = if is_in {
                            p == in_pronoun
                        } else {
                            let mut matches = Self::pronoun_matches(p, gender, vachana);
                            if !matches && !matches!(**right, Expression::Literal(_)) {
                                let (mut r_gender, r_vachana) = self.get_expr_gender_and_vachana(right);
                                if r_gender.is_none() {
                                    if let Expression::Identifier(r_id) = &**right {
                                        if r_id.text.ends_with("म्") || r_id.text.ends_with('ं') {
                                            r_gender = Some(Linga::Neuter);
                                        } else if r_id.text.ends_with('ः') {
                                            r_gender = Some(Linga::Masculine);
                                        } else if r_id.text.ends_with('ा') {
                                            r_gender = Some(Linga::Feminine);
                                        }
                                    }
                                }
                                if let Some(rg) = r_gender {
                                    let rv = r_vachana.unwrap_or(Vachana::Eka);
                                    matches = Self::pronoun_matches(p, rg, rv);
                                }
                            }
                            matches
                        };
                        if !ok && is_in && item_count > 2 && matches!(p.as_str(), "इत्येषु" | "इत्यासु") {
                            // Right number, wrong gender.
                            self.warnings.push(format!(
                                "Grammar Warning: Pronoun Gender Mismatch. The column is {}, so the list's pronoun is `{}`, not `{}`",
                                if gender == Linga::Feminine { "feminine" } else { "masculine or neuter" }, in_pronoun, p
                            ));
                        } else if !ok && is_in {
                            self.warnings.push(format!(
                                "Grammar Warning: the list has {} values, so its pronoun is the locative `{}`, not `{}`",
                                item_count, in_pronoun, p
                            ));
                        } else if !ok && !p.starts_with("इत्य") {
                            // A column in the instrumental, not the pronoun.
                            self.warnings.push(format!(
                                "Grammar Warning: `{}` is a column in the instrumental; a compared column is named with the pronoun: `पुस्तकस्य प्रकाशनवर्षम् इत्यनेन समम्`",
                                p
                            ));
                        } else if !ok {
                            self.warnings.push(format!(
                                "Grammar Warning: Pronoun Gender Mismatch. `{}` must agree with the column ({}){}",
                                p, gender_number(gender, vachana),
                                expected_pronoun(p, gender, vachana).map(|e| format!(": write `{}`", e)).unwrap_or_default()
                            ));
                        }
                    }

                    // Adjective check
                    if let Some(adj_id) = adjective {
                        let adj = &adj_id.text;
                        // सदृश takes ङीप् in the feminine (सदृशी, 4.1.15),
                        // the others टाप् (समा, अधिका, न्यूना, रिक्ता).
                        let ok = match adj.as_str() {
                            "अनुरूपेण" => true,
                            _ => match gender {
                                Linga::Masculine => match vachana {
                                    Vachana::Eka => matches!(adj.as_str(), "समः" | "अधिकः" | "न्यूनः" | "रिक्तः" | "सदृशः"),
                                    Vachana::Dvi => matches!(adj.as_str(), "समौ" | "अधिकौ" | "न्यूनौ" | "रिक्तौ"),
                                    Vachana::Bahu => matches!(adj.as_str(), "समाः" | "अधिकाः" | "न्यूनाः" | "रिक्ताः"),
                                },
                                Linga::Feminine => match vachana {
                                    Vachana::Eka => matches!(adj.as_str(), "समा" | "अधिका" | "न्यूना" | "रिक्ता" | "सदृशी"),
                                    Vachana::Dvi => matches!(adj.as_str(), "समे" | "अधिके" | "न्यूने" | "रिक्ते"),
                                    Vachana::Bahu => matches!(adj.as_str(), "समाः" | "अधिकाः" | "न्यूनाः" | "रिक्ताः"),
                                },
                                Linga::Neuter => match vachana {
                                    Vachana::Eka => matches!(adj.as_str(), "समम्" | "अधिकम्" | "न्यूनम्" | "रिक्तम्" | "सदृशम्"),
                                    Vachana::Dvi => matches!(adj.as_str(), "समे" | "अधिके" | "न्यूने" | "रिक्ते"),
                                    Vachana::Bahu => matches!(adj.as_str(), "समानि" | "अधिकानि" | "न्यूनानि" | "रिक्तानि"),
                                },
                            }
                        };
                        if !ok {
                            self.warnings.push(format!(
                                "Grammar Warning: Adjective Gender Mismatch. `{}` must agree with the column ({}){}",
                                adj, gender_number(gender, vachana),
                                expected_adjective(adj, gender, vachana).map(|e| format!(": write `{}`", e)).unwrap_or_default()
                            ));
                        }
                    }
                }
            },
            Expression::BinaryOp { left, right, op } => {
                self.visit_expression(left);
                self.visit_expression(right);
                // A date is no number: `sale_date + 7` was 2030 on SQLite, which
                // read the text as its year, and a week later on PostgreSQL.
                for operand in [&**left, &**right] {
                    let is_date = self.get_expr_sql_type(operand).is_some_and(|t| {
                        let t = t.to_ascii_lowercase();
                        t.contains("date") || t.contains("time")
                    });
                    if is_date {
                        self.warnings.push("Grammar Warning: arithmetic on a date is not available, since SQLite and PostgreSQL read it differently; `अन्तरम्` gives the days from a date to today, and `वर्षम्`, `मासः` and `दिनम्` its parts".to_string());
                    }
                }
                // The object of a gerund is in the accusative: what is taken
                // away (`… अपकर्षम् विशोध्य`) and what is divided (`भण्डारम्
                // २ विभज्य`). A neuter's accusative is its nominative, so the
                // word is compared with the column's own accusative.
                match op {
                    crate::ast::BinaryOperator::Subtract => self.check_gerund_object(right, "विशोध्य"),
                    crate::ast::BinaryOperator::Divide => self.check_gerund_object(left, "विभज्य"),
                    _ => {}
                }
            },
            Expression::LogicalOp { left, right, .. } => {
                self.visit_expression(left);
                self.visit_expression(right);
            },
            Expression::Case { cases } => {
                 for case in cases {
                     if let Some(c) = &case.condition { self.visit_expression(c); }
                     self.visit_expression(&case.result);
                 }
            },
            Expression::Aggregate { expr, func, distinct_token, pronoun_token, .. } => {
                // `सर्वम् इत्यस्य गणना` reads as counting the word "everything";
                // COUNT(*) counts rows (expert review F5).
                if matches!(**expr, Expression::Wildcard) && pronoun_token.is_some() {
                    self.warnings.push("Grammar Warning: COUNT(*) counts rows; write `पङ्क्तीनाम् गणना`, not `सर्वम् इत्यस्य गणना`".to_string());
                }
                // A sum or an average is of numbers: SUM over text was 0 on
                // SQLite and an error on PostgreSQL.
                if matches!(func, crate::ast::AggFunc::Sum | crate::ast::AggFunc::Avg) {
                    let kind = match self.get_expr_sql_type(expr).as_deref() {
                        Some("text") => Some("text"),
                        Some("date") => Some("a date"),
                        Some("boolean") => Some("true or false"),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        let word = if matches!(func, crate::ast::AggFunc::Sum) { "योगः" } else { "माध्यम्" };
                        self.warnings.push(format!("Grammar Warning: `{}` is of numbers, and this column holds {}; SQLite would read it as 0 and PostgreSQL refuses it", word, kind));
                    }
                }
                self.visit_expression(expr);
                let (op_gender, op_vachana) = self.get_expr_gender_and_vachana(expr);
                if let Some(gender) = op_gender {
                    let vachana = op_vachana.unwrap_or(Vachana::Eka);
                    // विशिष्ट- agrees with the word it qualifies, which may be
                    // the genitive plural: `विशिष्टानाम् भाषाणाम् गणना`.
                    let word_vachana = if Self::is_genitive_plural(expr) { Vachana::Bahu } else { vachana };
                    if let Some(d_id) = distinct_token {
                        let d_text = &d_id.text;
                        let ok = match gender {
                            Linga::Masculine => match word_vachana {
                                Vachana::Eka => d_text == "विशिष्टस्य",
                                Vachana::Dvi => d_text == "विशिष्टयोः",
                                Vachana::Bahu => d_text == "विशिष्टानाम्",
                            },
                            Linga::Feminine => match word_vachana {
                                Vachana::Eka => d_text == "विशिष्टायाः",
                                Vachana::Dvi => d_text == "विशिष्टयोः",
                                Vachana::Bahu => d_text == "विशिष्टानाम्",
                            },
                            Linga::Neuter => match word_vachana {
                                Vachana::Eka => d_text == "विशिष्टस्य",
                                Vachana::Dvi => d_text == "विशिष्टयोः",
                                Vachana::Bahu => d_text == "विशिष्टानाम्",
                            },
                        };
                        if !ok {
                            self.warnings.push(format!(
                                "Grammar Warning: Distinct modifier gender/number mismatch. Column has gender {:?} and number {:?}, but you used '{}'",
                                gender, word_vachana, d_text
                            ));
                        }
                    }
                    if let Some(p_id) = pronoun_token {
                        let p_text = &p_id.text;
                        let ok = match gender {
                            Linga::Masculine | Linga::Neuter => match vachana {
                                Vachana::Eka => p_text == "इत्यस्य",
                                Vachana::Dvi => p_text == "इत्यनयोः",
                                Vachana::Bahu => p_text == "इत्येषाम्",
                            },
                            Linga::Feminine => match vachana {
                                Vachana::Eka => p_text == "इत्यस्याः",
                                Vachana::Dvi => p_text == "इत्यनयोः",
                                Vachana::Bahu => p_text == "इत्यासाम्",
                            },
                        };
                        if !ok {
                            self.warnings.push(format!(
                                "Grammar Warning: Genitive pronoun gender/number mismatch. Column has gender {:?} and number {:?}, but you used pronoun '{}'",
                                gender, vachana, p_text
                            ));
                        }
                    }
                }
            },
            Expression::Cast { expr, .. } => self.visit_expression(expr),
            Expression::Between { expr, low, high, .. } => {
                self.visit_expression(expr);
                self.visit_expression(low);
                self.visit_expression(high);
            }
            Expression::Exists { subquery, .. } => {
                self.visit_dql(subquery);
            }
            Expression::FieldAccess { object, field: _ } => {
                self.visit_expression(object);
                if let Expression::Identifier(obj_id) = &**object {
                    let table_name = &obj_id.text;
                    let table_pratipadika = if obj_id.pratipadika.is_empty() {
                        table_name
                    } else {
                        &obj_id.pratipadika
                    };
                    
                    if let Some(table_info) = self.schema.tables.get(table_pratipadika) {
                        let actual_gender = table_info.gender;
                        let vachana = obj_id.vibhakti_form.vachana;
                        // The table's own genitive, from its declared stem;
                        // a stem the morphology cannot decline is not judged.
                        let genitive = crate::morphology::MorphologyEngine::new()
                            .and_then(|mut m| m.decline_with_gender(&table_info.pratipadika, actual_gender, vachana))
                            .ok()
                            .and_then(|t| t.forms.get(&Vibhakti::Shashthi).cloned());
                        let ok = genitive.map_or(true, |g| g == *table_name);
                        if !ok {
                            self.warnings.push(format!(
                                "Grammar Warning: Genitive table modifier gender/number mismatch. Table '{}' has actual gender {:?}, but you used '{}'",
                                table_pratipadika, actual_gender, table_name
                            ));
                        }
                    }
                }
            }
            Expression::SetOp(set_op) => {
                self.visit_set_op(set_op);
            }
            Expression::NamedArg { value, .. } => {
                self.visit_expression(value);
            }
            _ => {}
        }
    }

}

/// "feminine singular", for a message.
fn gender_number(gender: Linga, vachana: Vachana) -> String {
    let g = match gender { Linga::Masculine => "masculine", Linga::Feminine => "feminine", Linga::Neuter => "neuter" };
    let v = match vachana { Vachana::Eka => "singular", Vachana::Dvi => "dual", Vachana::Bahu => "plural" };
    format!("{} {}", g, v)
}

/// The form of a comparison adjective's family (सम, अधिक, न्यून, रिक्त,
/// सदृश) that agrees with `gender` and `vachana`.
fn expected_adjective(adj: &str, gender: Linga, vachana: Vachana) -> Option<String> {
    let family = ["सदृश", "अधिक", "न्यून", "रिक्त", "सम"].into_iter().find(|s| adj.starts_with(s))?;
    let ending = match (gender, vachana) {
        (Linga::Masculine, Vachana::Eka) => "ः",
        (Linga::Masculine, Vachana::Dvi) => "ौ",
        (Linga::Masculine, Vachana::Bahu) => "ाः",
        (Linga::Feminine, Vachana::Eka) => if family == "सदृश" { "ी" } else { "ा" },
        (Linga::Feminine, Vachana::Dvi) => "े",
        (Linga::Feminine, Vachana::Bahu) => "ाः",
        (Linga::Neuter, Vachana::Eka) => "म्",
        (Linga::Neuter, Vachana::Dvi) => "े",
        (Linga::Neuter, Vachana::Bahu) => "ानि",
    };
    Some(format!("{}{}", family, ending))
}

/// The pronoun of the same case as `p` (instrumental `इत्यनेन`… or ablative
/// `इत्यस्मात्`…) that agrees with `gender` and `vachana`.
fn expected_pronoun(p: &str, gender: Linga, vachana: Vachana) -> Option<&'static str> {
    let instrumental = matches!(p, "इत्यनेन" | "इत्यनया" | "इत्येभिः" | "इत्याभिः" | "इत्याभ्याम्");
    // इत्येषाम् / इत्यासाम् / इत्यनयोः are genitives, written here in place
    // of an ablative; the ablative plural is इत्येभ्यः / इत्याभ्यः.
    let ablative = matches!(p, "इत्यस्मात्" | "इत्यस्याः" | "इत्येभ्यः" | "इत्याभ्यः" | "इत्येषाम्" | "इत्यासाम्" | "इत्यनयोः");
    let feminine = gender == Linga::Feminine;
    match (instrumental, ablative, vachana) {
        (true, _, Vachana::Eka) => Some(if feminine { "इत्यनया" } else { "इत्यनेन" }),
        (true, _, Vachana::Dvi) => Some("इत्याभ्याम्"),
        (true, _, Vachana::Bahu) => Some(if feminine { "इत्याभिः" } else { "इत्येभिः" }),
        (_, true, Vachana::Eka) => Some(if feminine { "इत्यस्याः" } else { "इत्यस्मात्" }),
        (_, true, Vachana::Dvi) => Some("इत्याभ्याम्"),
        (_, true, Vachana::Bahu) => Some(if feminine { "इत्याभ्यः" } else { "इत्येभ्यः" }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Vachana, Linga, VibhaktiForm, Vibhakti, SymbolType};
    use crate::ast::{Expression, Identifier, AstNode, DqlStatement, SourceClause, SelectClause, SelectItem, BinaryOperator};
    use crate::schema::{Schema, TableInfo, ColumnInfo};
    use std::collections::HashMap;

    fn make_test_schema() -> Schema {
        let mut tables = HashMap::new();
        
        // Table name is "विभागाः" (Masculine Plural)
        let mut fields = HashMap::new();
        fields.insert("क्रमाङ्क".to_string(), ColumnInfo {
            sql_name: "id".to_string(),
            sql_type: "integer".to_string(),
            gender: Linga::Masculine,
            pratipadika: "क्रमाङ्क".to_string(),
            vachana: Some(Vachana::Eka),
        });
        fields.insert("नाम".to_string(), ColumnInfo {
            sql_name: "name".to_string(),
            sql_type: "text".to_string(),
            gender: Linga::Neuter,
            pratipadika: "नाम".to_string(),
            vachana: Some(Vachana::Eka),
        });
        
        let mut form_map = HashMap::new();
        form_map.insert("विभागाः".to_string(), "विभाग".to_string());
        
        let mut column_form_map = HashMap::new();
        column_form_map.insert("क्रमाङ्कः".to_string(), "क्रमाङ्क".to_string());
        column_form_map.insert("नाम".to_string(), "नाम".to_string());

        tables.insert("विभाग".to_string(), TableInfo {
            sql_name: "sections".to_string(),
            gender: Linga::Masculine,
            number: Vachana::Bahu,
            pratipadika: "विभाग".to_string(),
            fields,
            form_map,
            column_form_map,
            primary_key_sql_name: None,
        });

        Schema {
            dialect: "sqlite".to_string(),
            tables,
            ..Default::default()
        }
    }

    #[test]
    fn test_gender_concordance() {
        let schema = make_test_schema();
        
        // 1. Valid Masculine check: (क्रमाङ्कः ५ इत्यनेन समः अस्ति)
        let left = Expression::Identifier(Identifier {
            text: "क्रमाङ्कः".to_string(),
            vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
            pratipadika: "क्रमाङ्क".to_string(),
            symbol_type: SymbolType::Column,
            span: None,
        });
        let right = Expression::Literal(crate::ast::Literal::Number("5".to_string()));
        
        let pronoun = Some(Identifier {
            text: "इत्यनेन".to_string(),
            vibhakti_form: VibhaktiForm::new(Vibhakti::Tritiya, Vachana::Eka),
            pratipadika: "इति".to_string(),
            symbol_type: SymbolType::Variable,
            span: None,
        });
        
        let adjective = Some(Identifier {
            text: "समः".to_string(),
            vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
            pratipadika: "सम".to_string(),
            symbol_type: SymbolType::Variable,
            span: None,
        });
        
        let valid_masc = Expression::SanskritComparison {
            left: Box::new(left.clone()),
            op: BinaryOperator::Equal,
            right: Box::new(right.clone()),
            pronoun: pronoun.clone(),
            adjective: adjective.clone(),
        };

        let mut validator = Validator::new(&schema);
        validator.current_table_context = Some("विभाग".to_string());
        
        let ast = AstNode::Statement(crate::ast::Statement::Dql(DqlStatement {
            select: SelectClause { columns: vec![SelectItem { expr: left.clone(), alias: None }], distinct: false },
            source: SourceClause {
                derived: None,
                table: Identifier {
                    text: "विभागेभ्यः".to_string(),
                    vibhakti_form: VibhaktiForm::new(Vibhakti::Panchami, Vachana::Bahu),
                    pratipadika: "विभाग".to_string(),
                    symbol_type: SymbolType::Table,
                    span: None,
                },
            },
            joins: vec![],
            where_clause: Some(crate::ast::WhereClause { condition: Box::new(valid_masc) }),
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            verb_token: Some(Identifier {
                text: "आह्रियताम्".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                pratipadika: "आह्रियताम्".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            }),
        }));
        
        let diags = validator.validate(&ast).unwrap();
        assert!(diags.is_empty(), "Expected no diagnostics, but got: {:?}", diags);

        // 2. Invalid Pronoun check: (क्रमाङ्कः ५ इत्यनया समः अस्ति) - mismatch pronoun
        let invalid_pronoun = Some(Identifier {
            text: "इत्यनया".to_string(),
            vibhakti_form: VibhaktiForm::new(Vibhakti::Tritiya, Vachana::Eka),
            pratipadika: "इति".to_string(),
            symbol_type: SymbolType::Variable,
            span: None,
        });
        
        let invalid_p_expr = Expression::SanskritComparison {
            left: Box::new(left.clone()),
            op: BinaryOperator::Equal,
            right: Box::new(right.clone()),
            pronoun: invalid_pronoun,
            adjective: adjective.clone(),
        };
        
        let ast_invalid_p = AstNode::Statement(crate::ast::Statement::Dql(DqlStatement {
            select: SelectClause { columns: vec![SelectItem { expr: left.clone(), alias: None }], distinct: false },
            source: SourceClause {
                derived: None,
                table: Identifier {
                    text: "विभागेभ्यः".to_string(),
                    vibhakti_form: VibhaktiForm::new(Vibhakti::Panchami, Vachana::Bahu),
                    pratipadika: "विभाग".to_string(),
                    symbol_type: SymbolType::Table,
                    span: None,
                },
            },
            joins: vec![],
            where_clause: Some(crate::ast::WhereClause { condition: Box::new(invalid_p_expr) }),
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            verb_token: Some(Identifier {
                text: "आह्रियताम्".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                pratipadika: "आह्रियताम्".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            }),
        }));
        
        let diags = validator.validate(&ast_invalid_p).unwrap();
        assert!(diags.is_empty(), "Expected no diagnostics, but got: {:?}", diags);
        assert_eq!(validator.warnings.len(), 1, "{:?}", validator.warnings);
        assert!(validator.warnings[0].contains("Pronoun Gender Mismatch"));

        // 3. Invalid Adjective check: (क्रमाङ्कः ५ इत्यनेन समा अस्ति) - mismatch adjective
        let invalid_adj = Some(Identifier {
            text: "समा".to_string(),
            vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
            pratipadika: "सम".to_string(),
            symbol_type: SymbolType::Variable,
            span: None,
        });
        
        let invalid_adj_expr = Expression::SanskritComparison {
            left: Box::new(left.clone()),
            op: BinaryOperator::Equal,
            right: Box::new(right.clone()),
            pronoun: pronoun.clone(),
            adjective: invalid_adj,
        };
        
        let ast_invalid_adj = AstNode::Statement(crate::ast::Statement::Dql(DqlStatement {
            select: SelectClause { columns: vec![SelectItem { expr: left.clone(), alias: None }], distinct: false },
            source: SourceClause {
                derived: None,
                table: Identifier {
                    text: "विभागेभ्यः".to_string(),
                    vibhakti_form: VibhaktiForm::new(Vibhakti::Panchami, Vachana::Bahu),
                    pratipadika: "विभाग".to_string(),
                    symbol_type: SymbolType::Table,
                    span: None,
                },
            },
            joins: vec![],
            where_clause: Some(crate::ast::WhereClause { condition: Box::new(invalid_adj_expr) }),
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            verb_token: Some(Identifier {
                text: "आह्रियताम्".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                pratipadika: "आह्रियताम्".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            }),
        }));
        
        let diags = validator.validate(&ast_invalid_adj).unwrap();
        assert!(diags.is_empty(), "Expected no diagnostics, but got: {:?}", diags);
        assert_eq!(validator.warnings.len(), 1, "{:?}", validator.warnings);
        assert!(validator.warnings[0].contains("Adjective Gender Mismatch"));
    }

    #[test]
    fn test_grammar_and_agreement_validation() {
        let schema = make_test_schema();

        // 1. Test Source Table Ablative Plural check
        let mut validator = Validator::new(&schema);
        let invalid_source_ast = AstNode::Statement(crate::ast::Statement::Dql(DqlStatement {
            select: SelectClause { columns: vec![], distinct: false },
            source: SourceClause {
                derived: None,
                table: Identifier {
                    text: "विभागः".to_string(),
                    vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                    pratipadika: "विभाग".to_string(),
                    symbol_type: SymbolType::Table,
                    span: None,
                },
            },
            joins: vec![],
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            verb_token: Some(Identifier {
                text: "आह्रियताम्".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                pratipadika: "आह्रियताम्".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            }),
        }));
        validator.validate(&invalid_source_ast).unwrap();
        assert!(validator.warnings.iter().any(|w| w.contains("FROM source table must be in the Ablative Plural form")));

        // 2. Test Subject-Verb Agreement check
        let mut validator = Validator::new(&schema);
        let invalid_verb_ast = AstNode::Statement(crate::ast::Statement::Dql(DqlStatement {
            select: SelectClause {
                columns: vec![
                    crate::ast::SelectItem {
                        expr: Expression::Identifier(Identifier {
                            text: "नाम".to_string(),
                            vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                            pratipadika: "नाम".to_string(),
                            symbol_type: SymbolType::Column,
                            span: None,
                        }),
                        alias: None,
                    }
                ],
                distinct: false,
            },
            source: SourceClause {
                derived: None,
                table: Identifier {
                    text: "विभागेभ्यः".to_string(),
                    vibhakti_form: VibhaktiForm::new(Vibhakti::Panchami, Vachana::Bahu),
                    pratipadika: "विभाग".to_string(),
                    symbol_type: SymbolType::Table,
                    span: None,
                },
            },
            joins: vec![],
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            verb_token: Some(Identifier {
                text: "आह्रियेताम्".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                pratipadika: "आह्रियताम्".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            }),
        }));
        validator.validate(&invalid_verb_ast).unwrap();
        assert!(validator.warnings.iter().any(|w| w.contains("Subject-Verb Agreement Mismatch")));

        // 3. Test Qualified Field Access Table Gender Mismatch
        let mut validator = Validator::new(&schema);
        let field_access_expr = Expression::FieldAccess {
            object: Box::new(Expression::Identifier(Identifier {
                text: "विभागायाः".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Shashthi, Vachana::Eka),
                pratipadika: "विभाग".to_string(),
                symbol_type: SymbolType::Table,
                span: None,
            })),
            field: Identifier {
                text: "नाम".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                pratipadika: "नाम".to_string(),
                symbol_type: SymbolType::Column,
                span: None,
            },
        };
        let field_access_ast = AstNode::Statement(crate::ast::Statement::Dql(DqlStatement {
            select: SelectClause {
                columns: vec![crate::ast::SelectItem { expr: field_access_expr, alias: None }],
                distinct: false,
            },
            source: SourceClause {
                derived: None,
                table: Identifier {
                    text: "विभागेभ्यः".to_string(),
                    vibhakti_form: VibhaktiForm::new(Vibhakti::Panchami, Vachana::Bahu),
                    pratipadika: "विभाग".to_string(),
                    symbol_type: SymbolType::Table,
                    span: None,
                },
            },
            joins: vec![],
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            verb_token: Some(Identifier {
                text: "आह्रियताम्".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                pratipadika: "आह्रियताम्".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            }),
        }));
        validator.validate(&field_access_ast).unwrap();
        assert!(validator.warnings.iter().any(|w| w.contains("Genitive table modifier gender/number mismatch")));
    }

    #[test]
    fn test_set_operations_validation() {
        let schema = make_test_schema();

        // Subquery 1: (विभागक्रमाङ्कः) select item: id (Masculine)
        let q1 = Expression::Subquery(Box::new(DqlStatement {
            select: SelectClause {
                columns: vec![crate::ast::SelectItem {
                    expr: Expression::Identifier(Identifier {
                        text: "क्रमाङ्कः".to_string(),
                        vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                        pratipadika: "क्रमाङ्क".to_string(),
                        symbol_type: SymbolType::Column,
                        span: None,
                    }),
                    alias: None,
                }],
                distinct: false,
            },
            source: SourceClause {
                derived: None,
                table: Identifier {
                    text: "विभागेभ्यः".to_string(),
                    vibhakti_form: VibhaktiForm::new(Vibhakti::Panchami, Vachana::Bahu),
                    pratipadika: "विभाग".to_string(),
                    symbol_type: SymbolType::Table,
                    span: None,
                },
            },
            joins: vec![],
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            verb_token: Some(Identifier {
                text: "आह्रियताम्".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                pratipadika: "आह्रियताम्".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            }),
        }));

        // Subquery 2: select item: name (Neuter) -> triggers gender mismatch
        let q2 = Expression::Subquery(Box::new(DqlStatement {
            select: SelectClause {
                columns: vec![crate::ast::SelectItem {
                    expr: Expression::Identifier(Identifier {
                        text: "नाम".to_string(),
                        vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                        pratipadika: "नाम".to_string(),
                        symbol_type: SymbolType::Column,
                        span: None,
                    }),
                    alias: None,
                }],
                distinct: false,
            },
            source: SourceClause {
                derived: None,
                table: Identifier {
                    text: "विभागेभ्यः".to_string(),
                    vibhakti_form: VibhaktiForm::new(Vibhakti::Panchami, Vachana::Bahu),
                    pratipadika: "विभाग".to_string(),
                    symbol_type: SymbolType::Table,
                    span: None,
                },
            },
            joins: vec![],
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            verb_token: Some(Identifier {
                text: "आह्रियताम्".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                pratipadika: "आह्रियताम्".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            }),
        }));

        // Subquery 3: select items: id, name -> triggers arity mismatch
        let q3 = Expression::Subquery(Box::new(DqlStatement {
            select: SelectClause {
                columns: vec![
                    crate::ast::SelectItem {
                        expr: Expression::Identifier(Identifier {
                            text: "क्रमाङ्कः".to_string(),
                            vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                            pratipadika: "क्रमाङ्क".to_string(),
                            symbol_type: SymbolType::Column,
                            span: None,
                        }),
                        alias: None,
                    },
                    crate::ast::SelectItem {
                        expr: Expression::Identifier(Identifier {
                            text: "नाम".to_string(),
                            vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                            pratipadika: "नाम".to_string(),
                            symbol_type: SymbolType::Column,
                            span: None,
                        }),
                        alias: None,
                    }
                ],
                distinct: false,
            },
            source: SourceClause {
                derived: None,
                table: Identifier {
                    text: "विभागेभ्यः".to_string(),
                    vibhakti_form: VibhaktiForm::new(Vibhakti::Panchami, Vachana::Bahu),
                    pratipadika: "विभाग".to_string(),
                    symbol_type: SymbolType::Table,
                    span: None,
                },
            },
            joins: vec![],
            where_clause: None,
            group_by: None,
            having: None,
            order_by: None,
            limit: None,
            verb_token: Some(Identifier {
                text: "आह्रियताम्".to_string(),
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                pratipadika: "आह्रियताम्".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            }),
        }));

        let set_op_ast = AstNode::Statement(crate::ast::Statement::SetOp(SetOpStatement {
            queries: vec![q1, q2, q3],
            op: crate::ast::SetOperation::Union,
            use_all: true,
        }));

        let mut validator = Validator::new(&schema);
        validator.validate(&set_op_ast).unwrap();

        println!("SetOp Warnings: {:#?}", validator.warnings);
        assert!(validator.warnings.iter().any(|w| w.contains("Set operation column gender mismatch")));
        assert!(validator.warnings.iter().any(|w| w.contains("Set operation arity mismatch")));
    }
}
