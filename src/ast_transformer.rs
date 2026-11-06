//! AST Transformer
//!
//! Transforms the SSQL AST by resolving Devanagari identifiers to their SQL equivalents
//! based on the schema. This creates an "intermediate" AST where all names are English/SQL.
//! This simplifies code generation and debugging.

use std::collections::HashMap;
use crate::ast::*;
use crate::schema::Schema;
use crate::symbol_table::SymbolTable;
use crate::types::SymbolType;
use anyhow::Result;

pub struct AstTransformer<'a> {
    schema: &'a Schema,
    symbol_table: &'a SymbolTable,
    active_scopes: Vec<Vec<String>>,
    // Map of CTE Name -> { Column Pratipadika -> Resolved SQL Name }
    cte_map: HashMap<String, HashMap<String, String>>,
    // Map of CTE Name -> Source Table Name
    cte_source_map: HashMap<String, String>,
}

impl<'a> AstTransformer<'a> {
    pub fn new(schema: &'a Schema, symbol_table: &'a SymbolTable) -> Self {
        Self { 
            schema,
            symbol_table,
            active_scopes: Vec::new(),
            cte_map: HashMap::new(),
            cte_source_map: HashMap::new(),
        }
    }

    pub fn transform_statement(&mut self, stmt: &mut Statement) -> Result<()> {
        match stmt {
            Statement::Dql(dql) => self.transform_dql(dql)?,
            Statement::Ddl(ddl) => self.transform_ddl(ddl)?,
            Statement::Dcl(dcl) => self.transform_dcl(dcl)?,
            Statement::SetOp(set_op) => self.transform_set_op(set_op)?,
            Statement::VarDecl { name, value, cycle_col, .. } => {
                // 0. Transform the CTE Name itself (e.g. Sanskrit -> SQL Name)
                // This ensures the WITH clause uses the correct internal name (e.g. cte_reviews_agg)
                name.symbol_type = SymbolType::Table; 
                self.transform_identifier(name)?;

                if let Some(col) = cycle_col {
                    col.symbol_type = SymbolType::Column;
                    self.transform_identifier(col)?;
                }

                // 1. Clone the untransformed value to preserve Sanskrit names
                let original_value = value.clone();
                
                // 2. Transform the value (Inner Query) - This mutates 'value' to SQL names
                self.transform_expression(value)?;

                // 3. If it's a Subquery (CTE), infer schema by comparing Original (Sanskrit) vs Transformed (SQL)
                if let (Expression::Subquery(transformed_dql), Expression::Subquery(original_dql)) = (&**value, &*original_value) {
                    // Populate Source Map (Ancestry)
                    // Use ORIGINAL DQL to get the Sanskrit Pratipadika of the source table
                    let source_table = &original_dql.source.table.pratipadika;
                    // Key must match what identifiers resolve to. identifiers resolve to SQL Names (cte_reviews_agg).
                    // So we must use the transformed name as key.
                    self.cte_source_map.insert(name.text.clone(), source_table.clone());
                    self.cte_source_map.insert(name.pratipadika.clone(), source_table.clone());

                    let mut col_map = HashMap::new();
                    
                    // Iterate columns in parallel
                    for (t_col, o_col) in transformed_dql.select.columns.iter().zip(original_dql.select.columns.iter()) {
                         // Determine SQL Name from Transformed Column
                         let sql_name = if let Some(alias) = &t_col.alias {
                             alias.clone()
                         } else {
                             match &t_col.expr {
                                 Expression::Identifier(id) => id.text.clone(),
                                 Expression::FieldAccess { field, .. } => field.text.clone(),
                                 _ => "".to_string()
                             }
                         };

                         // Determine Sanskrit Name(s) from Original Column
                         // We map multiple Sanskrit keys (Pratipadika, Text, Alias) to the single SQL name
                         if !sql_name.is_empty() {
                             // 1. Alias (if present in original)
                             if let Some(alias) = &o_col.alias {
                                 col_map.insert(alias.clone(), sql_name.clone());
                             } else {
                                 // 2. Identifier (if no alias)
                                 match &o_col.expr {
                                     Expression::Identifier(id) => {
                                         col_map.insert(id.pratipadika.clone(), sql_name.clone());
                                         col_map.insert(id.text.clone(), sql_name.clone());
                                     },
                                     Expression::FieldAccess { field, .. } => {
                                         col_map.insert(field.pratipadika.clone(), sql_name.clone());
                                         col_map.insert(field.text.clone(), sql_name.clone());
                                     },
                                     _ => {}
                                 }
                             }
                         }
                    }
                    
                    self.cte_map.insert(name.pratipadika.clone(), col_map);
                }
            },
            Statement::Dml(dml) => self.transform_dml(dml)?,
            Statement::Explain(inner) => self.transform_statement(inner)?,
            Statement::Tcl(tcl) => self.transform_tcl(tcl)?,
        }
        Ok(())
    }

    fn transform_tcl(&mut self, _tcl: &mut TclStatement) -> Result<()> {
        Ok(())
    }

    fn transform_set_op(&mut self, set_op: &mut SetOpStatement) -> Result<()> {
        for query in &mut set_op.queries {
            self.transform_expression(query)?;
        }
        Ok(())
    }

    fn transform_dml(&mut self, dml: &mut DmlStatement) -> Result<()> {
        match dml {
             DmlStatement::Insert { table, values, .. } => {
                 self.transform_identifier(table)?;
                 self.transform_expression(values)?;
             },
             DmlStatement::Update { table, where_clause, set_clause, .. } => {
                 self.transform_identifier(table)?;
                 self.transform_expression(&mut set_clause.assignments)?;
                 if let Some(w) = where_clause {
                     self.transform_expression(&mut w.condition)?;
                 }
             },
             DmlStatement::Delete { table, where_clause } => {
                 self.transform_identifier(table)?;
                 if let Some(w) = where_clause {
                     self.transform_expression(&mut w.condition)?;
                 }
             },
             DmlStatement::InsertSelect { table, columns, query, .. } => {
                 self.transform_identifier(table)?;
                 if let Some(cols) = columns {
                     for col in cols {
                         self.transform_identifier(col)?;
                     }
                 }
                 self.transform_dql(query)?;
             }
        }
        Ok(())
    }

    fn transform_dql(&mut self, dql: &mut DqlStatement) -> Result<()> {
        // 1. Transform Tables FIRST to get strict SQL Names (e.g. cte_reviews_agg)
        self.transform_identifier(&mut dql.source.table)?;
        
        for join in &mut dql.joins {
            self.transform_identifier(&mut join.table)?;
        }

        // 2. Build Scope using Transformed Names
        let mut scope_tables = Vec::new();
        scope_tables.push(dql.source.table.pratipadika.clone());
        for join in &dql.joins {
            scope_tables.push(join.table.pratipadika.clone());
        }
        
        self.active_scopes.push(scope_tables);
        
        // 3. Transform Expressions (Conditions, Selects) WITH the new Scope active.
        
        // Joins Conditions
        for join in &mut dql.joins {
            if let Some(cond) = &mut join.condition {
                 self.transform_expression(cond)?;
            }
        }

        // 3. Select Columns
        for item in &mut dql.select.columns {
            self.transform_expression(&mut item.expr)?;
        }

        // 4. Where
        if let Some(where_clause) = &mut dql.where_clause {
            self.transform_expression(&mut where_clause.condition)?;
        }

        // 5. Group By
        if let Some(group_by) = &mut dql.group_by {
            for expr in &mut group_by.expressions {
                self.transform_expression(expr)?;
            }
        }

        // 6. Having
        if let Some(having) = &mut dql.having {
            self.transform_expression(&mut having.condition)?;
        }

        // 7. Order By
        if let Some(order_by) = &mut dql.order_by {
            for item in &mut order_by.items {
                self.transform_expression(&mut item.expr)?;
            }
        }

        // 8. Limit
        if let Some(limit) = &mut dql.limit {
            self.transform_expression(&mut limit.count)?;
        }
        
        self.active_scopes.pop();

        Ok(())
    }
    
    fn transform_ddl(&mut self, ddl: &mut DdlStatement) -> Result<()> {
        match ddl {
            DdlStatement::Create(create_stmt) => {
                // Transform Table Name (Sanskrit -> English)
                self.transform_identifier(&mut create_stmt.name)?;
                
                // If there's a subquery (CREATE VIEW AS ...), transform it
                if let Some(query) = &mut create_stmt.query {
                    self.transform_statement(query)?;
                }
            },
            DdlStatement::Drop(drop_stmt) => {
                 self.transform_identifier(&mut drop_stmt.name)?;
            }
            DdlStatement::CreateIndex(create_idx) => {
                self.transform_identifier(&mut create_idx.table_name)?;
                self.transform_identifier(&mut create_idx.name)?;
                for col in &mut create_idx.columns {
                    self.transform_identifier(&mut col.name)?;
                }
            }
            DdlStatement::DropIndex(drop_idx) => {
                self.transform_identifier(&mut drop_idx.name)?;
            }
            DdlStatement::AlterTable(alter_stmt) => {
                self.transform_identifier(&mut alter_stmt.table_name)?;
                match &mut alter_stmt.action {
                    crate::ast::AlterAction::AddColumn(column_def) => {
                        self.transform_identifier(&mut column_def.name)?;
                    }
                    crate::ast::AlterAction::DropColumn(col_name) => {
                        self.transform_identifier(col_name)?;
                    }
                    crate::ast::AlterAction::RenameColumn { old_name, new_name } => {
                        self.transform_identifier(old_name)?;
                        self.transform_identifier(new_name)?;
                    }
                    crate::ast::AlterAction::RenameTable(new_table_name) => {
                        self.transform_identifier(new_table_name)?;
                    }
                }
            }
            DdlStatement::CreateUdf(_) => {}
        }
        Ok(())
    }

    fn transform_dcl(&mut self, _dcl: &mut DclStatement) -> Result<()> {
        Ok(())
    }

    fn transform_expression(&mut self, expr: &mut Expression) -> Result<()> {
        match expr {
            // RANDOM takes no argument and so stands alone, where every other
            // word is a column; its name is reserved (reserved_words.rs).
            Expression::Identifier(ident) if ident.text == "यादृच्छिकः" => {
                *expr = Expression::FunctionCall { name: ident.text.clone(), args: vec![], window: None, pronoun_token: None };
            }
            Expression::Identifier(ident) => self.transform_identifier(ident)?,
            Expression::FieldAccess { object, field } => {
                let name = field.pratipadika.clone();
                // A built-in said of a column (`शीर्षकस्य मार्जनम्`), by its
                // inflected name; a hand-kept stem list used to miss some and
                // pass the stem on, which code generation rejects.
                let is_builtin_func = crate::reserved_words::BUILTIN_FUNCTION_WORDS.contains(&field.text.as_str());
                let is_real_column_of_table = if let Expression::Identifier(obj_id) = &**object {
                    if let Some(table_stem) = self
                        .schema
                        .resolve_pratipadika(&obj_id.pratipadika)
                        .or_else(|| self.schema.resolve_pratipadika(&obj_id.text))
                    {
                        self.schema.resolve_column(&field.text, Some(table_stem)).is_some()
                            || self.schema.resolve_column(&field.pratipadika, Some(table_stem)).is_some()
                    } else {
                        false
                    }
                } else {
                    false
                };
                let is_udf_func = !is_real_column_of_table
                    && (self.schema.functions.contains_key(&field.text)
                        || self.schema.functions.contains_key(&field.pratipadika)
                        || self.schema.functions.contains_key(&name));
                if is_builtin_func || is_udf_func {
                    let mut obj = *object.clone();
                    if let Expression::Identifier(ref mut id) = obj {
                        if self.schema.resolve_table(&id.pratipadika).is_none()
                            && self.schema.resolve_table(&id.text).is_none()
                        {
                            id.symbol_type = SymbolType::Column;
                        }
                    }
                    self.transform_expression(&mut obj)?;
                    let fn_name = field.text.clone();
                    *expr = Expression::FunctionCall {
                        name: fn_name,
                        args: vec![obj],
                        window: None,
                        pronoun_token: None,
                    };
                    return Ok(());
                }

                // 1. Capture context BEFORE transform
                // We need the Sanskrit Pratipadika of the object (Table) for schema lookup
                // OR checking against CTE map
                let context = if let Expression::Identifier(obj_id) = &**object {
                    Some(obj_id.pratipadika.clone())
                } else {
                    None
                };

                // 2. Transform the object (Table/Source)
                self.transform_expression(object)?;
                
                // 3. Transform the field with context
                self.transform_identifier_with_context(field, context.as_deref())?;
            },
            Expression::BinaryOp { left, right, .. } | 
            Expression::LogicalOp { left, right, .. } => {
                self.transform_expression(left)?;
                self.transform_expression(right)?;
            },
            Expression::FunctionCall { name, args, window, .. } => {
                for arg in args.iter_mut() {
                    self.transform_expression(arg)?;
                }
                if args.iter().any(|a| matches!(a, Expression::NamedArg { .. })) {
                    let stem = name
                        .trim_end_matches("म्")
                        .trim_end_matches('ं')
                        .trim_end_matches('ः');
                    if let Some(expected_params) = self
                        .schema
                        .udf_params
                        .get(name)
                        .or_else(|| self.schema.udf_params.get(stem))
                    {
                        let mut ordered_args = Vec::with_capacity(expected_params.len());
                        for expected_stem in expected_params {
                            let found = args.iter().find_map(|a| {
                                if let Expression::NamedArg { param, value } = a {
                                    let p_stem = param
                                        .pratipadika
                                        .trim_end_matches("म्")
                                        .trim_end_matches('ं')
                                        .trim_end_matches('ः');
                                    let p_text_stem = param
                                        .text
                                        .trim_end_matches("म्")
                                        .trim_end_matches('ं')
                                        .trim_end_matches('ः');
                                    if p_stem == expected_stem
                                        || p_text_stem == expected_stem
                                        || &param.text == expected_stem
                                        || &param.pratipadika == expected_stem
                                    {
                                        return Some((**value).clone());
                                    }
                                }
                                None
                            });
                            if let Some(val) = found {
                                ordered_args.push(val);
                            } else {
                                anyhow::bail!(
                                    "Missing named argument '{}' in call to UDF '{}'",
                                    expected_stem,
                                    name
                                );
                            }
                        }
                        *args = ordered_args;
                    } else {
                        // Unwrap NamedArg in-place if param list isn't registered
                        for arg in args.iter_mut() {
                            if let Expression::NamedArg { value, .. } = arg {
                                *arg = (**value).clone();
                            }
                        }
                    }
                }
                if let Some(window) = window {
                    for part in &mut window.partition_by {
                        self.transform_expression(part)?;
                    }
                    for order in &mut window.order_by {
                        self.transform_expression(&mut order.expr)?;
                    }
                }
            },
            Expression::NamedArg { value, .. } => {
                self.transform_expression(value)?;
            },
            Expression::Case { cases } => {
                for case in cases {
                    if let Some(cond) = &mut case.condition {
                        self.transform_expression(cond)?;
                    }
                    self.transform_expression(&mut case.result)?;
                }
            },
            Expression::Cast { expr, .. } => {
                self.transform_expression(expr)?;
            },

            Expression::Aggregate { expr, .. } => {
                self.transform_expression(expr)?;
            },
            Expression::Subquery(dql) => {
                // If the source table is already in scope and selects a single binary comparison
                // involving a column, interpret it as a FieldAccess filter in the active scope.
                
                let mut convert = false;
                let table_name = &dql.source.table.pratipadika;
                
                if let Some(scope) = self.active_scopes.last() {
                    if scope.contains(table_name) 
                       && dql.select.columns.len() == 1 
                       && dql.joins.is_empty() 
                       && dql.where_clause.is_none() 
                       && dql.group_by.is_none() 
                       && dql.having.is_none() {
                        
                        if let Expression::BinaryOp { left, .. } = &dql.select.columns[0].expr {
                            if let Expression::Identifier(_) = &**left {
                                convert = true;
                            }
                        }
                    }
                }

                if convert {
                     // Perform conversion: Subquery -> BinaryOp(FieldAccess, Right)
                     // We take ownership of parts
                     let source_table = dql.source.table.clone();
                     // Take the expression from select columns
                     // We can't move out of borrowed dql easily without replace/clone
                     let inner_expr = dql.select.columns[0].expr.clone();
                     
                     if let Expression::BinaryOp { left, op, right } = inner_expr {
                         if let Expression::Identifier(col_ident) = *left {
                             // Construct FieldAccess
                             let field_access = Expression::FieldAccess {
                                 object: Box::new(Expression::Identifier(source_table)),
                                 field: col_ident
                             };
                             
                             let new_expr = Expression::BinaryOp {
                                 left: Box::new(field_access),
                                 op,
                                 right,
                             };
                             
                             *expr = new_expr;
                             
                             // Re-transform the Result (Important!)
                             self.transform_expression(expr)?;
                             return Ok(());
                         }
                     }
                }

                self.transform_dql(dql)?;
            },
            Expression::SetOp(set_op) => {
                self.transform_set_op(set_op)?;
            },
            _ => {} 
        }
        Ok(())
    }

    fn transform_identifier(&mut self, ident: &mut Identifier) -> Result<()> {
        self.transform_identifier_with_context(ident, None)
    }

    fn transform_identifier_with_context(&mut self, ident: &mut Identifier, context: Option<&str>) -> Result<()> {
        match ident.symbol_type {
            SymbolType::Table => {
                // Resolve as Table
                if let Some(sql_name) = self.schema.resolve_table(&ident.pratipadika) {
                    ident.text = sql_name.to_string();
                } else if let Some(sql_name) = self.schema.resolve_table(&ident.text) {
                    ident.text = sql_name.to_string();
                } else {
                    // Not in schema? Likely a CTE or variable table.
                    // Normalize to Pratipadika to match the definition site (which uses Pratipadika).
                    ident.text = ident.pratipadika.clone();
                }

                // Also normalize `pratipadika` itself to the resolved base
                // stem. A CTE name referenced via an inflected surface form
                // (e.g. ablative "...आत्") in a *later* top-level statement
                // can't be recognized as such until the CTE is registered
                // mid-compile - the parser has no declension pattern for it
                // up front - so pratipadika still holds the raw surface
                // form at this point for that case. Downstream codegen
                // (table_context lookups into Schema::tables, keyed by
                // pratipadika) needs the real base stem, not whatever
                // case-inflected form the query happened to use, or every
                // such lookup misses entirely.
                if let Some(p) = self.schema.resolve_pratipadika(&ident.pratipadika) {
                    ident.pratipadika = p.to_string();
                } else if let Some(p) = self.schema.resolve_pratipadika(&ident.text) {
                    ident.pratipadika = p.to_string();
                }

                // Rewrite to In-Scope Ancestor if needed
                if let Some(current_scope) = self.active_scopes.last() {
                     let table_name = ident.pratipadika.clone();
                     if !current_scope.contains(&table_name) {
                         // Check ancestors
                         for scope_table in current_scope {
                             let mut curr = scope_table.as_str();
                             for _ in 0..10 {
                                 if curr == table_name {
                                     // Found ancestor! Rewrite Identifier to use scope_table (Descendant)
                                     // Because we are accessing TableName.Field
                                     // We want Descendant.Field
                                     ident.text = scope_table.clone();
                                     ident.pratipadika = scope_table.clone();
                                     break;
                                 }
                                 if let Some(parent) = self.cte_source_map.get(curr) {
                                     curr = parent;
                                 } else {
                                     break;
                                 }
                             }
                         }
                     }
                }
            },
            SymbolType::Column => {
                // 0. Check for Variable Shadowing
                if context.is_none() && (self.symbol_table.reverse_lookup.contains_key(&ident.text) 
                    || self.cte_map.contains_key(&ident.text) 
                    || self.cte_map.contains_key(&ident.pratipadika)) {
                     ident.symbol_type = SymbolType::Variable;
                     return Ok(());
                }

                let mut resolved = None;
                
                // 1. Explicit Context
                if let Some(ctx) = context {
                    // Try Schema Resolution
                    if let Some((sql_col, _, _)) = self.schema.resolve_column(&ident.pratipadika, Some(ctx)) {
                        resolved = Some(sql_col.to_string());
                    } else if let Some((sql_col, _, _)) = self.schema.resolve_column(&ident.text, Some(ctx)) {
                         resolved = Some(sql_col.to_string());
                    }

                    // CTE Fallback: If not found in proper schema context, check CTE map
                    if resolved.is_none() {
                        if let Some(cols) = self.cte_map.get(ctx) {
                            if let Some(sql_col) = cols.get(&ident.pratipadika).or_else(|| cols.get(&ident.text)) {
                                resolved = Some(sql_col.clone());
                            }
                        }
                    }

                    // Validation handles scope checking if Schema Table
                    if resolved.is_some() && self.schema.resolve_table(ctx).is_some() {
                        if let Some(current_scope) = self.active_scopes.last() {
                            if !current_scope.contains(&ctx.to_string()) {
                                // Check for Ancestry Validity (Using Sanskrit names)
                                let mut valid = false;
                                for scope_table in current_scope {
                                     let mut curr = scope_table.clone();
                                     for _ in 0..10 {
                                         if curr == ctx {
                                             valid = true;
                                             break;
                                         }
                                         
                                         // Look up parent using current node
                                         let parent_name = if let Some(p) = self.cte_source_map.get(&curr) {
                                             p
                                         } else {
                                             break;
                                         };
                                         curr = parent_name.clone();
                                     }
                                     if valid { break; }
                                }
                                
                                if !valid {
                                    anyhow::bail!("Table '{}' is referenced by column '{}' but not joined.", ctx, ident.text);
                                }
                            }
                        }
                    }
                }
                
                // 2. Implicit Context (Active Scope Search)
                if resolved.is_none() && context.is_none() {
                    if let Some(current_scope) = self.active_scopes.last() {
                         let mut matches = Vec::new();

                         for table_pratipadika in current_scope {
                             // Case A: Table is in Schema -> Check Schema
                             if let Some((sql_col, _, _)) = self.schema.resolve_column(&ident.pratipadika, Some(table_pratipadika)) {
                                 matches.push(sql_col.to_string());
                             } else if let Some((sql_col, _, _)) = self.schema.resolve_column(&ident.text, Some(table_pratipadika)) {
                                 matches.push(sql_col.to_string());
                             } else {
                                 // Case B: Table is CTE -> Check CTE Map
                                 if let Some(cols) = self.cte_map.get(table_pratipadika) {
                                     if let Some(sql_col) = cols.get(&ident.pratipadika).or_else(|| cols.get(&ident.text)) {
                                         matches.push(sql_col.clone());
                                     }
                                 }
                             }
                         }
                         
                         if matches.len() == 1 {
                             resolved = Some(matches[0].clone());
                         }
                    }
                }

                // 3. Last Resort: Global Search (Only for Schema Tables)
                if resolved.is_none() {
                     if let Some((sql_col, _, origin_table)) = self.schema.resolve_column(&ident.pratipadika, None)
                        .or_else(|| self.schema.resolve_column(&ident.text, None)) {
                            
                         // Verify validity
                         let mut is_valid = false;
                         if let Some(current_scope) = self.active_scopes.last() {
                             if current_scope.contains(&origin_table.to_string()) {
                                 is_valid = true;
                             } else {
                                 // Check for CTEs (Not really applicable for schema global lookup, but if CTE shadows schema table?)
                                 // Usually global lookup implies Schema Table.
                                 // We only allow if the Origin Table is in scope.
                             }
                         }
                         
                         if is_valid {
                             resolved = Some(sql_col.to_string());
                         }
                     }
                }

                // Apply
                if let Some(sql_col) = resolved {
                     ident.text = sql_col;
                     // We keep pratipadika as is? Or update it?
                     // Usually update it to match SQL for consistency, but if it's an alias it might not matter.
                     ident.pratipadika = ident.text.clone(); 
                }
            },
            _ => {}
        }
        Ok(())
    }
}
