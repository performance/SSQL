//! # Abstract Syntax Tree (AST)
//!
//! AST representation for SSQL programs

use crate::types::{SymbolType, Vachana, Vibhakti, VibhaktiForm};

/// Root AST node
#[derive(Debug, Clone)]
pub enum AstNode {
    /// A complete query batch (multiple statements)
    QueryBatch {
        parameters: Vec<ParameterDecl>,
        statements: Vec<Statement>,
    },
    
    /// Individual statement types
    Statement(Statement),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// DQL: Data Query Language (SELECT)
    Dql(DqlStatement),
    
    /// Set Operations (UNION, etc.)
    SetOp(SetOpStatement),
    
    /// DML: Data Manipulation (INSERT/UPDATE/DELETE)
    Dml(DmlStatement),
    
    /// DDL: Data Definition (CREATE/DROP)
    Ddl(DdlStatement),
    
    /// DCL: Data Control (GRANT/REVOKE)
    Dcl(DclStatement),
    
    /// Variable declaration
    VarDecl {
        name: Identifier,
        value: Box<Expression>,
        is_recursive: bool,
        cycle_col: Option<Identifier>,
    },
    
    /// EXPLAIN PLAN
    Explain(Box<Statement>),
    
    /// TCL: Transaction Control Language (BEGIN/COMMIT/ROLLBACK/SAVEPOINT)
    Tcl(TclStatement),
}

#[derive(Debug, Clone)]
pub struct ParameterDecl {
    pub name: Identifier,
}

/// Identifier with vibhakti information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub text: String,
    pub vibhakti_form: VibhaktiForm,
    pub pratipadika: String,
    pub symbol_type: SymbolType,
    pub span: Option<(usize, usize)>,
}

/// DQL (SELECT) statement
#[derive(Debug, Clone, PartialEq)]
pub struct DqlStatement {
    pub source: SourceClause,
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<WhereClause>,
    pub group_by: Option<GroupByClause>,
    pub having: Option<HavingClause>,
    pub select: SelectClause,
    pub order_by: Option<OrderByClause>,
    pub limit: Option<LimitClause>,
    pub verb_token: Option<Identifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceClause {
    pub table: Identifier,  // Panchami form
    /// A derived table (`यत् <query> इति आश्रित्य`). Present only between
    /// parsing and `desugar_derived_tables`, which turns it into an
    /// anonymous CTE and points `table` at it.
    pub derived: Option<Box<DqlStatement>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JoinClause {
    pub table: Identifier,  // Tritiya form
    pub condition: Option<Box<Expression>>,
    pub join_type: Option<JoinType>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    Left,    // वाममेलनम्
    Right,   // दक्षिणमेलनम्
    Full,    // पूर्णमेलनम्
    Cross,   // सर्वमेलनेन
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    pub condition: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupByClause {
    pub expressions: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HavingClause {
    pub condition: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectItem {
    pub expr: Expression,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectClause {
    pub columns: Vec<SelectItem>,
    pub distinct: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderByClause {
    pub items: Vec<OrderExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LimitClause {
    pub count: Box<Expression>,
    pub offset: Option<Box<Expression>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetOperation {
    Union,
    Intersect, // Future
    Except,    // Future
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetOpStatement {
    /// Queries to combine (can be DQL or Identifiers/CTEs)
    pub queries: Vec<Expression>,
    pub op: SetOperation,
    pub use_all: bool,
}

/// TCL: Transaction Control Language (BEGIN/COMMIT/ROLLBACK/SAVEPOINT)
#[derive(Debug, Clone, PartialEq)]
pub enum TclStatement {
    Begin,
    Commit,
    Rollback { savepoint: Option<Identifier> },
    Savepoint { name: Identifier },
}

/// DML statements
#[derive(Debug, Clone, PartialEq)]
pub enum DmlStatement {
    Insert {
        table: Identifier,  // Saptami form
        values: Box<Expression>,
        /// निवेश्यताम् / निवेश्येताम् / निवेश्यन्ताम्, agreeing with the column count
        verb: String,
    },
    InsertSelect {
        table: Identifier,  // Saptami form
        columns: Option<Vec<Identifier>>,
        query: Box<DqlStatement>,
        verb: String,
    },
    Update {
        table: Identifier,  // Saptami form
        /// None only for the explicit update-all form `<table> सर्वत्र …`.
        where_clause: Option<WhereClause>,
        set_clause: SetClause,
        /// परिवर्त्यताम् / परिवर्त्येताम् / परिवर्त्यन्ताम्, agreeing with the assignment count
        verb: String,
    },
    Delete {
        table: Identifier,  // Panchami form
        /// None only for the explicit delete-all form `<table> सर्वम् निष्कास्यताम्`.
        where_clause: Option<WhereClause>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetClause {
    pub assignments: Box<Expression>,
}

/// SQL data types for type-safe column mappings
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataType {
    Integer,
    Text,
    Decimal,
    Date,
    Boolean,
    Custom(String),
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Integer => write!(f, "INTEGER"),
            DataType::Text => write!(f, "TEXT"),
            DataType::Decimal => write!(f, "DECIMAL"),
            DataType::Date => write!(f, "DATE"),
            DataType::Boolean => write!(f, "BOOLEAN"),
            DataType::Custom(s) => write!(f, "{}", s),
        }
    }
}

// ... existing code ...

#[derive(Debug, Clone, PartialEq)]
pub enum DdlStatement {
    Create(CreateStatement),
    CreateUdf(CreateUdfStatement),
    Drop(DropStatement),
    CreateIndex(CreateIndexStatement),
    DropIndex(DropIndexStatement),
    AlterTable(AlterTableStatement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UdfParam {
    pub name: Identifier,
    pub data_type: DataType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateUdfStatement {
    pub name: Identifier,
    pub sql_name: Option<String>,
    pub return_type: DataType,
    pub params: Vec<UdfParam>,
    pub param_verb: String,
    pub body: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateIndexStatement {
    pub name: Identifier,
    pub table_name: Identifier,
    pub columns: Vec<IndexColumn>,
    pub is_unique: bool,
    pub if_not_exists: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexColumn {
    pub name: Identifier,
    pub direction: Option<OrderDirection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DropIndexStatement {
    pub name: Identifier,
    pub is_exists_check: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlterAction {
    AddColumn(ColumnDef),
    DropColumn(Identifier),
    RenameColumn { old_name: Identifier, new_name: Identifier },
    RenameTable(Identifier),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlterTableStatement {
    pub table_name: Identifier,
    pub action: AlterAction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DropStatement {
    pub name: Identifier,
    pub type_name: Identifier,
    pub is_exists_check: bool,
}

// CreateStatement is already defined at bottom, move it here or leave it?
// We will rely on existing definition but ensure it uses DataType properly.

// Expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    // ... existing variants ...
    Identifier(Identifier),
    Literal(Literal),
    BinaryOp { left: Box<Expression>, op: BinaryOperator, right: Box<Expression> },
    LogicalOp { left: Box<Expression>, op: LogicalOperator, right: Box<Expression> },
    FieldAccess { object: Box<Expression>, field: Identifier },
    Case { cases: Vec<CaseArm> },
    FunctionCall {
        name: String,
        args: Vec<Expression>,
        window: Option<WindowSpec>,
        pronoun_token: Option<Identifier>,
    },
    Aggregate {
        expr: Box<Expression>,
        func: AggFunc,
        distinct: bool,
        distinct_token: Option<Identifier>,
        pronoun_token: Option<Identifier>,
    },
    Subquery(Box<DqlStatement>),
    SetOp(Box<SetOpStatement>),
    Wildcard,
    
    // NEW: Cast
    Cast {
        expr: Box<Expression>,
        data_type: DataType,
    },

    // NEW: Sanskrit Comparison (for Grammatical validation, lowered to BinaryOp before codegen)
    SanskritComparison {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
        pronoun: Option<Identifier>,
        adjective: Option<Identifier>,
    },

    // NEW: Between (e.g. price BETWEEN 20 AND 50)
    Between {
        expr: Box<Expression>,
        low: Box<Expression>,
        high: Box<Expression>,
        /// `… इत्यनयोः मध्ये नास्ति` (NOT BETWEEN)
        negated: bool,
    },

    // NEW: Exists / Not Exists
    Exists {
        subquery: Box<DqlStatement>,
        negated: bool,
    },

    // Named UDF call argument: <param> इत्यस्य मूल्यम् <value> इति
    NamedArg {
        param: Identifier,
        value: Box<Expression>,
    },
}

/// DCL statements
#[derive(Debug, Clone, PartialEq)]
pub enum DclStatement {
    Grant {
        recipient: Identifier,  // Chaturthi form
        object: Identifier,     // Saptami form
        rights: Box<Expression>,
    },
    Revoke {
        target: Identifier,     // Panchami form
        object: Identifier,     // Saptami form
        rights: Box<Expression>,
    },
}



#[derive(Debug, Clone, PartialEq)]
pub struct WindowSpec {
    pub partition_by: Vec<Expression>,
    pub order_by: Vec<OrderExpr>,
    pub frame: Option<WindowFrame>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowFrame {
    pub units: FrameUnits,
    pub start: FrameBound,
    pub end: Option<FrameBound>,
    pub exclude: Option<FrameExclude>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum FrameUnits {
    Rows,
    Range,
    Groups,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FrameBound {
    UnboundedPreceding,
    Preceding(Box<Expression>),
    CurrentRow,
    Following(Box<Expression>),
    UnboundedFollowing,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum FrameExclude {
    CurrentRow,
    Group,
    Ties,
    NoOthers,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderExpr {
    pub expr: Box<Expression>,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(String),
    String(String),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Equal,        // इत्यनेन समः अस्ति
    NotEqual,     // इत्यनेन समः नास्ति
    Greater,      // इत्यस्मात् अधिकः अस्ति
    Less,         // इत्यस्मात् न्यूनः अस्ति
    GreaterEqual, // इत्यस्मात् न्यूनः नास्ति ("is not less than")
    LessEqual,    // इत्यस्मात् अधिकः नास्ति ("is not more than")
    Like,         // इत्यनेन सदृशः अस्ति
    NotLike,      // इत्यनेन सदृशः नास्ति
    Regexp,       // अनुरूपेण अस्ति
    In,           // इत्येषु अस्ति (IN)
    NotIn,        // इत्येषु नास्ति (NOT IN)
    Add,         // योगः (Sum)
    Subtract,    // अन्तरम् / अवशिष्टम् (Difference/Remainder)
    Multiply,    // गुणनम् (Product)
    Divide,      // भागफलम् (Division)
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogicalOperator {
    And,  // च
    Or,   // वा
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseArm {
    pub condition: Option<Box<Expression>>,  // None for ANYASMIN (else)
    pub result: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AggregateFunc {
    Sum,      // योगः
    Diff,     // अन्तरम्
    Product,  // गुणनम्
    Quotient, // भागफलम्
}

/// SQL data types for type-safe column mappings
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SqlType {
    Integer,
    Real,
    Text,
    Blob,
    Numeric,
    Boolean,
    Date,
    Timestamp,
}

/// Aggregate function types (used by CodeGen)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AggFunc {
    Sum,
    Avg,
    Count,
    Max,
    Min,
}

impl AggFunc {
    /// The SQL aggregate, the same whether used plainly or over a window.
    pub fn sql_name(&self) -> &'static str {
        match self {
            AggFunc::Sum => "SUM",
            AggFunc::Avg => "AVG",
            AggFunc::Count => "COUNT",
            AggFunc::Max => "MAX",
            AggFunc::Min => "MIN",
        }
    }
}

// ============================================================================
// Window Function AST Nodes
// ============================================================================

/// Window function expression
#[derive(Debug, Clone)]
pub struct WindowFunction {
    pub function: WindowFunctionType,
    pub partition_by: Vec<Expression>,
    pub order_by: Vec<OrderExpr>,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowFunctionType {
    Rank,       // श्रेणी
    DenseRank,
    RowNumber,
    Lead,
    Lag,
    Greater,
    Less,
    Equal,
    Like, // LIKE operator
}

/// Order by expression with direction

#[derive(Debug, Clone, PartialEq)]
pub struct CreateStatement {
    pub name: Identifier,
    pub sql_name: Option<String>,
    pub type_name: Identifier, // "View", "Table" etc.
    pub is_temporary: bool,
    pub config: Vec<Identifier>, // e.g. अन्वेषणप्रधानम्
    pub pratipadika: String,
    pub gender: crate::types::Linga,
    pub number: crate::types::Vachana,
    pub columns: Vec<ColumnDef>,
    pub query: Option<Box<Statement>>, // For CREATE VIEW AS ...
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    pub name: Identifier,
    pub data_type: DataType,
    pub sql_name: Option<String>,
    pub pratipadika: String,
    pub gender: crate::types::Linga,
    pub number: crate::types::Vachana,
    pub constraints: Vec<Constraint>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    PrimaryKey,
    NotNull,
    Null,
    Unique,
    AutoIncrement,
    ForeignKey(Identifier),
}

/// Common Table Expression (CTE) - for WITH ... AS (...)
#[derive(Debug, Clone)]
pub struct CteDefinition {
    pub name: Identifier,
    pub query: Box<DqlStatement>,
    pub alias: Option<String>,
}

/// Enhanced join clause with table alias (for self-joins)
#[derive(Debug, Clone)]
pub struct JoinClauseV2 {
    pub table: Identifier,
    pub alias: Option<Identifier>,  // For self-joins: विक्रयाधिकारिणः इति प्रबन्धकाः
    pub condition: Option<Box<Expression>>,
    pub join_type: JoinType,
}

impl Default for JoinType {
    fn default() -> Self {
        JoinType::Left // Default to INNER
    }
}

/// Table reference with auto-generated alias
#[derive(Debug, Clone)]
pub struct TableRefV2 {
    pub table_name: String,
    pub alias: String,
    pub resolved_from: String,  // Original Sanskrit name
}

/// Column reference with resolved table and type
#[derive(Debug, Clone)]
pub struct ColumnRefV2 {
    pub column_name: String,
    pub table_alias: Option<String>,
    pub sql_type: Option<SqlType>,
    pub resolved_from: String,  // Original Sanskrit name
}

/// Resolved SELECT column (after semantic analysis)
#[derive(Debug, Clone)]
pub struct SelectColumnV2 {
    pub expr: Expression,
    pub alias: Option<String>,
    pub sql_type: Option<SqlType>,
}

impl AstNode {
    /// Turn every derived table (`यत् <query> इति आश्रित्य`) into an anonymous
    /// CTE placed immediately before the statement that uses it, and point
    /// the query's source at that CTE. A derived table means exactly what a
    /// CTE means (it cannot refer to the enclosing query), so all CTE
    /// machinery - column registration for resolution and agreement, and
    /// codegen - applies unchanged. Nested derived tables are handled
    /// innermost first.
    pub fn desugar_derived_tables(&mut self) {
        let statements = match self {
            AstNode::QueryBatch { statements, .. } => std::mem::take(statements),
            AstNode::Statement(stmt) => vec![stmt.clone()],
        };
        let mut counter = 0usize;
        let mut out = Vec::with_capacity(statements.len());
        for mut stmt in statements {
            let mut ctes = Vec::new();
            Self::visit_dqls_in_statement(&mut stmt, &mut |dql| {
                if let Some(query) = dql.source.derived.take() {
                    counter += 1;
                    let name = format!("derived_{}", counter);
                    ctes.push(Statement::VarDecl {
                        name: Identifier {
                            text: name.clone(),
                            vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Bahu),
                            pratipadika: name.clone(),
                            symbol_type: SymbolType::Table,
                            span: dql.source.table.span,
                        },
                        value: Box::new(Expression::Subquery(query)),
                        is_recursive: false,
                        cycle_col: None,
                    });
                    dql.source.table.text = name.clone();
                    dql.source.table.pratipadika = name;
                }
            });
            out.extend(ctes);
            out.push(stmt);
        }
        match self {
            AstNode::QueryBatch { statements, .. } => *statements = out,
            AstNode::Statement(_) => {
                *self = if out.len() == 1 {
                    AstNode::Statement(out.pop().unwrap())
                } else {
                    AstNode::QueryBatch { parameters: vec![], statements: out }
                }
            }
        }
    }

    /// Call `f` on every query in `stmt`, innermost first.
    fn visit_dqls_in_statement(stmt: &mut Statement, f: &mut dyn FnMut(&mut DqlStatement)) {
        match stmt {
            Statement::Dql(dql) => Self::visit_dql(dql, f),
            Statement::SetOp(set_op) => {
                for q in &mut set_op.queries {
                    Self::visit_dqls_in_expression(q, f);
                }
            }
            Statement::VarDecl { value, .. } => Self::visit_dqls_in_expression(value, f),
            Statement::Explain(inner) => Self::visit_dqls_in_statement(inner, f),
            Statement::Dml(DmlStatement::InsertSelect { query, .. }) => Self::visit_dql(query, f),
            _ => {}
        }
    }

    fn visit_dql(dql: &mut DqlStatement, f: &mut dyn FnMut(&mut DqlStatement)) {
        if let Some(derived) = &mut dql.source.derived {
            Self::visit_dql(derived, f);
        }
        for col in &mut dql.select.columns {
            Self::visit_dqls_in_expression(&mut col.expr, f);
        }
        if let Some(w) = &mut dql.where_clause {
            Self::visit_dqls_in_expression(&mut w.condition, f);
        }
        if let Some(h) = &mut dql.having {
            Self::visit_dqls_in_expression(&mut h.condition, f);
        }
        f(dql);
    }

    fn visit_dqls_in_expression(expr: &mut Expression, f: &mut dyn FnMut(&mut DqlStatement)) {
        match expr {
            Expression::Subquery(dql) | Expression::Exists { subquery: dql, .. } => Self::visit_dql(dql, f),
            Expression::SetOp(set_op) => {
                for q in &mut set_op.queries {
                    Self::visit_dqls_in_expression(q, f);
                }
            }
            Expression::SanskritComparison { left, right, .. }
            | Expression::BinaryOp { left, right, .. }
            | Expression::LogicalOp { left, right, .. } => {
                Self::visit_dqls_in_expression(left, f);
                Self::visit_dqls_in_expression(right, f);
            }
            _ => {}
        }
    }

    /// Recursively lower all SanskritComparison transient nodes to standard BinaryOp nodes in the AST.
    pub fn lower_ast(&mut self) {
        match self {
            AstNode::QueryBatch { statements, .. } => {
                for stmt in statements {
                    Self::lower_statement(stmt);
                }
            }
            AstNode::Statement(stmt) => {
                Self::lower_statement(stmt);
            }
        }
    }

    fn lower_statement(stmt: &mut Statement) {
        match stmt {
            Statement::Dql(dql) => Self::lower_dql(dql),
            Statement::SetOp(set_op) => {
                for q in &mut set_op.queries {
                    Self::lower_expression(q);
                }
            }
            Statement::Dml(dml) => Self::lower_dml(dml),
            Statement::Ddl(DdlStatement::CreateUdf(udf)) => {
                Self::lower_expression(&mut udf.body);
            }
            // A view's (or CREATE TABLE … AS's) query.
            Statement::Ddl(DdlStatement::Create(create)) => {
                if let Some(query) = &mut create.query {
                    Self::lower_statement(query);
                }
            }
            Statement::VarDecl { value, .. } => {
                Self::lower_expression(value);
            }
            Statement::Explain(inner) => {
                Self::lower_statement(inner);
            }
            _ => {}
        }
    }

    fn lower_dml(dml: &mut DmlStatement) {
        match dml {
            DmlStatement::Insert { values, .. } => {
                Self::lower_expression(values);
            }
            DmlStatement::Update { where_clause, set_clause, .. } => {
                if let Some(w) = where_clause {
                    Self::lower_expression(&mut w.condition);
                }
                Self::lower_expression(&mut set_clause.assignments);
            }
            DmlStatement::Delete { where_clause, .. } => {
                if let Some(w) = where_clause {
                    Self::lower_expression(&mut w.condition);
                }
            }
            DmlStatement::InsertSelect { query, .. } => {
                Self::lower_dql(query);
            }
        }
    }

    fn lower_dql(dql: &mut DqlStatement) {
        for col in &mut dql.select.columns {
            Self::lower_expression(&mut col.expr);
        }
        for join in &mut dql.joins {
            if let Some(cond) = &mut join.condition {
                Self::lower_expression(cond);
            }
        }
        if let Some(w) = &mut dql.where_clause {
            Self::lower_expression(&mut w.condition);
        }
        if let Some(g) = &mut dql.group_by {
            for e in &mut g.expressions {
                Self::lower_expression(e);
            }
        }
        if let Some(h) = &mut dql.having {
            Self::lower_expression(&mut h.condition);
        }
        if let Some(o) = &mut dql.order_by {
            for item in &mut o.items {
                Self::lower_expression(&mut item.expr);
            }
        }
        if let Some(l) = &mut dql.limit {
            Self::lower_expression(&mut l.count);
            if let Some(off) = &mut l.offset {
                Self::lower_expression(off);
            }
        }
    }

    pub fn lower_expression(expr: &mut Expression) {
        match expr {
            Expression::SanskritComparison { left, op: _, right, .. } => {
                Self::lower_expression(left);
                Self::lower_expression(right);
                
                let taken = std::mem::replace(expr, Expression::Wildcard);
                if let Expression::SanskritComparison { left, op, right, .. } = taken {
                    *expr = Expression::BinaryOp { left, op, right };
                }
            }
            Expression::BinaryOp { left, right, .. } | Expression::LogicalOp { left, right, .. } => {
                Self::lower_expression(left);
                Self::lower_expression(right);
            }
            Expression::FieldAccess { object, .. } => {
                Self::lower_expression(object);
            }
            Expression::Case { cases } => {
                for case in cases {
                    if let Some(cond) = &mut case.condition {
                        Self::lower_expression(cond);
                    }
                    Self::lower_expression(&mut case.result);
                }
            }
            Expression::FunctionCall { args, window, .. } => {
                for arg in args {
                    Self::lower_expression(arg);
                }
                if let Some(w) = window {
                    for p in &mut w.partition_by {
                        Self::lower_expression(p);
                    }
                    for o in &mut w.order_by {
                        Self::lower_expression(&mut o.expr);
                    }
                }
            }
            Expression::Aggregate { expr, .. } | Expression::Cast { expr, .. } | Expression::NamedArg { value: expr, .. } => {
                Self::lower_expression(expr);
            }
            Expression::Subquery(dql) => {
                Self::lower_dql(dql);
            }
            Expression::SetOp(set_op) => {
                for q in &mut set_op.queries {
                    Self::lower_expression(q);
                }
            }
            Expression::Between { expr, low, high, .. } => {
                Self::lower_expression(expr);
                Self::lower_expression(low);
                Self::lower_expression(high);
            }
            Expression::Exists { subquery, .. } => {
                Self::lower_dql(subquery);
            }
            _ => {}
        }
    }
}

