use crate::ast::{AstNode, Statement, Identifier, DqlStatement};
use crate::schema::Schema;
use crate::types::SymbolType;

pub struct SemanticResolver<'a> {
    schema: &'a Schema,
}

impl<'a> SemanticResolver<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self { schema }
    }

    pub fn resolve(&self, ast: &mut AstNode) {
        match ast {
            AstNode::QueryBatch { statements, .. } => {
                for stmt in statements {
                    self.resolve_statement(stmt);
                }
            }
            AstNode::Statement(stmt) => self.resolve_statement(stmt),
        }
    }

    fn resolve_statement(&self, stmt: &mut Statement) {
        match stmt {
            Statement::Dql(query) => self.resolve_dql(query),
            Statement::SetOp(set_op) => {
                for q in &mut set_op.queries {
                    self.resolve_expression(q, None);
                }
            },
            _ => {}
        }
    }

    fn resolve_dql(&self, query: &mut DqlStatement) {
        // 1. Resolve source table first to establish context
        let mut table_context: Option<String> = None;
        
        let source_table = &mut query.source.table;
        // Identify table
        if let Some(_) = self.schema.resolve_table(&source_table.text) {
             source_table.symbol_type = SymbolType::Table;
             // If the text is a declined form (e.g. पुस्तकेभ्यः), resolve to pratipadika (पुस्तक)
             if let Some(pratipadika) = self.schema.resolve_pratipadika(&source_table.text) {
                 source_table.pratipadika = pratipadika.to_string();
                 table_context = Some(pratipadika.to_string());
                 
                 // Resolving Gender
                 if let Some(table_info) = self.schema.tables.get(pratipadika) {
                     source_table.vibhakti_form.linga = Some(table_info.gender);
                 }
             } else {
                 // Already pratipadika
                 source_table.pratipadika = source_table.text.clone();
                 table_context = Some(source_table.text.clone());
                 
                 if let Some(table_info) = self.schema.tables.get(&source_table.text) {
                     source_table.vibhakti_form.linga = Some(table_info.gender);
                 }
             }
        }

        // 2. Resolve Select Columns
        for col in &mut query.select.columns {
            self.resolve_expression(&mut col.expr, table_context.as_deref());
        }

        // 3. Resolve Where Clause
        if let Some(where_clause) = &mut query.where_clause {
             self.resolve_expression(&mut where_clause.condition, table_context.as_deref());
        }
    }

    fn resolve_expression(&self, expr: &mut crate::ast::Expression, table_context: Option<&str>) {
        match expr {
            crate::ast::Expression::Identifier(id) => self.resolve_identifier(id, table_context),
            crate::ast::Expression::BinaryOp { left, right, .. } => {
                 self.resolve_expression(left, table_context);
                 self.resolve_expression(right, table_context);
            }
             crate::ast::Expression::Literal(_) => {},
             crate::ast::Expression::FieldAccess { object, field } => {
                 self.resolve_expression(object, table_context);
                 // If object resolved to a Table, use it as context for the field
                 let new_context = if let crate::ast::Expression::Identifier(obj_id) = &**object {
                     if obj_id.symbol_type == SymbolType::Table {
                         Some(obj_id.pratipadika.as_str())
                     } else {
                         table_context
                     }
                 } else {
                     table_context
                 };
                 self.resolve_identifier(field, new_context);
             },
             crate::ast::Expression::Aggregate { expr, .. } => {
                 self.resolve_expression(expr, table_context);
             },
             crate::ast::Expression::FunctionCall { args, window, .. } => {
                 for arg in args {
                     self.resolve_expression(arg, table_context);
                 }
                 if let Some(w) = window {
                     for p in &mut w.partition_by {
                         self.resolve_expression(p, table_context);
                     }
                     for o in &mut w.order_by {
                         self.resolve_expression(&mut o.expr, table_context);
                     }
                 }
             },
             crate::ast::Expression::Subquery(dql) => {
                 self.resolve_dql(dql);
             },
             crate::ast::Expression::SetOp(set_op) => {
                 for q in &mut set_op.queries {
                     self.resolve_expression(q, table_context);
                 }
             },
             _ => {}
        }
    }

fn normalize_sanskrit_word(text: &str) -> String {
    let mut normalized = text.to_string();
    if normalized.ends_with('ं') {
        normalized.pop();
        normalized.push_str("म्");
    }
    normalized.replace('ण', "न")
}

    fn resolve_identifier(&self, id: &mut Identifier, table_context: Option<&str>) {
        // 1. Check if it's a column
        if let Some((_, _, _)) = self.schema.resolve_column(&id.text, table_context) {
             id.symbol_type = SymbolType::Column;
             
             // Resolve Pratipadika for column (e.g. मूल्येन -> मूल्य)
             // Need to check schema for this mapping
             if let Some(table) = table_context.and_then(|t| self.schema.tables.get(t)) {
                  let mut found_stem = None;
                  if let Some(p) = table.column_form_map.get(&id.text) {
                      found_stem = Some(p.clone());
                  } else {
                      let norm = Self::normalize_sanskrit_word(&id.text);
                      if let Some(p) = table.column_form_map.get(&norm) {
                          found_stem = Some(p.clone());
                      }
                  }

                  if let Some(p) = found_stem {
                       id.pratipadika = p.clone();
                       // Find field info for gender
                       if let Some(col_info) = table.fields.get(&p) {
                           id.vibhakti_form.linga = Some(col_info.gender);
                       }
                       return;
                  }
                  // Try direct pratipadika match
                  if let Some(col_info) = table.fields.get(&id.text) {
                      id.pratipadika = id.text.clone();
                      id.vibhakti_form.linga = Some(col_info.gender);
                      return;
                  }
                  let norm = Self::normalize_sanskrit_word(&id.text);
                  if let Some(col_info) = table.fields.get(&norm) {
                      id.pratipadika = norm;
                      id.vibhakti_form.linga = Some(col_info.gender);
                      return;
                  }
             }
             
             // Fallback: If we resolved it as a column (via global lookup in schema.resolve_column),
             // but failed to get specific table context details above (e.g. ambiguous or no table context passed).
             // Let's try to find it blindly to get gender if possible.
             if id.pratipadika.is_empty() {
                 id.pratipadika = id.text.clone();
             }
             
             // Minimal effort gender lookup if still unknown
             if id.vibhakti_form.linga.is_none() {
                 // Loop all tables to find this column? Expensive but safe for now.
                 for table in self.schema.tables.values() {
                     if let Some(col) = table.fields.get(&id.pratipadika) {
                         id.vibhakti_form.linga = Some(col.gender);
                         break;
                     }
                     // Or check form map?
                     if let Some(p) = table.column_form_map.get(&id.text) {
                          if let Some(col) = table.fields.get(p) {
                             id.vibhakti_form.linga = Some(col.gender);
                             id.pratipadika = p.clone();
                             break;
                          }
                     }
                 }
             }
             return;
        }

        // 2. Check if it's a Table
        if self.schema.resolve_table(&id.text).is_some() 
            || self.schema.resolve_table(&id.pratipadika).is_some() {
             id.symbol_type = SymbolType::Table;
             if let Some(p) = self.schema.resolve_pratipadika(&id.text) {
                 id.pratipadika = p.to_string();
                 if let Some(t) = self.schema.tables.get(p) {
                     id.vibhakti_form.linga = Some(t.gender);
                 }
             } else {
                 if self.schema.tables.contains_key(&id.pratipadika) {
                     // Existing pratipadika matches schema, use it
                     if let Some(t) = self.schema.tables.get(&id.pratipadika) {
                         id.vibhakti_form.linga = Some(t.gender);
                     }
                 } else {
                     // Fallback to text
                     id.pratipadika = id.text.clone();
                     if let Some(t) = self.schema.tables.get(&id.text) {
                         id.vibhakti_form.linga = Some(t.gender);
                     }
                 }
             }
        }
    }
}
