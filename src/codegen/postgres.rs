//! PostgreSQL Dialect Adaptor
//!
//! Implements SQL generation specific to PostgreSQL syntax.

use super::traits::*;
use crate::ast::SqlType;

pub struct PostgresAdaptor;

impl PostgresAdaptor {
    pub fn new() -> Self {
        PostgresAdaptor
    }
}

impl Default for PostgresAdaptor {
    fn default() -> Self {
        Self::new()
    }
}

impl SqlDialectAdaptor for PostgresAdaptor {
    fn name(&self) -> &str {
        "postgres"
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
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL OUTER JOIN",
            JoinType::Cross => "CROSS JOIN",
        };
        if join.join_type == JoinType::Cross {
            // CROSS JOIN is an unconditional Cartesian product - standard SQL
            // has no ON clause for it (any join condition supplied would
            // belong on a subsequent WHERE clause instead).
            return format!("{} {} {}", join_keyword, join.table.table_name, join.table.alias);
        }
        format!("{} {} {} ON {}",
            join_keyword,
            join.table.table_name,
            join.table.alias,
            join.condition
        )
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
        
        // PostgreSQL: cast numeric aggregates to DOUBLE PRECISION (not REAL) for
        // sqlx::Any compatibility. REAL is Postgres's single-precision float4;
        // SQLite has no single-precision type at all - its REAL storage class is
        // always an 8-byte IEEE double - so casting to REAL here silently lost
        // precision relative to SQLite's output for large-enough aggregates.
        // DOUBLE PRECISION (float8) matches SQLite's actual precision.
        match func {
            AggregateFunc::Sum => format!("CAST(SUM({}) AS DOUBLE PRECISION)", col),
            AggregateFunc::Avg => format!("CAST(AVG({}) AS DOUBLE PRECISION)", col),
            AggregateFunc::Count => format!("COUNT({})", col),
            AggregateFunc::Max => format!("MAX({})", col),
            AggregateFunc::Min => format!("MIN({})", col),
            AggregateFunc::Median => {
                // PostgreSQL uses PERCENTILE_CONT for median
                format!("PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY {})", col)
            }
        }
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
            SqlType::Blob => "BYTEA",
            SqlType::Numeric => "NUMERIC",
            // SSQL writes truth as १ / ०, which a BOOLEAN column refuses.
            SqlType::Boolean => "SMALLINT",
            SqlType::Date => "DATE",
            SqlType::Timestamp => "TIMESTAMP",
        };
        format!("CAST({} AS {})", expr, type_str)
    }
    
    fn emit_concat(&self, parts: &[&str]) -> String {
        // PostgreSQL supports both || and CONCAT, using || for consistency
        parts.join(" || ")
    }
    
    fn emit_date_part(&self, part: &str, expr: &str, sql_type: Option<&str>) -> String {
        // A date part is an integer on both dialects (SQLite casts
        // strftime's text). EXTRACT has no overload for plain TEXT - unlike
        // SQLite's strftime, which parses "HH:MM:SS"/"YYYY-MM-DD" text
        // directly - so a column declared TEXT (SSQL's वाक्यम्) is cast to a
        // real temporal type first.
        let is_text = sql_type.map(|t| t.eq_ignore_ascii_case("text")).unwrap_or(false);
        let is_date = sql_type.map(|t| t.eq_ignore_ascii_case("date")).unwrap_or(false);
        let part = part.to_lowercase();
        let time_part = matches!(part.as_str(), "hour" | "minute" | "second");
        let cast_expr = if is_text {
            // A time of date-only text is midnight, as in strftime; `::time`
            // alone refuses `2024-02-29`, and `::timestamp` refuses `10:30:00`.
            if time_part { format!("({0} || CASE WHEN {0} LIKE '%:%' THEN '' ELSE ' 00:00:00' END)::time", expr) } else { format!("{}::date", expr) }
        } else if is_date && time_part {
            // EXTRACT(HOUR FROM date) is an error; a date is midnight.
            format!("{}::timestamp", expr)
        } else {
            expr.to_string()
        };
        let field = match part.as_str() {
            "year" | "month" | "day" | "hour" | "minute" => part.to_uppercase(),
            // Whole seconds, as strftime('%S'): FLOOR, since a cast rounds.
            "second" => return format!("CAST(FLOOR(EXTRACT(SECOND FROM {})) AS INTEGER)", cast_expr),
            // Weekday 0-6, Sunday = 0, as strftime('%w').
            "dayname" => "DOW".to_string(),
            _ => return format!("TO_CHAR({}, 'YYYY-MM-DD')", cast_expr),
        };
        format!("CAST(EXTRACT({} FROM {}) AS INTEGER)", field, cast_expr)
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
        // Division by zero is NULL, as on SQLite, not an error.
        if matches!(op, crate::ast::BinaryOperator::Divide) {
            return format!("{} / NULLIF({}, 0)", left, right);
        }
        // A backslash is an ordinary character in the pattern, as in
        // SQLite's LIKE; PostgreSQL's escapes the next character.
        if matches!(op, crate::ast::BinaryOperator::Like | crate::ast::BinaryOperator::NotLike) {
            let not = if matches!(op, crate::ast::BinaryOperator::NotLike) { "NOT " } else { "" };
            return format!("{} {}ILIKE {} ESCAPE ''", left, not, right);
        }
        let op_str = match op {
            crate::ast::BinaryOperator::Equal => "=",
            crate::ast::BinaryOperator::NotEqual => "<>",
            crate::ast::BinaryOperator::Greater => ">",
            crate::ast::BinaryOperator::Less => "<",
            crate::ast::BinaryOperator::GreaterEqual => ">=",
            crate::ast::BinaryOperator::LessEqual => "<=",
            // SQLite's LIKE ignores case; ILIKE is the same match here.
            crate::ast::BinaryOperator::Like => "ILIKE",
            crate::ast::BinaryOperator::NotLike => "NOT ILIKE",
            crate::ast::BinaryOperator::Regexp => "~", // PostgreSQL regex operator
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
            // 64-bit, as SQLite's INTEGER; PostgreSQL's INTEGER is 32-bit.
            crate::ast::DataType::Integer => "BIGINT".to_string(),
            crate::ast::DataType::Text => "TEXT".to_string(),
            crate::ast::DataType::Decimal => "NUMERIC".to_string(),
            crate::ast::DataType::Date => "DATE".to_string(),
            crate::ast::DataType::Boolean => "SMALLINT".to_string(),
            crate::ast::DataType::Custom(s) => s.clone(),
        }
    }

    fn emit_column_def(&self, name: &str, data_type: &crate::ast::DataType, constraints: &[crate::ast::Constraint], resolve_fk: &dyn Fn(&crate::ast::Identifier) -> anyhow::Result<(String, String)>) -> anyhow::Result<String> {
        let has_autoincrement = constraints.iter().any(|c| matches!(c, crate::ast::Constraint::AutoIncrement));
        let mut col_sql = if has_autoincrement && data_type == &crate::ast::DataType::Integer {
            format!("{} BIGSERIAL", name)
        } else {
            format!("{} {}", name, self.data_type_sql(data_type))
        };
        // Text compares and sorts byte by byte, as in SQLite, wherever it is
        // read (MIN/MAX, `<`, an ORDER BY of a function), not by the
        // database's locale.
        if data_type == &crate::ast::DataType::Text {
            col_sql.push_str(" COLLATE \"C\"");
        }
        for c in constraints {
            match c {
                crate::ast::Constraint::PrimaryKey => col_sql.push_str(" PRIMARY KEY"),
                crate::ast::Constraint::NotNull => col_sql.push_str(" NOT NULL"),
                crate::ast::Constraint::Null => col_sql.push_str(" NULL"),
                crate::ast::Constraint::Unique => col_sql.push_str(" UNIQUE"),
                crate::ast::Constraint::AutoIncrement => {}, // Omit for Postgres BIGSERIAL
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
        let op_str = if exists_check { "CREATE OR REPLACE " } else { "CREATE " };
        format!("{}{}VIEW {}", op_str, temp_str, name)
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
