//! SSQL Compiler
//!
//! Orchestrates the full compilation pipeline:
//! 1. Discover and load schema from a sibling native DDL file (`schema.सं`)
//! 2. Lex source into tokens
//! 3. Pass 1: Build symbol table
//! 4. Pass 2: Annotate tokens with Vibhakti
//! 5. Parse into AST (chumsky-based, see `ssql_parser/`)
//! 6. Semantic analysis with type checking
//! 7. CodeGen via dialect adaptor

use std::path::Path;
use std::fs;
use anyhow::{Context, Result, bail};

use crate::schema::Schema;
use crate::codegen::traits::{SqlDialectAdaptor, TableRef, ColumnRef};
use crate::codegen::sqlite::SQLiteAdaptor;
use crate::codegen::postgres::PostgresAdaptor;

/// Reorders CREATE TABLE statements (only - views, functions, and every
/// other statement type keep their original position) so that a table is
/// never emitted before a table it has a foreign key to, using the
/// already-fully-populated `schema` (built in an earlier pass, see
/// `update_from_ddl`) to resolve each FK's target to a physical table name.
/// DDL statement order in the source .सं file has no guaranteed relationship
/// to dependency order (for example,
/// examples/university_bookstore/SAMSQL/schema.सं declares "sales"
/// before "stores" despite a FK from the former to the latter, which is
/// fatal on Postgres: it validates FK targets exist at CREATE TABLE time).
/// Falls back to leaving the original order untouched if a cycle is
/// detected (a genuine circular FK, which no reordering can fix) or if a
/// dependency points outside this batch (nothing to reorder against).
fn topo_sort_table_creates(statements: Vec<crate::ast::Statement>, schema: &Schema) -> Vec<crate::ast::Statement> {
    use std::collections::{HashMap, HashSet};

    let mut indices = Vec::new();
    let mut phys_names = Vec::new();
    let mut deps_per: Vec<HashSet<String>> = Vec::new();

    for (i, stmt) in statements.iter().enumerate() {
        if let crate::ast::Statement::Ddl(crate::ast::DdlStatement::Create(cs)) = stmt {
            let pratipadika = cs.type_name.pratipadika.as_str();
            let is_table = !matches!(
                pratipadika,
                "दृश्यम्" | "दृश्य" | "कल्पितपटल" | "कल्पितपटलम्"
            );
            if !is_table {
                continue;
            }
            let phys = cs.sql_name.clone().unwrap_or_else(|| cs.name.pratipadika.clone());
            let mut deps = HashSet::new();
            for col in &cs.columns {
                for c in &col.constraints {
                    if let crate::ast::Constraint::ForeignKey(ident) = c {
                        let target = schema.resolve_table(&ident.text)
                            .or_else(|| schema.resolve_table(&ident.pratipadika))
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| ident.pratipadika.clone());
                        if target != phys {
                            deps.insert(target);
                        }
                    }
                }
            }
            indices.push(i);
            phys_names.push(phys);
            deps_per.push(deps);
        }
    }

    if indices.len() <= 1 {
        return statements;
    }

    let name_to_pos: HashMap<&str, usize> = phys_names.iter().enumerate()
        .map(|(pos, n)| (n.as_str(), pos))
        .collect();

    let n = indices.len();
    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (pos, deps) in deps_per.iter().enumerate() {
        for dep_name in deps {
            if let Some(&dep_pos) = name_to_pos.get(dep_name.as_str()) {
                if dep_pos != pos {
                    adj[dep_pos].push(pos);
                    in_degree[pos] += 1;
                }
            }
        }
    }

    // Kahn's algorithm; the ready-set is re-sorted each iteration so ties
    // resolve in original-index order rather than arbitrary HashSet order,
    // keeping the result deterministic and as close to source order as the
    // dependency constraints allow.
    let mut ready: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut sorted_order = Vec::with_capacity(n);
    while !ready.is_empty() {
        ready.sort();
        let cur = ready.remove(0);
        sorted_order.push(cur);
        for &next in &adj[cur] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                ready.push(next);
            }
        }
    }

    if sorted_order.len() != n {
        // Cycle detected - can't topologically sort, leave as authored.
        return statements;
    }

    let mut slots: Vec<Option<crate::ast::Statement>> = statements.into_iter().map(Some).collect();
    let mut table_stmts: Vec<Option<crate::ast::Statement>> = indices.iter().map(|&i| slots[i].take()).collect();
    for (k, &idx) in indices.iter().enumerate() {
        let source_pos = sorted_order[k];
        slots[idx] = table_stmts[source_pos].take();
    }

    // Hoist CREATE VIEW / CREATE INDEX statements to always emit after all
    // CREATE TABLEs. Table statements above keep their
    // original slot *positions* even after being reordered by FK
    // dependency - only their *content* moves between slots - so a
    // VIEW/INDEX originally sitting between a table's old and new position
    // could otherwise still end up emitted before a table it depends on.
    let last_table_idx = *indices.iter().max().unwrap();
    let mut hoisted: Vec<crate::ast::Statement> = Vec::new();
    for slot in slots.iter_mut() {
        let is_hoistable = match slot {
            Some(crate::ast::Statement::Ddl(crate::ast::DdlStatement::CreateIndex(_))) => true,
            Some(crate::ast::Statement::Ddl(crate::ast::DdlStatement::Create(cs))) => {
                matches!(cs.type_name.pratipadika.as_str(), "दृश्यम्" | "दृश्य")
            }
            _ => false,
        };
        if is_hoistable {
            hoisted.push(slot.take().unwrap());
        }
    }
    let mut result: Vec<crate::ast::Statement> = Vec::with_capacity(slots.len() + hoisted.len());
    for (i, slot) in slots.into_iter().enumerate() {
        if let Some(stmt) = slot {
            result.push(stmt);
        }
        if i == last_table_idx {
            result.append(&mut hoisted);
        }
    }
    result
}

/// Disambiguates un-aliased `SELECT` columns that immediately precede a
/// positional `प्रदाय` UDF call (`col1 च arg1 च arg2 च प्रदाय my_udf`):
/// because both the `SELECT` column list and positional `प्रदाय` arguments
/// are separated by `च`, the parser's `multi_positional_pradaya_tail` collects
/// all leading `च`-separated expressions into `FunctionCall.args`. When the
/// target UDF has a known parameter count `p` (`p >= 1`) and `args.len() > p`,
/// the leading `args.len() - p` expressions are the preceding bare `SELECT`
/// columns (`SelectItem { expr, alias: None }`) and the trailing `p`
/// expressions are the UDF's positional arguments.
fn normalize_udf_select_columns(ast: &mut crate::ast::AstNode, schema: &Schema) {
    use std::collections::HashMap;
    let mut arity_map: HashMap<String, usize> = schema
        .udf_params
        .iter()
        .map(|(k, v)| (k.clone(), v.len()))
        .collect();

    let stmts: &mut [crate::ast::Statement] = match ast {
        crate::ast::AstNode::QueryBatch { statements, .. } => statements.as_mut_slice(),
        crate::ast::AstNode::Statement(s) => std::slice::from_mut(s),
    };

    for stmt in stmts.iter() {
        if let crate::ast::Statement::Ddl(crate::ast::DdlStatement::CreateUdf(f)) = stmt {
            let p = f.params.len();
            let stem = f
                .name
                .pratipadika
                .trim_end_matches("म्")
                .trim_end_matches('ं')
                .trim_end_matches('ः')
                .to_string();
            arity_map.insert(f.name.text.clone(), p);
            arity_map.insert(f.name.pratipadika.clone(), p);
            arity_map.insert(stem, p);
        }
    }

    if arity_map.is_empty() {
        return;
    }

    fn normalize_dql(dql: &mut crate::ast::DqlStatement, arity_map: &HashMap<String, usize>) {
        let old_cols = std::mem::take(&mut dql.select.columns);
        let mut new_cols = Vec::with_capacity(old_cols.len());
        for col in old_cols {
            if let crate::ast::Expression::FunctionCall {
                ref name,
                ref args,
                ref window,
                ref pronoun_token,
            } = col.expr
            {
                let has_named = args.iter().any(|a| matches!(a, crate::ast::Expression::NamedArg { .. }));
                let clean = name
                    .trim_end_matches("म्")
                    .trim_end_matches('ं')
                    .trim_end_matches('ः');
                let expected = arity_map
                    .get(name.as_str())
                    .or_else(|| arity_map.get(clean))
                    .copied();
                if !has_named {
                    if let Some(p) = expected {
                        if p >= 1 && args.len() > p {
                            let split_idx = args.len() - p;
                            for leading in &args[..split_idx] {
                                new_cols.push(crate::ast::SelectItem {
                                    expr: leading.clone(),
                                    alias: None,
                                });
                            }
                            new_cols.push(crate::ast::SelectItem {
                                expr: crate::ast::Expression::FunctionCall {
                                    name: name.clone(),
                                    args: args[split_idx..].to_vec(),
                                    window: window.clone(),
                                    pronoun_token: pronoun_token.clone(),
                                },
                                alias: col.alias,
                            });
                            continue;
                        }
                    }
                }
            }
            new_cols.push(col);
        }
        dql.select.columns = new_cols;
    }

    for stmt in stmts.iter_mut() {
        match stmt {
            crate::ast::Statement::Dql(dql) => normalize_dql(dql, &arity_map),
            crate::ast::Statement::VarDecl { value, .. } => {
                if let crate::ast::Expression::Subquery(dql) = value.as_mut() {
                    normalize_dql(dql, &arity_map);
                }
            }
            _ => {}
        }
    }
}

/// Compiler configuration
pub struct CompilerConfig {
    pub dialect: String,
    pub verbose: bool,
    pub dump_ast: bool,
    pub dump_resolved: bool,
    pub dump_symtable: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            dialect: "sqlite".to_string(),
            verbose: false,
            dump_ast: false,
            dump_resolved: false,
            dump_symtable: false,
        }
    }
}

/// The SSQL compiler
pub struct Compiler {
    pub schema: Schema,
    adaptor: Box<dyn SqlDialectAdaptor>,
    config: CompilerConfig,
    alias_counter: usize,
    /// The dumps the `dump_*` flags asked for, in order, for the caller to
    /// show; the library itself prints nothing.
    pub dumps: Vec<String>,
}

impl Compiler {
    /// Create a new compiler
    pub fn new(config: CompilerConfig) -> Result<Self> {
        let adaptor: Box<dyn SqlDialectAdaptor> = match config.dialect.as_str() {
            "postgres" | "postgresql" => Box::new(PostgresAdaptor::new()),
            _ => Box::new(SQLiteAdaptor::new()),
        };
        
        Ok(Self {
            schema: Schema::default(),
            adaptor,
            config,
            alias_counter: 0,
            dumps: Vec::new(),
        })
    }
    
    /// Generate a unique table alias
    pub fn next_alias(&mut self) -> String {
        self.alias_counter += 1;
        format!("t{}", self.alias_counter)
    }
    
    /// Resolve a Sanskrit table name to SQL with auto-aliasing
    pub fn resolve_table(&mut self, sanskrit: &str) -> Option<TableRef> {
        // Get the SQL name first (immutable borrow ends here)
        let sql_name = self.schema.resolve_table(sanskrit)?.to_string();
        // Now we can mutate self
        let alias = self.next_alias();
        Some(TableRef {
            table_name: sql_name,
            alias,
        })
    }
    
    /// Resolve a Sanskrit column name to SQL
    pub fn resolve_column(&self, sanskrit: &str, table_context: Option<&str>) -> Option<ColumnRef> {
        self.schema.resolve_column(sanskrit, table_context).map(|(sql_name, sql_type, _)| {
            ColumnRef {
                table_alias: None, // Will be set by caller
                column_name: sql_name.to_string(),
                sql_type: Some(self.parse_sql_type(sql_type)),
            }
        })
    }
    
    /// Parse SQL type string to enum
    fn parse_sql_type(&self, type_str: &str) -> crate::ast::SqlType {
        match type_str {
            "integer" => crate::ast::SqlType::Integer,
            "real" => crate::ast::SqlType::Real,
            "numeric" => crate::ast::SqlType::Numeric,
            "text" => crate::ast::SqlType::Text,
            "date" => crate::ast::SqlType::Date,
            "timestamp" => crate::ast::SqlType::Timestamp,
            "boolean" => crate::ast::SqlType::Boolean,
            "blob" => crate::ast::SqlType::Blob,
            _ => crate::ast::SqlType::Text,
        }
    }
    
    /// Check if a column is safe for numeric aggregation
    pub fn is_numeric(&self, sanskrit_col: &str, table_context: Option<&str>) -> bool {
        self.schema.is_numeric_column(sanskrit_col, table_context)
    }
    
    /// Get the adaptor for SQL generation
    pub fn adaptor(&self) -> &dyn SqlDialectAdaptor {
        self.adaptor.as_ref()
    }
    
    /// Compile SSQL source to SQL
    pub fn compile(&mut self, source_path: &Path) -> Result<String> {
        let source = fs::read_to_string(source_path)
            .with_context(|| format!("Failed to read source file {:?}", source_path))?;
        
        // Auto-load native schema.सं from this file's directory or a parent
        // directory. Native SSQL DDL (schema.सं) is the authoritative
        // schema-declaration format across projects; if no schema.सं is
        // present in the directory hierarchy, compilation proceeds with
        // only in-file DDL declarations.
        let mut parent = source_path.parent();
        while let Some(p) = parent {
            let native_schema_path = p.join("schema.सं");

            let is_self_schema = if let (Ok(p1), Ok(p2)) = (native_schema_path.canonicalize(), source_path.canonicalize()) {
                p1 == p2
            } else {
                false
            };
            if native_schema_path.exists() && !is_self_schema {
                // A schema.सं was found - this directory is unambiguously the
                // intended schema root. Failing to load it from here on is a
                // hard error, not a cue to keep walking up to an unrelated
                // parent's schema: a stale/broken schema file must never be
                // silently skipped in favor of whatever schema happens to
                // exist further up the tree.
                let schema_source = fs::read_to_string(&native_schema_path)
                    .with_context(|| format!("Found schema file `{}` but failed to read it", native_schema_path.display()))?;
                self.load_schema_from_ssql_string(&schema_source)
                    .with_context(|| format!("Found schema file `{}` but failed to parse it - refusing to fall back to a parent directory's unrelated schema", native_schema_path.display()))?;
                break;
            }
            parent = p.parent();
        }

        self.compile_string_with_path(&source, source_path)
    }

    /// Load schema details dynamically from a SSQL DDL string
    /// Every table, column and CTE declaration names the nominative of its
    /// declared stem, gender and number (expert review J6/K2). The corpus
    /// ratchet held this until no finding was left; it is now an error. Only
    /// a source that declares something (each declaration names a gender) is
    /// checked, since the check parses it again.
    fn check_declarations(source: &str) -> Result<()> {
        if !["पुंलिङ्गम्", "स्त्रीलिङ्गम्", "नपुंसकलिङ्गम्"].iter().any(|g| source.contains(g)) {
            return Ok(());
        }
        if let Some(f) = crate::declaration_check::check_declarations(source).into_iter().next() {
            anyhow::bail!("Grammar error: `{}`: {}", f.name, f.problem);
        }
        Ok(())
    }

    /// A view's columns are those of its query, registered as a CTE's are, so
    /// a later statement can read from it (`अधिकविक्रीतपुस्तकेभ्यः
    /// विक्रीतप्रतयः आह्रियताम्`); before, only a column that happened to share
    /// a name with its base table's resolved. The view keeps its physical name.
    fn register_views(&mut self, ddl_stmts: &[crate::ast::DdlStatement]) -> Result<()> {
        for ddl in ddl_stmts {
            let crate::ast::DdlStatement::Create(create) = ddl else { continue };
            let Some(query) = &create.query else { continue };
            let crate::ast::Statement::Dql(dql) = query.as_ref() else { continue };
            let mut name = create.name.clone();
            name.pratipadika = create.pratipadika.clone();
            name.vibhakti_form.linga = Some(create.gender);
            let source = dql.source.table.pratipadika.as_str();
            let joined: Vec<String> = dql.joins.iter().map(|j| j.table.pratipadika.clone()).collect();
            self.schema.register_cte(&name, &dql.select.columns, Some(source), &joined)?;
            let physical = create.sql_name.clone().unwrap_or_else(|| create.name.text.clone());
            if let Some(view) = self.schema.tables.get_mut(&create.pratipadika) {
                view.sql_name = physical;
            }
        }
        Ok(())
    }

    pub fn load_schema_from_ssql_string(&mut self, source: &str) -> Result<()> {
        Self::check_declarations(source)?;
        use crate::lexer::tokenize;
        use crate::ssql_parser::{convert_vibhakti_tokens, parse};
        
        let lexer_tokens = tokenize(source).context("Lexer failed for schema")?;
        let positions: Vec<usize> = lexer_tokens.iter().map(|t| t.position).collect();
        
        let mut annotator = crate::annotator::Annotator::new()?;
        let enriched_tokens = annotator.annotate(lexer_tokens, &self.schema)?;
        drop(annotator);
        let parser_tokens = convert_vibhakti_tokens(enriched_tokens);
        
        let ast = parse(parser_tokens).map_err(|errors| {
            anyhow::anyhow!("Schema Parse errors: {}", spans_to_offsets(&errors.join("; "), &positions, source.len()))
        })?;
        
        let mut ddl_stmts = Vec::new();
        match ast {
            crate::ast::AstNode::QueryBatch { statements, .. } => {
                for stmt in statements {
                    if let crate::ast::Statement::Ddl(ddl) = stmt {
                        ddl_stmts.push(ddl);
                    }
                }
            },
            crate::ast::AstNode::Statement(stmt) => {
                if let crate::ast::Statement::Ddl(ddl) = stmt {
                    ddl_stmts.push(ddl);
                }
            }
        }
        
        if !ddl_stmts.is_empty() {
            self.schema.update_from_ddl(&ddl_stmts, &self.config.dialect)?;
            self.register_views(&ddl_stmts)?;
        }
        
        Ok(())
    }

    /// Compile string helper that also handles artifact dumping if path is provided
    pub fn compile_string_with_path(&mut self, source: &str, _source_path: &Path) -> Result<String> {
        use crate::lexer::tokenize;
        use crate::ssql_parser::{convert_vibhakti_tokens, parse};
        use crate::ast_codegen::AstCodeGen;
        
        // Step 1: Tokenize
        let lexer_tokens = tokenize(source).context("Lexer failed")?;
        let positions: Vec<usize> = lexer_tokens.iter().map(|t| t.position).collect();
        Self::check_declarations(source)?;

        // Every statement ends with the pūrṇavirāma: a source whose last
        // token is anything else has an unfinished final sentence.
        if let Some(last) = lexer_tokens.last() {
            if !matches!(last.token_type, crate::annotator::TokenType::Purnadanda) {
                anyhow::bail!(
                    "Incomplete statement: the last statement must end with '।' (found '{}' at byte offset {})",
                    last.text, last.position
                );
            }
        }
        
        // PASS 1: Build Schema using default annotator
        let mut annotator_pass1 = crate::annotator::Annotator::new()?;
        let enriched_tokens_pass1 = annotator_pass1.annotate(lexer_tokens.clone(), &self.schema)?;
        let parser_tokens_pass1 = convert_vibhakti_tokens(enriched_tokens_pass1);
        
        // Parse AST (assuming it succeeds for DDL with default schema)
        let mut ddl_stmts = Vec::new();
        let mut cte_decls = Vec::new();
        
        match parse(parser_tokens_pass1) {
            Ok(ast_node) => {
                match ast_node {
                    crate::ast::AstNode::QueryBatch { statements, .. } => {
                        for stmt in statements {
                            match stmt {
                                crate::ast::Statement::Ddl(ddl) => ddl_stmts.push(ddl),
                                crate::ast::Statement::VarDecl { name, value, .. } => cte_decls.push((name, *value)),
                                _ => {}
                            }
                        }
                    },
                    crate::ast::AstNode::Statement(stmt) => {
                        match stmt {
                            crate::ast::Statement::Ddl(ddl) => ddl_stmts.push(ddl),
                            crate::ast::Statement::VarDecl { name, value, .. } => cte_decls.push((name, *value)),
                            _ => {}
                        }
                    }
                }
            },
            // Pass 2 reports the same errors with the full schema.
            Err(_) => {}
        }

        drop(annotator_pass1); // Release pass-1 annotator before PASS 2
        
        // Update Schema with parsed DDLs
        if !ddl_stmts.is_empty() {
            self.schema.update_from_ddl(&ddl_stmts, &self.config.dialect)?;
            self.register_views(&ddl_stmts)?;
        }

        // Register CTE schemas in self.schema
        for (cte_name, value_expr) in cte_decls {
            match value_expr {
                crate::ast::Expression::Subquery(dql) => {
                    let default_ctx = dql.source.table.pratipadika.as_str();
                    // Also pass the CTE's own JOINed tables (already in
                    // scope here) so register_cte can scope its bare-column
                    // fallback search to the query's actual tables, not just
                    // the primary FROM-source.
                    let joined_ctxs: Vec<String> = dql.joins.iter().map(|j| j.table.pratipadika.clone()).collect();
                    self.schema.register_cte(&cte_name, &dql.select.columns, Some(default_ctx), &joined_ctxs)?;
                },
                crate::ast::Expression::SetOp(set_op) => {
                    if !set_op.queries.is_empty() {
                        if let crate::ast::Expression::Subquery(ref dql) = set_op.queries[0] {
                            let default_ctx = dql.source.table.pratipadika.as_str();
                            let joined_ctxs: Vec<String> = dql.joins.iter().map(|j| j.table.pratipadika.clone()).collect();
                            self.schema.register_cte(&cte_name, &dql.select.columns, Some(default_ctx), &joined_ctxs)?;
                        }
                    }
                },
                _ => {}
            }
        }
        
        // PASS 2: Full Parse with Real Schema
        let mut annotator = crate::annotator::Annotator::new()?;
        let enriched_tokens = annotator.annotate(lexer_tokens, &self.schema)?;
        let parser_tokens = convert_vibhakti_tokens(enriched_tokens);

        let mut ast = crate::ssql_parser::parse_checked(parser_tokens)
            .map_err(|errors| {
                anyhow::anyhow!("Parse errors: {}", spans_to_offsets(&errors.join("; "), &positions, source.len()))
            })?;

        normalize_udf_select_columns(&mut ast, &self.schema);
        
        // Step 4: Validate (Schema-First Check)
        let mut validator = crate::validator::Validator::new(&self.schema);
        validator.validate(&ast)?;
        
        // Fail compilation if any grammar warnings/errors exist
        if !validator.warnings.is_empty() {
            let messages: Vec<&str> = validator.warnings.iter().map(|w| w.trim_start_matches("Grammar Warning: ")).collect();
            return Err(anyhow::anyhow!("Grammar error: {}", messages.join("\n")));
        }
        

        // Lower transient SanskritComparison nodes to standard BinaryOp nodes
        ast.lower_ast();

        // Step 5: Semantic Resolution (Existing)
        let resolver = crate::resolution::SemanticResolver::new(&self.schema);
        resolver.resolve(&mut ast);

        if self.config.dump_ast {
            self.dumps.push(format!("=== AST ===\n{:#?}", ast));
        }
        if self.config.dump_symtable {
            let dump = self.symbol_table_dump(&ast, &annotator.symbol_table);
            self.dumps.push(dump);
        }
        
        // Step 4: Transform AST (Resolve Identifiers to SQL Names)
        let mut transformer = crate::ast_transformer::AstTransformer::new(&self.schema, &annotator.symbol_table);
        match &mut ast {
            crate::ast::AstNode::QueryBatch { statements, .. } => {
                for stmt in statements {
                    transformer.transform_statement(stmt)?;
                }
            },
            crate::ast::AstNode::Statement(stmt) => {
                 transformer.transform_statement(stmt)?;
            }
        }
        
        if self.config.dump_resolved {
            self.dumps.push(format!("=== Resolved AST ===\n{:#?}", ast));
        }

        // Extract parameters and statements
        let (parameters, statements) = match ast {
            crate::ast::AstNode::QueryBatch { parameters, statements, .. } => (parameters, statements),
            crate::ast::AstNode::Statement(stmt) => (vec![], vec![stmt]),
        };
        // self.schema is already fully populated (update_from_ddl ran in
        // Pass 1, above), so every CREATE TABLE's FK targets can already be
        // resolved to physical names - safe to reorder before emission.
        let statements = topo_sort_table_creates(statements, &self.schema);

        let param_names: Vec<String> = parameters.iter().map(|p| p.name.text.clone()).collect();

        // Step 4: Generate SQL using codegen
        let mut codegen = AstCodeGen::new(self.adaptor.as_ref(), &self.schema, param_names);
        
        let mut sql_parts = Vec::new();
        let mut ctes: Vec<String> = Vec::new();
        let mut has_recursive_cte = false;
        for stmt in statements {
                // Placeholders are numbered per SQL statement; a CTE and the
                // query it belongs to are one statement.
                if ctes.is_empty() {
                    codegen.param_count = 0;
                }
                match stmt {
                    crate::ast::Statement::VarDecl { name, value, is_recursive, cycle_col } => {
                        // Set codegen recursive context
                        codegen.is_recursive_cte = is_recursive;
                        codegen.cycle_col = cycle_col.clone();
                        codegen.cte_name = name.text.clone();
                        if is_recursive {
                            has_recursive_cte = true;
                        }

                        // Convert VarDecl to CTE
                        let sql = match *value {
                            crate::ast::Expression::Subquery(dql) => {
                                codegen.generate_dql_raw(&dql).context("Failed to generate CTE SQL")?
                            },
                            crate::ast::Expression::SetOp(set_op) => {
                                codegen.generate_set_op(&set_op).context("Failed to generate CTE SetOp SQL")?
                            },
                            expr => {
                                // Try to generate simple SELECT <expr>
                                // Note: If expr uses columns without FROM, SQL might fail, but we allow generation.
                                codegen.generate_expression(&expr, None, None).context("Failed to generate CTE Expression")?
                            }
                        };

                        let cte_sql = format!("{} AS (\n{}\n)", name.text, sql);
                        ctes.push(cte_sql);

                        // Reset codegen recursive context
                        codegen.is_recursive_cte = false;
                        codegen.cycle_col = None;
                        codegen.cte_name = String::new();
                    },
                    crate::ast::Statement::Dql(dql) => {
                         let mut sql = codegen.generate_dql(&dql).context("Code generation failed")?;
                         
                         // Prepend CTEs if any
                         if !ctes.is_empty() {
                             let with_keyword = if has_recursive_cte { "WITH RECURSIVE" } else { "WITH" };
                             let with_clause = format!("{} {}\n", with_keyword, ctes.join(",\n"));
                             sql = format!("{}{}", with_clause, sql);
                             ctes.clear(); // Consumed
                             has_recursive_cte = false;
                         }
                         sql_parts.push(sql);
                    },
                    crate::ast::Statement::SetOp(set_op) => {
                         let sql_inner = codegen.generate_set_op(&set_op).context("Code generation failed for SetOp")?;
                         let mut sql = format!("{};\n", sql_inner); // Add semicolon manually if generate_set_op doesn't

                         if !ctes.is_empty() {
                             let with_clause = format!("WITH {}\n", ctes.join(",\n"));
                             sql = format!("{}{}", with_clause, sql);
                             ctes.clear();
                         }
                         sql_parts.push(sql);
                    },
                    crate::ast::Statement::Dml(dml) => {
                         let mut sql = codegen.generate_dml(&dml).context("Code generation failed for DML")?;
                         if !ctes.is_empty() {
                             let with_clause = format!("WITH {}\n", ctes.join(",\n"));
                             sql = format!("{}{}", with_clause, sql);
                             ctes.clear();
                         }
                         sql_parts.push(sql);
                    },
                    crate::ast::Statement::Explain(inner) => {
                         let mut sql = codegen.generate_explain(&inner).context("Code generation failed for EXPLAIN")?;
                         if !ctes.is_empty() {
                             let with_clause = format!("WITH {}\n", ctes.join(",\n"));
                             sql = format!("{}{}", with_clause, sql);
                             ctes.clear();
                         }
                         sql_parts.push(sql);
                    },
                    crate::ast::Statement::Ddl(ddl) => {
                         let sql = codegen.generate_ddl(&ddl).context("Code generation failed for DDL")?;
                         ctes.clear(); 
                         if !sql.is_empty() {
                             sql_parts.push(sql);
                         }
                    },
                    crate::ast::Statement::Tcl(tcl) => {
                         let sql = codegen.generate_tcl(&tcl).context("Code generation failed for TCL")?;
                         ctes.clear();
                         if !sql.is_empty() {
                             sql_parts.push(sql);
                         }
                    },
                    _ => {
                        bail!("Unsupported statement type for code generation: {:?}", stmt);
                    }
                }
        }
        
        let sql = sql_parts.join("\n");
        
        // Add header comments
        let header = format!(
            "-- Generated SQL from SSQL (v2 compiler)\n-- Dialect: {}\n\n",
            self.adaptor.name()
        );
        
        Ok(format!("{}{}", header, sql))
    }
    
    /// Compile `source` against the schema exactly as it was loaded, and
    /// leave that schema unchanged afterwards. `compile_string` registers a
    /// query's in-file DDL and CTEs into `self.schema`; a long-lived compiler
    /// (the Python binding, an editor) must not let one query's CTEs make a
    /// later query that merely names them compile.
    pub fn compile_isolated(&mut self, source: &str) -> Result<String> {
        let loaded_schema = self.schema.clone();
        let result = self.compile_string(source);
        self.schema = loaded_schema;
        self.alias_counter = 0;
        result
    }

    /// Compile SSQL source string to SQL (legacy wrapper)
    pub fn compile_string(&mut self, source: &str) -> Result<String> {
        // Just use a dummy path for internal calls if needed, or modify to Option
        self.compile_string_with_path(source, Path::new("inline.ssql"))
    }

    /// Analyze SSQL source string for LSP
    pub fn analyze_string(&self, source: &str) -> Result<crate::analysis::AnalysisData> {
        use crate::lexer::tokenize;
        use crate::ssql_parser::{convert_vibhakti_tokens, parse};
        use crate::analysis::{SymbolCollector, AnalysisData};
        
        // 1. Tokenize
        let lexer_tokens = tokenize(source).context("Lexer failed")?;
        let positions: Vec<usize> = lexer_tokens.iter().map(|t| t.position).collect();
        
        // 2. Annotate
        // Annotator::new() might not be needed if static, but following compile_string_with_path
        let mut annotator = crate::annotator::Annotator::new()?;
        let enriched_tokens = annotator.annotate(lexer_tokens, &self.schema)?;
        let parser_tokens = convert_vibhakti_tokens(enriched_tokens);
        
        // 3. Parse
        let ast = parse(parser_tokens)
            .map_err(|errors| {
                anyhow::anyhow!("Parse errors: {}", spans_to_offsets(&errors.join("; "), &positions, source.len()))
            })?;
            
        // 4. Collect Symbols
        let mut collector = SymbolCollector::new(&self.schema);
        collector.analyze(&ast)?;
        
        // 5. Build Schema Metadata
        let mut tables = Vec::new();
        for (name, table_def) in &self.schema.tables {
             let columns = table_def.fields.keys().cloned().collect();
             tables.push(crate::analysis::TableMetadata {
                 name: name.clone(),
                 columns,
             });
        }
        
        Ok(AnalysisData {
            symbols: collector.symbols,
            schema_metadata: crate::analysis::SchemaMetadata { tables },
        })
    }

    /// The symbol table as `--dump-symtable` shows it: each identifier the
    /// query uses, then every declared symbol with its forms.
    fn symbol_table_dump(&self, ast: &crate::ast::AstNode, st: &crate::symbol_table::SymbolTable) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let _ = writeln!(out, "=== Symbol Table ===");
        let _ = writeln!(out, "{:<20} | {:<20} | {:<15} | {:<20}", "Token", "Pratipadika", "Type", "Source");
        let _ = writeln!(out, "{}", "-".repeat(80));
        self.analyze_symbols_with_callback(ast, |id, source| {
            let _ = writeln!(out, "{:<20} | {:<20} | {:<15?} | {:<20}", id.text, id.pratipadika, id.symbol_type, source);
        });
        let _ = writeln!(out, "\n=== Declared Symbols ===");
        for key in st.all_identifiers() {
            if let Some(entry) = st.get(key) {
                let _ = writeln!(out, "\n[{}] stem {}, {:?} {:?}", key, entry.pratipadika, entry.linga, entry.vachana);
                let mut forms: Vec<_> = entry.forms.iter().collect();
                forms.sort_by_key(|f| *f.0 as u8);
                for (vibhakti, form) in forms {
                    let _ = writeln!(out, "    {:<12}: {}", format!("{:?}", vibhakti), form);
                }
            }
        }
        out
    }

    fn analyze_symbols_with_callback<F>(&self, ast: &crate::ast::AstNode, mut callback: F) 
    where F: FnMut(&crate::ast::Identifier, &str) 
    {
         match ast {
            crate::ast::AstNode::QueryBatch { statements, .. } => {
                for stmt in statements {
                    match stmt {
                        crate::ast::Statement::Dql(dql) => self.visit_dql_with_callback(dql, &mut callback),
                        crate::ast::Statement::SetOp(set_op) => {
                            for q in &set_op.queries {
                                 // Now q is Expression
                                 self.visit_expression_with_callback(q, &mut callback);
                            }
                        },
                        _ => {}
                    }
                }
            },
            crate::ast::AstNode::Statement(stmt) => {
                match stmt {
                    crate::ast::Statement::Dql(dql) => self.visit_dql_with_callback(dql, &mut callback),
                    crate::ast::Statement::SetOp(set_op) => {
                        for q in &set_op.queries {
                             self.visit_expression_with_callback(q, &mut callback);
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    fn visit_dql_with_callback<F>(&self, dql: &crate::ast::DqlStatement, callback: &mut F) 
    where F: FnMut(&crate::ast::Identifier, &str)
    {
        // Source
        self.visit_identifier_with_callback(&dql.source.table, callback);
        
        // Select
        for col in &dql.select.columns {
            self.visit_expression_with_callback(&col.expr, callback);
        }
        
        // Where
        if let Some(w) = &dql.where_clause {
            self.visit_expression_with_callback(&w.condition, callback);
        }

        // Group By
        if let Some(g) = &dql.group_by {
            for expr in &g.expressions {
                self.visit_expression_with_callback(expr, callback);
            }
        }

        // Order By
        if let Some(o) = &dql.order_by {
            for item in &o.items {
                self.visit_expression_with_callback(&item.expr, callback);
            }
        }
        
        // Limit
        if let Some(l) = &dql.limit {
            self.visit_expression_with_callback(&l.count, callback);
        }
    }

    fn visit_expression_with_callback<F>(&self, expr: &crate::ast::Expression, callback: &mut F) 
    where F: FnMut(&crate::ast::Identifier, &str)
    {
        match expr {
            crate::ast::Expression::Cast { expr, .. } => self.visit_expression_with_callback(expr, callback),
            crate::ast::Expression::Identifier(id) => self.visit_identifier_with_callback(id, callback),
            crate::ast::Expression::FieldAccess { object, field } => {
                 self.visit_expression_with_callback(object, callback);
                 self.visit_identifier_with_callback(field, callback);
            },
            crate::ast::Expression::BinaryOp { left, right, .. } => {
                self.visit_expression_with_callback(left, callback);
                self.visit_expression_with_callback(right, callback);
            },
            crate::ast::Expression::SanskritComparison { left, right, .. } => {
                self.visit_expression_with_callback(left, callback);
                self.visit_expression_with_callback(right, callback);
            },
            crate::ast::Expression::FunctionCall { args, window, .. } => {
                for arg in args { self.visit_expression_with_callback(arg, callback); }
                if let Some(w) = window {
                    for e in &w.partition_by { self.visit_expression_with_callback(e, callback); }
                    for o in &w.order_by { self.visit_expression_with_callback(&o.expr, callback); }
                }
            },
            crate::ast::Expression::Wildcard => {}, // No identifiers in wildcard
            crate::ast::Expression::Aggregate { expr, .. } => {
                 self.visit_expression_with_callback(expr, callback);
            },
            crate::ast::Expression::Case { cases } => {
                for arm in cases {
                    if let Some(cond) = &arm.condition { self.visit_expression_with_callback(cond, callback); }
                    self.visit_expression_with_callback(&arm.result, callback);
                }
            },
            crate::ast::Expression::LogicalOp { left, right, .. } => {
                self.visit_expression_with_callback(left, callback);
                self.visit_expression_with_callback(right, callback);
            },
            crate::ast::Expression::Subquery(dql) => {
                self.visit_dql_with_callback(dql, callback);
            },
            crate::ast::Expression::Between { expr, low, high, .. } => {
                self.visit_expression_with_callback(expr, callback);
                self.visit_expression_with_callback(low, callback);
                self.visit_expression_with_callback(high, callback);
            },
            crate::ast::Expression::Exists { subquery, .. } => {
                self.visit_dql_with_callback(subquery, callback);
            },
            crate::ast::Expression::SetOp(set_op) => {
                 for q in &set_op.queries {
                     self.visit_expression_with_callback(q, callback);
                 }
            },
            crate::ast::Expression::Literal(_) => {},
            crate::ast::Expression::NamedArg { value, .. } => {
                self.visit_expression_with_callback(value, callback);
            },
        }
    }

    fn visit_identifier_with_callback<F>(&self, id: &crate::ast::Identifier, callback: &mut F)
    where F: FnMut(&crate::ast::Identifier, &str)
    {
        // Determine source
        let source = if self.schema.resolve_pratipadika(&id.text).is_some() {
            "schema.सं (table)"
        } else if self.schema.tables.contains_key(&id.pratipadika) {
             "schema.सं (stem)"
        } else if self.is_toml_field(id) {
             "schema.सं (column)"
        } else if id.symbol_type == crate::types::SymbolType::Table || id.symbol_type == crate::types::SymbolType::Column {
             "query (nominative)"
        } else {
             "query"
        };
        
        callback(id, source);
    } 

    fn is_toml_field(&self, id: &crate::ast::Identifier) -> bool {
        for table in self.schema.tables.values() {
             if table.fields.contains_key(&id.text) || table.fields.contains_key(&id.pratipadika) {
                 return true;
             }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compiler_creation() {
        let config = CompilerConfig {
            dialect: "postgres".to_string(),
            verbose: false,
            dump_ast: false,
            dump_resolved: false,
            dump_symtable: false,
        };
        
        let compiler = Compiler::new(config).unwrap();
        // Since schema is default, it has no tables initially
        assert!(compiler.schema.tables.is_empty());
    }
}

/// Replace each token range ` at a..b` in a parse error with the source byte
/// offset of token `a` (` at byte offset N`), the position every front end
/// turns into a line and column.
pub(crate) fn spans_to_offsets(message: &str, positions: &[usize], source_len: usize) -> String {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(i) = rest.find(" at ") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 4..];
        let start: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        let tail = &after[start.len()..];
        if !start.is_empty() && tail.starts_with("..") {
            let end: String = tail[2..].chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(token) = start.parse::<usize>() {
                let offset = positions.get(token).copied().unwrap_or(source_len);
                out.push_str(&format!(" at byte offset {}", offset));
                rest = &tail[2 + end.len()..];
                continue;
            }
        }
        out.push_str(" at ");
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Replace each "byte offset N" in a message with "line L, column C" of
/// `source`, for people reading the error.
pub fn offsets_to_line_col(message: &str, source: &str) -> String {
    const MARKER: &str = "byte offset ";
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(i) = rest.find(MARKER) {
        out.push_str(&rest[..i]);
        let after = &rest[i + MARKER.len()..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        match digits.parse::<usize>() {
            Ok(offset) => {
                let before = &source[..offset.min(source.len())];
                let line = before.matches('\n').count() + 1;
                let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
                out.push_str(&format!("line {}, column {}", line, column));
                rest = &after[digits.len()..];
            }
            Err(_) => {
                out.push_str(MARKER);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}
