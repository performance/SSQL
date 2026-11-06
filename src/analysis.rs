use crate::ast::*;
use crate::schema::Schema;
use crate::types::SymbolType;
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisData {
    pub symbols: Vec<SymbolInfo>,
    pub schema_metadata: SchemaMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaMetadata {
    pub tables: Vec<TableMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableMetadata {
    pub name: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolInfo {
    pub name: String, // Original text (Pratipadika or Text)
    pub resolved: Option<String>, // Resolved SQL name
    pub kind: String, // "Table", "Column", "Variable"
    pub span: (usize, usize),
    pub description: String,
    pub container: Option<String>, // Table name for columns
}

pub struct SymbolCollector<'a> {
    schema: &'a Schema,
    active_scopes: Vec<Vec<String>>,
    pub symbols: Vec<SymbolInfo>,
}

impl<'a> SymbolCollector<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            schema,
            active_scopes: Vec::new(),
            symbols: Vec::new(),
        }
    }

    pub fn analyze(&mut self, ast: &AstNode) -> Result<()> {
        match ast {
            AstNode::QueryBatch { statements, parameters, .. } => {
                for param in parameters {
                    self.resolve_identifier(&param.name, None)?;
                }
                for stmt in statements {
                    self.analyze_statement(stmt)?;
                }
            },
            AstNode::Statement(stmt) => {
                self.analyze_statement(stmt)?;
            }
        }
        Ok(())
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::Dql(dql) => self.analyze_dql(dql)?,
            Statement::Ddl(ddl) => self.analyze_ddl(ddl)?,
            Statement::Dcl(_dcl) => {},
            Statement::SetOp(set_op) => self.analyze_set_op(set_op)?,
            Statement::VarDecl { name, value, .. } => {
                self.resolve_identifier(name, None)?;
                self.analyze_expression(value)?;
            },
            Statement::Dml(dml) => self.analyze_dml(dml)?,
            Statement::Explain(inner) => self.analyze_statement(inner)?,
            Statement::Tcl(tcl) => self.analyze_tcl(tcl)?,
        }
        Ok(())
    }

    fn analyze_tcl(&mut self, tcl: &TclStatement) -> Result<()> {
        match tcl {
            TclStatement::Savepoint { name } => {
                if let Some(span) = name.span {
                    self.symbols.push(SymbolInfo {
                        name: name.pratipadika.clone(),
                        resolved: Some(name.text.clone()),
                        kind: "Savepoint".to_string(),
                        span,
                        description: "Savepoint Definition".to_string(),
                        container: None,
                    });
                }
            },
            TclStatement::Rollback { savepoint: Some(name) } => {
                if let Some(span) = name.span {
                    self.symbols.push(SymbolInfo {
                        name: name.pratipadika.clone(),
                        resolved: Some(name.text.clone()),
                        kind: "SavepointReference".to_string(),
                        span,
                        description: "Rollback target savepoint".to_string(),
                        container: None,
                    });
                }
            },
            _ => {}
        }
        Ok(())
    }

    fn analyze_set_op(&mut self, set_op: &SetOpStatement) -> Result<()> {
        for query in &set_op.queries {
            self.analyze_expression(query)?;
        }
        Ok(())
    }

    fn analyze_dml(&mut self, dml: &DmlStatement) -> Result<()> {
        match dml {
             DmlStatement::Insert { table, values, .. } => {
                 self.resolve_identifier(table, None)?;
                 self.analyze_expression(values)?;
             },
             DmlStatement::Update { table, where_clause, set_clause, .. } => {
                 self.resolve_identifier(table, None)?;
                 self.analyze_expression(&set_clause.assignments)?;
                 if let Some(w) = where_clause {
                     self.analyze_expression(&w.condition)?;
                 }
             },
             DmlStatement::Delete { table, where_clause } => {
                 self.resolve_identifier(table, None)?;
                 if let Some(w) = where_clause {
                     self.analyze_expression(&w.condition)?;
                 }
             },
             DmlStatement::InsertSelect { table, columns, query, .. } => {
                 self.resolve_identifier(table, None)?;
                 let table_stem = self.schema.resolve_pratipadika(&table.text)
                      .map(|s| s.to_string())
                      .unwrap_or_else(|| table.text.clone());
                 if let Some(cols) = columns {
                     for col in cols {
                         self.resolve_identifier(col, Some(&table_stem))?;
                     }
                 }
                 self.analyze_dql(query)?;
             }
        }
        Ok(())
    }

    fn analyze_dql(&mut self, dql: &DqlStatement) -> Result<()> {
        // Collect active tables (Sanskrit names) for context
        let mut scope_tables = Vec::new();
        scope_tables.push(dql.source.table.pratipadika.clone());
        for join in &dql.joins {
            scope_tables.push(join.table.pratipadika.clone());
        }
        
        self.active_scopes.push(scope_tables);
        
        // 1. Source
        self.resolve_identifier(&dql.source.table, None)?;
        
        // 2. Joins
        for join in &dql.joins {
            self.resolve_identifier(&join.table, None)?;
            if let Some(cond) = &join.condition {
                 self.analyze_expression(cond)?;
            }
        }

        // 3. Select Columns
        for item in &dql.select.columns {
            self.analyze_expression(&item.expr)?;
        }

        // 4. Where
        if let Some(where_clause) = &dql.where_clause {
            self.analyze_expression(&where_clause.condition)?;
        }

        // 5. Group By
        if let Some(group_by) = &dql.group_by {
            for expr in &group_by.expressions {
                self.analyze_expression(expr)?;
            }
        }

        // 6. Having
        if let Some(having) = &dql.having {
            self.analyze_expression(&having.condition)?;
        }

        // 7. Order By
        if let Some(order_by) = &dql.order_by {
            for item in &order_by.items {
                self.analyze_expression(&item.expr)?;
            }
        }

        // 8. Limit
        if let Some(limit) = &dql.limit {
            self.analyze_expression(&limit.count)?;
        }
        
        self.active_scopes.pop();
        Ok(())
    }
    
    fn analyze_ddl(&mut self, ddl: &DdlStatement) -> Result<()> {
        match ddl {
            DdlStatement::Create(create_stmt) => {
                // Determine kind ("Table" or "View")
                let kind = if create_stmt.type_name.pratipadika == "दृश्यम्" { "View" } else { "Table" };
                
                // Add symbol for the Created Entity
                if let Some(span) = create_stmt.name.span {
                    self.symbols.push(SymbolInfo {
                         name: create_stmt.name.pratipadika.clone(),
                         resolved: Some(create_stmt.name.text.clone()), // Use text as default resolved?
                         kind: kind.to_string(),
                         span,
                         description: format!("New {} Definition", kind),
                         container: None,
                    });
                }
                
                if let Some(query) = &create_stmt.query {
                    self.analyze_statement(query)?;
                }
            },
            DdlStatement::Drop(drop_stmt) => {
                 self.resolve_identifier(&drop_stmt.name, None)?;
            }
            DdlStatement::CreateIndex(create_idx) => {
                if let Some(span) = create_idx.name.span {
                    self.symbols.push(SymbolInfo {
                        name: create_idx.name.pratipadika.clone(),
                        resolved: Some(create_idx.name.text.clone()),
                        kind: "Index".to_string(),
                        span,
                        description: "New Index Definition".to_string(),
                        container: Some(create_idx.table_name.pratipadika.clone()),
                    });
                }
                self.resolve_identifier(&create_idx.table_name, None)?;
                for col in &create_idx.columns {
                    self.resolve_identifier(&col.name, Some(&create_idx.table_name.pratipadika))?;
                }
            }
            DdlStatement::DropIndex(drop_idx) => {
                self.resolve_identifier(&drop_idx.name, None)?;
            }
            DdlStatement::AlterTable(alter_stmt) => {
                self.resolve_identifier(&alter_stmt.table_name, None)?;
                match &alter_stmt.action {
                    crate::ast::AlterAction::AddColumn(column_def) => {
                        if let Some(span) = column_def.name.span {
                            self.symbols.push(SymbolInfo {
                                name: column_def.name.pratipadika.clone(),
                                resolved: Some(column_def.name.text.clone()),
                                kind: "Column".to_string(),
                                span,
                                description: "Added Column Definition".to_string(),
                                container: Some(alter_stmt.table_name.pratipadika.clone()),
                            });
                        }
                    }
                    crate::ast::AlterAction::DropColumn(col_name) => {
                        self.resolve_identifier(col_name, Some(&alter_stmt.table_name.pratipadika))?;
                    }
                    crate::ast::AlterAction::RenameColumn { old_name, new_name } => {
                        self.resolve_identifier(old_name, Some(&alter_stmt.table_name.pratipadika))?;
                        if let Some(span) = new_name.span {
                            self.symbols.push(SymbolInfo {
                                name: new_name.pratipadika.clone(),
                                resolved: Some(new_name.text.clone()),
                                kind: "Column".to_string(),
                                span,
                                description: "Renamed Column Definition".to_string(),
                                container: Some(alter_stmt.table_name.pratipadika.clone()),
                            });
                        }
                    }
                    crate::ast::AlterAction::RenameTable(new_table_name) => {
                        if let Some(span) = new_table_name.span {
                            self.symbols.push(SymbolInfo {
                                name: new_table_name.pratipadika.clone(),
                                resolved: Some(new_table_name.text.clone()),
                                kind: "Table".to_string(),
                                span,
                                description: "Renamed Table Definition".to_string(),
                                container: None,
                            });
                        }
                    }
                }
            }
            DdlStatement::CreateUdf(udf_stmt) => {
                if let Some(span) = udf_stmt.name.span {
                    self.symbols.push(SymbolInfo {
                        name: udf_stmt.name.pratipadika.clone(),
                        resolved: Some(udf_stmt.name.text.clone()),
                        kind: "Function".to_string(),
                        span,
                        description: "User-Defined Function (शुद्धम् कार्यम्)".to_string(),
                        container: None,
                    });
                }
                self.analyze_expression(&udf_stmt.body)?;
            }
        }
        Ok(())
    }

    fn analyze_expression(&mut self, expr: &Expression) -> Result<()> {
        match expr {
            Expression::NamedArg { value, .. } => self.analyze_expression(value)?,
            Expression::Identifier(ident) => self.resolve_identifier(ident, None)?,
            Expression::FieldAccess { object, field } => {
                // 1. Context Resolution
                let context = if let Expression::Identifier(obj_id) = &**object {
                    Some(obj_id.pratipadika.clone())
                } else {
                    None
                };

                // 2. Analyze object
                self.analyze_expression(object)?;
                
                // 3. Analyze field with context
                self.resolve_identifier(field, context.as_deref())?;
            },
            Expression::BinaryOp { left, right, .. } | 
            Expression::LogicalOp { left, right, .. } => {
                self.analyze_expression(left)?;
                self.analyze_expression(right)?;
            },
            Expression::FunctionCall { args, window, .. } => {
                for arg in args {
                    self.analyze_expression(arg)?;
                }
                if let Some(window) = window {
                    for part in &window.partition_by {
                        self.analyze_expression(part)?;
                    }
                    for order in &window.order_by {
                        self.analyze_expression(&order.expr)?;
                    }
                }
            },
            Expression::Case { cases } => {
                for case in cases {
                    if let Some(cond) = &case.condition {
                        self.analyze_expression(cond)?;
                    }
                    self.analyze_expression(&case.result)?;
                }
            },
            Expression::Cast { expr, .. } => {
                self.analyze_expression(expr)?;
            },
            Expression::Aggregate { expr, .. } => {
                self.analyze_expression(expr)?;
            },
            Expression::Subquery(dql) => {
                self.analyze_dql(dql)?;
            },
            Expression::SetOp(set_op) => {
                self.analyze_set_op(set_op)?;
            },
            _ => {} 
        }
        Ok(())
    }
    
    fn resolve_identifier(&mut self, ident: &Identifier, context: Option<&str>) -> Result<()> {
        let span = match ident.span {
            Some(s) => s,
            None => return Ok(()), // No source location, skip
        };
        
        // Skip placeholders
        if ident.text == "?" { return Ok(()); }

        let mut resolved_name = None;
        let kind = ident.symbol_type.clone();
        let mut container = context.map(|s| s.to_string());
        let mut description = String::new();

        match ident.symbol_type {
            SymbolType::Table => {
                 if let Some(sql_name) = self.schema.resolve_table(&ident.pratipadika) {
                     resolved_name = Some(sql_name.to_string());
                     description = format!("Table: {}", sql_name);
                 } else if let Some(sql_name) = self.schema.resolve_table(&ident.text) {
                     resolved_name = Some(sql_name.to_string());
                     description = format!("Table: {}", sql_name);
                 }
            },
            SymbolType::Column => {
                // Resolution Logic (Mirrors AstTransformer)
                let mut resolved = None;
                
                // 1. Explicit Context
                if let Some(ctx) = context {
                    if let Some((sql_col, _, _)) = self.schema.resolve_column(&ident.pratipadika, Some(ctx)) {
                        resolved = Some(sql_col);
                    } else if let Some((sql_col, _, _)) = self.schema.resolve_column(&ident.text, Some(ctx)) {
                        resolved = Some(sql_col);
                    }
                }
                
                // 2. Global/Scope Search
                if resolved.is_none() {
                     if context.is_none() {
                         if let Some(current_scope) = self.active_scopes.last() {
                             // Prefer active scope
                             for table_pratipadika in current_scope {
                                 if let Some((sql_col, _, parent_table)) = self.schema.resolve_column(&ident.pratipadika, Some(table_pratipadika)) {
                                     resolved = Some(sql_col);
                                     container = Some(parent_table.to_string());
                                     break; // First match wins
                                 } else if let Some((sql_col, _, parent_table)) = self.schema.resolve_column(&ident.text, Some(table_pratipadika)) {
                                     resolved = Some(sql_col);
                                     container = Some(parent_table.to_string());
                                     break;
                                 }
                             }
                         }
                     }
                     
                     if resolved.is_none() {
                          if let Some((sql_col, _, parent_table)) = self.schema.resolve_column(&ident.pratipadika, None) {
                              resolved = Some(sql_col);
                              container = Some(parent_table.to_string());
                          } else if let Some((sql_col, _, parent_table)) = self.schema.resolve_column(&ident.text, None) {
                              resolved = Some(sql_col);
                              container = Some(parent_table.to_string());
                          }
                     }
                }
                
                if let Some(sql_col) = resolved {
                    resolved_name = Some(sql_col.to_string());
                    description = format!("Column: {}", sql_col);
                }
            },
            SymbolType::Variable => {
                description = "Variable".to_string();
            },
            SymbolType::Function => {
                 description = "Function".to_string();
            },
            _ => { description = format!("{:?}", kind); }
        }
        
        // Enhance Description
        if let Some(ref r) = resolved_name {
            if let Some(ref c) = container {
                description = format!("Column: {}.{}", c, r);
            }
        }
        
        self.symbols.push(SymbolInfo {
            name: ident.pratipadika.clone(),
            resolved: resolved_name,
            kind: format!("{:?}", kind),
            span,
            description,
            container,
        });
        
        Ok(())
    }
}
