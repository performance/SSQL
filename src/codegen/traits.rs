//! SQL Dialect Adaptor Trait
//!
//! Defines the interface that all SQL dialect adaptors must implement.
//! This enables pluggable code generation for different database engines.

use crate::ast::SqlType;

/// Represents a column reference with optional table qualification
#[derive(Debug, Clone)]
pub struct ColumnRef {
    pub table_alias: Option<String>,
    pub column_name: String,
    pub sql_type: Option<SqlType>,
}

/// Represents a column expression in SELECT (may include alias)
#[derive(Debug, Clone)]
pub struct ColumnExpr {
    pub expr: String,
    pub alias: Option<String>,
}

/// Represents a table reference with auto-generated alias
#[derive(Debug, Clone)]
pub struct TableRef {
    pub table_name: String,
    pub alias: String,
}

/// Join types supported
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Represents a JOIN expression
#[derive(Debug, Clone)]
pub struct JoinExpr {
    pub table: TableRef,
    pub join_type: JoinType,
    pub condition: String,
}

/// Represents ORDER BY direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

/// Represents an ORDER BY expression
#[derive(Debug, Clone)]
pub struct OrderExpr {
    pub column: ColumnRef,
    pub direction: OrderDirection,
}

/// Represents a window function expression
#[derive(Debug, Clone)]
pub struct WindowExpr {
    pub function: String,           // e.g., "RANK"
    pub partition_by: Vec<ColumnRef>,
    pub order_by: Vec<OrderExpr>,
    pub alias: Option<String>,
}

/// Aggregate function types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AggregateFunc {
    Sum,
    Avg,
    Count,
    Max,
    Min,
    Median,
}

/// The core trait that all SQL dialect adaptors must implement
pub trait SqlDialectAdaptor: Send + Sync {
    /// Returns the dialect name (e.g., "sqlite", "postgres")
    fn name(&self) -> &str;
    
    // ========================================
    // Core SQL Generation
    // ========================================
    
    /// Emit a SELECT clause with columns
    fn emit_select(&self, columns: &[ColumnExpr], distinct: bool) -> String;
    
    /// Emit a FROM clause with table and alias
    fn emit_from(&self, table: &TableRef) -> String;
    
    /// Emit a JOIN clause
    fn emit_join(&self, join: &JoinExpr) -> String;
    
    /// Emit a WHERE clause
    fn emit_where(&self, condition: &str) -> String;
    
    /// Emit a GROUP BY clause
    fn emit_group_by(&self, columns: &[ColumnRef]) -> String;
    
    /// Emit an ORDER BY clause
    fn emit_order_by(&self, orders: &[OrderExpr]) -> String;
    
    /// Emit a LIMIT clause
    fn emit_limit(&self, limit: u64) -> String;
    
    /// Emit an OFFSET clause (may vary by dialect)
    fn emit_offset(&self, offset: u64) -> String;
    
    // ========================================
    // Dialect-Specific Features
    // ========================================
    
    /// Emit a window function (RANK() OVER ...)
    fn emit_window_function(&self, window: &WindowExpr) -> String;
    
    /// Emit a CTE (WITH ... AS)
    fn emit_cte(&self, name: &str, query: &str) -> String;
    
    /// Emit an aggregate function with optional CAST for type compatibility
    fn emit_aggregate(&self, func: AggregateFunc, column: &ColumnRef) -> String;
    
    /// Emit a NULL check (IS NULL or IS NOT NULL)
    fn emit_null_check(&self, column: &ColumnRef, is_null: bool) -> String;
    
    /// Emit a type cast
    fn emit_cast(&self, expr: &str, target_type: SqlType) -> String;
    
    /// Emit string concatenation (dialect-specific: || vs CONCAT)
    fn emit_concat(&self, parts: &[&str]) -> String;
    
    /// Emit date/time extraction (month, year, etc.). `sql_type` is the
    /// declared SQL type of `expr` if known (e.g. "text", "date"): a column
    /// declared as plain TEXT holding a "HH:MM:SS"-formatted string (SSQL's
    /// वाक्यम् type) needs an explicit cast under PostgreSQL before
    /// TO_CHAR/EXTRACT will accept it, since PostgreSQL (unlike SQLite's
    /// strftime) has no TEXT overload for those functions.
    fn emit_date_part(&self, part: &str, expr: &str, sql_type: Option<&str>) -> String;
    
    /// Quote an identifier (table/column name) for the dialect
    fn quote_identifier(&self, name: &str) -> String;

    /// SQL type name for an SSQL data type. `वास्तविकः` is a real number and
    /// must behave as one in every dialect (`५५ च २ च इत्यनयोः भागः` is 27.5):
    /// SQLite REAL (not DECIMAL, whose NUMERIC affinity keeps 55 an integer
    /// and makes 55 / 2 = 27) and PostgreSQL NUMERIC (exact, and accepted by
    /// ROUND(x, n) and MOD, which DOUBLE PRECISION is not).
    fn data_type_sql(&self, data_type: &crate::ast::DataType) -> String;

    /// Emit a binary operation
    fn emit_binary_op(&self, left: &str, right: &str, op: crate::ast::BinaryOperator) -> String;

    /// Emit dialect-specific column definition (handles SERIAL vs AUTOINCREMENT).
    /// `resolve_fk` can fail and returns a `Result` rather than swallowing
    /// an unresolvable FK target identifier into a raw, untranslated
    /// literal, which would otherwise produce a
    /// `REFERENCES <raw-sanskrit-stem>(id)` clause that looks valid but is
    /// broken SQL.
    fn emit_column_def(&self, name: &str, data_type: &crate::ast::DataType, constraints: &[crate::ast::Constraint], resolve_fk: &dyn Fn(&crate::ast::Identifier) -> anyhow::Result<(String, String)>) -> anyhow::Result<String>;

    /// Emit CREATE TABLE header
    fn emit_create_table(&self, name: &str, is_temporary: bool, exists_check: bool) -> String;

    /// Emit CREATE VIEW header
    fn emit_create_view(&self, name: &str, is_temporary: bool, exists_check: bool) -> String;

    /// Emit CREATE INDEX
    fn emit_create_index(&self, name: &str, table: &str, columns: &[(String, Option<OrderDirection>)], is_unique: bool, if_not_exists: bool) -> String;

    /// Emit DROP INDEX
    fn emit_drop_index(&self, name: &str, if_exists: bool) -> String;

    /// Emit ALTER TABLE ADD COLUMN
    fn emit_alter_table_add_column(&self, table: &str, column_def: &str) -> String;

    /// Emit ALTER TABLE DROP COLUMN
    fn emit_alter_table_drop_column(&self, table: &str, column: &str) -> String;

    /// Emit ALTER TABLE RENAME COLUMN
    fn emit_alter_table_rename_column(&self, table: &str, old_name: &str, new_name: &str) -> String;

    /// Emit ALTER TABLE RENAME TABLE
    fn emit_alter_table_rename_table(&self, table: &str, new_name: &str) -> String;
}

/// Factory function to get an adaptor by dialect name
pub fn get_adaptor(dialect: &str) -> Box<dyn SqlDialectAdaptor> {
    match dialect.to_lowercase().as_str() {
        "sqlite" => Box::new(super::sqlite::SQLiteAdaptor::new()),
        "postgres" | "postgresql" => Box::new(super::postgres::PostgresAdaptor::new()),
        _ => panic!("Unsupported dialect: {}", dialect),
    }
}
