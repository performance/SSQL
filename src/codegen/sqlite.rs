//! SQLite Dialect Adaptor
//!
//! Implements SQL generation specific to SQLite syntax.

use super::traits::*;
use crate::ast::SqlType;

pub struct SQLiteAdaptor;

impl SQLiteAdaptor {
    pub fn new() -> Self {
        SQLiteAdaptor
    }
}

impl Default for SQLiteAdaptor {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlDialectAdaptor for SQLiteAdaptor {
    fn name(&self) -> &str {
        "sqlite"
    }
    
    fn emit_select(&self, columns: &[ColumnExpr], distinct: bool) -> String {
        let cols = columns.iter()
            .map(|c| match &c.alias {
                Some(alias) => format!("{} AS {}", c.expr, alias),
                None => c.expr.clone(),
            })
            .collect::<Vec<_>>()
            .join(",\n    ");
        
        if distinct {
            format!("SELECT DISTINCT\n    {}", cols)
        } else {
            format!("SELECT\n    {}", cols)
        }
    }
    
    fn emit_from(&self, table: &TableRef) -> String {
        format!("FROM\n    {} {}", table.table_name, table.alias)
    }
    
    fn emit_join(&self, join: &JoinExpr) -> String {
        let join_keyword = match join.join_type {
            JoinType::Inner => "JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN", // Note: SQLite doesn't support RIGHT JOIN natively
            JoinType::Full => "FULL OUTER JOIN",
            JoinType::Cross => "CROSS JOIN",
        };
        if join.join_type == JoinType::Cross {
            // CROSS JOIN is an unconditional Cartesian product - standard SQL
            // has no ON clause for it (any join condition supplied would
            // belong on a subsequent WHERE clause instead).
            format!("{} {} {}", join_keyword, join.table.table_name, join.table.alias)
        } else {
            format!("{} {} {} ON {}",
                join_keyword,
                join.table.table_name,
                join.table.alias,
                join.condition
            )
        }
    }
    
    fn emit_where(&self, condition: &str) -> String {
        format!("WHERE\n    {}", condition)
    }
    
    fn emit_group_by(&self, columns: &[ColumnRef]) -> String {
        let cols = columns.iter()
            .map(|c| match &c.table_alias {
                Some(alias) => format!("{}.{}", alias, c.column_name),
                None => c.column_name.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("GROUP BY\n    {}", cols)
    }
    
    fn emit_order_by(&self, orders: &[OrderExpr]) -> String {
        let cols = orders.iter()
            .map(|o| {
                let col = match &o.column.table_alias {
                    Some(alias) => format!("{}.{}", alias, o.column.column_name),
                    None => o.column.column_name.clone(),
                };
                match o.direction {
                    OrderDirection::Asc => col,
                    OrderDirection::Desc => format!("{} DESC", col),
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("ORDER BY\n    {}", cols)
    }
    
    fn emit_limit(&self, limit: u64) -> String {
        format!("LIMIT {}", limit)
    }
    
    fn emit_offset(&self, offset: u64) -> String {
        format!("OFFSET {}", offset)
    }
    
    fn emit_window_function(&self, window: &WindowExpr) -> String {
        let partition = if window.partition_by.is_empty() {
            String::new()
        } else {
            let cols = window.partition_by.iter()
                .map(|c| match &c.table_alias {
                    Some(a) => format!("{}.{}", a, c.column_name),
                    None => c.column_name.clone(),
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("PARTITION BY {}", cols)
        };
        
        let order = if window.order_by.is_empty() {
            String::new()
        } else {
            let cols = window.order_by.iter()
                .map(|o| {
                    let col = match &o.column.table_alias {
                        Some(a) => format!("{}.{}", a, o.column.column_name),
                        None => o.column.column_name.clone(),
                    };
                    match o.direction {
                        OrderDirection::Asc => col,
                        OrderDirection::Desc => format!("{} DESC", col),
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("ORDER BY {}", cols)
        };
        
        let over_clause = [partition, order]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        
        format!("{}() OVER ({})", window.function, over_clause)
    }
    
    fn emit_cte(&self, name: &str, query: &str) -> String {
        format!("WITH {} AS (\n{}\n)", name, query)
    }
    
    fn emit_aggregate(&self, func: AggregateFunc, column: &ColumnRef) -> String {
        let col = match &column.table_alias {
            Some(a) => format!("{}.{}", a, column.column_name),
            None => column.column_name.clone(),
        };
        
        let func_name = match func {
            AggregateFunc::Sum => "SUM",
            AggregateFunc::Avg => "AVG",
            AggregateFunc::Count => "COUNT",
            AggregateFunc::Max => "MAX",
            AggregateFunc::Min => "MIN",
            AggregateFunc::Median => "MEDIAN", // Note: SQLite doesn't have MEDIAN natively
        };
        
        format!("{}({})", func_name, col)
    }
    
    fn emit_null_check(&self, column: &ColumnRef, is_null: bool) -> String {
        let col = match &column.table_alias {
            Some(a) => format!("{}.{}", a, column.column_name),
            None => column.column_name.clone(),
        };
        
        if is_null {
            format!("{} IS NULL", col)
        } else {
            format!("{} IS NOT NULL", col)
        }
    }
    
    fn emit_cast(&self, expr: &str, target_type: SqlType) -> String {
        let type_str = match target_type {
            SqlType::Integer => "INTEGER",
            SqlType::Real => "REAL",
            SqlType::Text => "TEXT",
            SqlType::Blob => "BLOB",
            SqlType::Numeric => "NUMERIC",
            SqlType::Boolean => "INTEGER", // SQLite uses INTEGER for booleans
            SqlType::Date => "TEXT",
            SqlType::Timestamp => "TEXT",
        };
        format!("CAST({} AS {})", expr, type_str)
    }
    
    fn emit_concat(&self, parts: &[&str]) -> String {
        parts.join(" || ")
    }
    
    fn emit_date_part(&self, part: &str, expr: &str, _sql_type: Option<&str>) -> String {
        // strftime() already accepts plain TEXT time/date strings directly,
        // so SQLite never needs the cast PostgreSQL requires.
        let format_str = match part.to_lowercase().as_str() {
            "year" => "%Y",
            "month" => "%m",
            "day" => "%d",
            "hour" => "%H",
            "minute" => "%M",
            "second" => "%S",
            "dayname" => "%w", // 0-6 Sunday-Saturday
            // SSQL Default: assume standard ISO date string parsing
            _ => return format!("strftime('%Y-%m-%d', {})", expr),
        };
        // A date part is a number (`वर्षम् २०२३ इत्यस्मात् अधिकम्`), as
        // PostgreSQL's EXTRACT is: strftime's text compared with a number is
        // always the greater in SQLite, so `'2023' > 2099` held for every row.
        format!("CAST(strftime('{}', {}) AS INTEGER)", format_str, expr)
    }
    
    fn quote_identifier(&self, name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }

    fn emit_binary_op(&self, left: &str, right: &str, op: crate::ast::BinaryOperator) -> String {
        if right == "NULL" {
            match op {
                crate::ast::BinaryOperator::Equal => return format!("{} IS NULL", left),
                crate::ast::BinaryOperator::NotEqual => return format!("{} IS NOT NULL", left),
                _ => {}
            }
        }
        let op_str = match op {
            crate::ast::BinaryOperator::Equal => "=",
            crate::ast::BinaryOperator::NotEqual => "<>",
            crate::ast::BinaryOperator::Greater => ">",
            crate::ast::BinaryOperator::Less => "<",
            crate::ast::BinaryOperator::GreaterEqual => ">=",
            crate::ast::BinaryOperator::LessEqual => "<=",
            crate::ast::BinaryOperator::Like => "LIKE",
            crate::ast::BinaryOperator::NotLike => "NOT LIKE",
            crate::ast::BinaryOperator::Regexp => "REGEXP",
            crate::ast::BinaryOperator::Add => "+",
            crate::ast::BinaryOperator::Subtract => "-",
            crate::ast::BinaryOperator::Multiply => "*",
            crate::ast::BinaryOperator::Divide => "/",
            crate::ast::BinaryOperator::In => "IN",
            crate::ast::BinaryOperator::NotIn => "NOT IN",
        };
        format!("{} {} {}", left, op_str, right)
    }

    fn data_type_sql(&self, data_type: &crate::ast::DataType) -> String {
        match data_type {
            crate::ast::DataType::Integer => "INTEGER".to_string(),
            crate::ast::DataType::Text => "TEXT".to_string(),
            crate::ast::DataType::Decimal => "REAL".to_string(),
            crate::ast::DataType::Date => "DATE".to_string(),
            crate::ast::DataType::Boolean => "BOOLEAN".to_string(),
            crate::ast::DataType::Custom(s) => s.clone(),
        }
    }

    fn emit_column_def(&self, name: &str, data_type: &crate::ast::DataType, constraints: &[crate::ast::Constraint], resolve_fk: &dyn Fn(&crate::ast::Identifier) -> anyhow::Result<(String, String)>) -> anyhow::Result<String> {
        let mut col_sql = format!("{} {}", name, self.data_type_sql(data_type));
        let has_pk = constraints.iter().any(|c| matches!(c, crate::ast::Constraint::PrimaryKey));
        let has_auto = constraints.iter().any(|c| matches!(c, crate::ast::Constraint::AutoIncrement));

        if has_pk {
            col_sql.push_str(" PRIMARY KEY");
        }
        if has_auto {
            col_sql.push_str(" AUTOINCREMENT");
        }

        for c in constraints {
            match c {
                crate::ast::Constraint::PrimaryKey | crate::ast::Constraint::AutoIncrement => {},
                crate::ast::Constraint::NotNull => col_sql.push_str(" NOT NULL"),
                crate::ast::Constraint::Null => col_sql.push_str(" NULL"),
                crate::ast::Constraint::Unique => col_sql.push_str(" UNIQUE"),
                crate::ast::Constraint::ForeignKey(ident) => {
                    let (ref_table, ref_pk) = resolve_fk(ident)?;
                    col_sql.push_str(&format!(" REFERENCES {}({})", ref_table, ref_pk));
                }
            }
        }
        Ok(col_sql)
    }

    fn emit_create_table(&self, name: &str, is_temporary: bool, exists_check: bool) -> String {
        let temp_str = if is_temporary { "TEMPORARY " } else { "" };
        let exists_str = if exists_check { "IF NOT EXISTS " } else { "" };
        format!("CREATE {}{}TABLE {}", temp_str, exists_str, name)
    }

    fn emit_create_view(&self, name: &str, is_temporary: bool, exists_check: bool) -> String {
        let temp_str = if is_temporary { "TEMPORARY " } else { "" };
        let exists_str = if exists_check { "IF NOT EXISTS " } else { "" };
        format!("CREATE {}{}VIEW {}", temp_str, exists_str, name)
    }

    fn emit_create_index(&self, name: &str, table: &str, columns: &[(String, Option<OrderDirection>)], is_unique: bool, if_not_exists: bool) -> String {
        let unique_str = if is_unique { "UNIQUE " } else { "" };
        let exists_str = if if_not_exists { "IF NOT EXISTS " } else { "" };
        let col_strs: Vec<String> = columns.iter().map(|(col, dir)| {
            let dir_str = match dir {
                Some(OrderDirection::Asc) => " ASC",
                Some(OrderDirection::Desc) => " DESC",
                None => "",
            };
            format!("{}{}", col, dir_str)
        }).collect();
        format!("CREATE {}INDEX {}{} ON {} ({})", unique_str, exists_str, name, table, col_strs.join(", "))
    }

    fn emit_drop_index(&self, name: &str, if_exists: bool) -> String {
        let exists_str = if if_exists { "IF EXISTS " } else { "" };
        format!("DROP INDEX {}{}", exists_str, name)
    }

    fn emit_alter_table_add_column(&self, table: &str, column_def: &str) -> String {
        format!("ALTER TABLE {} ADD COLUMN {}", table, column_def)
    }

    fn emit_alter_table_drop_column(&self, table: &str, column: &str) -> String {
        format!("ALTER TABLE {} DROP COLUMN {}", table, column)
    }

    fn emit_alter_table_rename_column(&self, table: &str, old_name: &str, new_name: &str) -> String {
        format!("ALTER TABLE {} RENAME COLUMN {} TO {}", table, old_name, new_name)
    }

    fn emit_alter_table_rename_table(&self, table: &str, new_name: &str) -> String {
        format!("ALTER TABLE {} RENAME TO {}", table, new_name)
    }
}
