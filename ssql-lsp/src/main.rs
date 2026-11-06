use regex::Regex;
use ssql_compiler::analysis::{AnalysisData, SchemaMetadata, SymbolCollector, SymbolInfo, TableMetadata};
use ssql_compiler::annotator::{LexerToken, TokenType};
use ssql_compiler::ast::{AstNode, DdlStatement, Statement};
use ssql_compiler::ast_codegen::AstCodeGen;
use ssql_compiler::ast_transformer::AstTransformer;
use ssql_compiler::codegen::sqlite::SQLiteAdaptor;
use ssql_compiler::compiler::{Compiler, CompilerConfig};
use ssql_compiler::lexer::tokenize;
use ssql_compiler::morphology::MorphologyEngine;
use ssql_compiler::resolution::SemanticResolver;
use ssql_compiler::schema::Schema;
use ssql_compiler::ssql_parser::{convert_vibhakti_tokens, parse};
use ssql_compiler::types::{Linga, Vachana, Vibhakti, VibhaktiToken};
use ssql_compiler::validator::Validator;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    // Store source code per URI to map positions
    documents: Arc<RwLock<HashMap<Url, String>>>,
    // Store analysis data per URI
    analysis: Arc<RwLock<HashMap<Url, AnalysisData>>>,
}

impl Backend {
    /// Walk parent directories of `source_path` to find a sibling or ancestor `schema.सं`.
    fn find_schema_file(source_path: &Path) -> Option<PathBuf> {
        let mut parent = source_path.parent();
        while let Some(p) = parent {
            let candidate = p.join("schema.सं");
            if candidate.exists() {
                let is_same = match (candidate.canonicalize(), source_path.canonicalize()) {
                    (Ok(c1), Ok(c2)) => c1 == c2,
                    _ => candidate == source_path,
                };
                if !is_same {
                    return Some(candidate);
                }
            }
            parent = p.parent();
        }
        None
    }

    /// Parse SSQL tokens without printing to stdout.
    fn parse_tokens_quiet(
        tokens: Vec<ssql_compiler::ssql_parser::Token>,
    ) -> std::result::Result<AstNode, Vec<String>> {
        parse(tokens)
    }

    /// Populate `schema` from a DDL source string (such as `schema.सं`).
    fn load_ddl_into_schema(schema: &mut Schema, ddl_source: &str) {
        if let Ok(lex_tokens) = tokenize(ddl_source) {
            if let Ok(mut annotator) = ssql_compiler::annotator::Annotator::new() {
                if let Ok(enriched) = annotator.annotate(lex_tokens, schema) {
                    let parser_tokens = convert_vibhakti_tokens(enriched);
                    if let Ok(ast) = Self::parse_tokens_quiet(parser_tokens) {
                        let stmts = match ast {
                            AstNode::QueryBatch { statements, .. } => statements,
                            AstNode::Statement(s) => vec![s],
                        };
                        let mut ddl_stmts = Vec::new();
                        for stmt in stmts {
                            if let Statement::Ddl(ddl) = stmt {
                                ddl_stmts.push(ddl);
                            }
                        }
                        if !ddl_stmts.is_empty() {
                            let _ = schema.update_from_ddl(&ddl_stmts, "sqlite");
                        }
                    }
                }
            }
        }
    }

    /// Analyze SSQL source: `AnalysisData` for hover, completion and
    /// go-to-definition, and the diagnostics the editor shows.
    ///
    /// The diagnostics are exactly the CLI's verdict - the same `schema.सं`
    /// discovery and the same core compile (`Compiler::compile_isolated`),
    /// one diagnostic per line of the compiler's message - plus the advisory
    /// findings of the core linter (`--lint`). The editor never reports an
    /// error the CLI would not report, nor stays silent about one it would.
    fn analyze_and_diagnose(source: &str, source_path: &Path) -> (Option<AnalysisData>, Vec<Diagnostic>) {
        let (analysis, _stage_diagnostics) = Self::analyze_symbols(source, source_path);
        let mut diagnostics = Self::core_compile_diagnostics(source, source_path);
        Self::append_lint_diagnostics(source, &mut diagnostics);
        (analysis, diagnostics)
    }

    fn core_compile_diagnostics(source: &str, source_path: &Path) -> Vec<Diagnostic> {
        let config = CompilerConfig {
            dialect: "sqlite".to_string(),
            verbose: false,
            dump_ast: false,
            dump_resolved: false,
            dump_symtable: false,
        };
        let mut compiler = match Compiler::new(config) {
            Ok(c) => c,
            Err(e) => return vec![Self::make_error_diag(Self::line_range(0), "ssql", format!("{:#}", e))],
        };
        if let Some(schema_path) = Self::find_schema_file(source_path) {
            let loaded = std::fs::read_to_string(&schema_path)
                .map_err(anyhow::Error::from)
                .and_then(|ddl| compiler.load_schema_from_ssql_string(&ddl));
            if let Err(e) = loaded {
                return vec![Self::make_error_diag(
                    Self::line_range(0),
                    "ssql-schema",
                    format!("{}: {:#}", schema_path.display(), e),
                )];
            }
        }
        let message = match compiler.compile_isolated(source) {
            Ok(_) => return Vec::new(),
            Err(e) => format!("{:#}", e),
        };
        let tokens = tokenize(source).unwrap_or_default();
        message
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| {
                let range = if tokens.is_empty() {
                    Self::extract_lexer_error_range(source, line)
                } else {
                    Self::find_token_range_from_message(source, &tokens, line)
                };
                Self::make_error_diag(range, "ssql", line.to_string())
            })
            .collect()
    }

    /// Hover/completion/go-to-definition data. Runs the compiler stages to
    /// collect symbols; its stage diagnostics are not shown (the editor's
    /// verdict comes from `core_compile_diagnostics`).
    fn analyze_symbols(source: &str, source_path: &Path) -> (Option<AnalysisData>, Vec<Diagnostic>) {
        let mut diagnostics = Vec::new();
        let mut schema = Schema {
            dialect: "sqlite".to_string(),
            ..Default::default()
        };

        // 1. Auto-discover and load ancestor `schema.सं` if present
        if let Some(schema_path) = Self::find_schema_file(source_path) {
            if let Ok(ddl_src) = std::fs::read_to_string(&schema_path) {
                Self::load_ddl_into_schema(&mut schema, &ddl_src);
            }
        }

        // 2. Tokenize source
        let lexer_tokens = match tokenize(source) {
            Ok(tokens) => tokens,
            Err(e) => {
                let err_msg = e.to_string();
                let range = Self::extract_lexer_error_range(source, &err_msg);
                diagnostics.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("ssql-lexer".to_string()),
                    message: err_msg,
                    related_information: None,
                    tags: None,
                    data: None,
                });
                Self::append_lint_diagnostics(source, &mut diagnostics);
                return (Some(Self::empty_analysis_with_schema(&schema)), diagnostics);
            }
        };

        if lexer_tokens.is_empty() {
            return (Some(Self::empty_analysis_with_schema(&schema)), diagnostics);
        }

        // 3. Pass 1: Extract in-file DDL and CTE definitions into `schema`
        if let Ok(mut annotator_p1) = ssql_compiler::annotator::Annotator::new() {
            if let Ok(enriched_p1) = annotator_p1.annotate(lexer_tokens.clone(), &schema) {
                let parser_tokens_p1 = convert_vibhakti_tokens(enriched_p1);
                if let Ok(ast_p1) = Self::parse_tokens_quiet(parser_tokens_p1) {
                    let stmts = match ast_p1 {
                        AstNode::QueryBatch { statements, .. } => statements,
                        AstNode::Statement(s) => vec![s],
                    };
                    let mut ddl_stmts = Vec::new();
                    let mut cte_decls = Vec::new();
                    for stmt in stmts {
                        match stmt {
                            Statement::Ddl(ddl) => ddl_stmts.push(ddl),
                            Statement::VarDecl { name, value, .. } => cte_decls.push((name, *value)),
                            _ => {}
                        }
                    }
                    if !ddl_stmts.is_empty() {
                        if let Err(e) = schema.update_from_ddl(&ddl_stmts, "sqlite") {
                            let msg = e.to_string();
                            let range = Self::find_token_range_from_message(source, &lexer_tokens, &msg);
                            diagnostics.push(Self::make_error_diag(range, "ssql-schema", msg));
                        }
                    }
                    for (cte_name, value_expr) in cte_decls {
                        if let ssql_compiler::ast::Expression::Subquery(dql) = value_expr {
                            let default_ctx = dql.source.table.pratipadika.as_str();
                            let joined_ctxs: Vec<String> =
                                dql.joins.iter().map(|j| j.table.pratipadika.clone()).collect();
                            let _ = schema.register_cte(
                                &cte_name,
                                &dql.select.columns,
                                Some(default_ctx),
                                &joined_ctxs,
                            );
                        }
                    }
                }
            }
        }

        // 4. Pass 2: Annotate and parse with the enriched schema
        let mut annotator = match ssql_compiler::annotator::Annotator::new() {
            Ok(a) => a,
            Err(e) => {
                diagnostics.push(Self::make_error_diag(
                    Self::line_range(0),
                    "ssql-annotator",
                    e.to_string(),
                ));
                return (Some(Self::empty_analysis_with_schema(&schema)), diagnostics);
            }
        };

        let enriched_tokens = match annotator.annotate(lexer_tokens.clone(), &schema) {
            Ok(t) => t,
            Err(e) => {
                let msg = e.to_string();
                let range = Self::find_token_range_from_message(source, &lexer_tokens, &msg);
                diagnostics.push(Self::make_error_diag(range, "ssql-annotator", msg));
                return (Some(Self::empty_analysis_with_schema(&schema)), diagnostics);
            }
        };

        let parser_tokens = convert_vibhakti_tokens(enriched_tokens.clone());
        let mut ast = match Self::parse_tokens_quiet(parser_tokens) {
            Ok(ast) => ast,
            Err(parse_errors) => {
                for msg in parse_errors {
                    let range = Self::find_token_range_from_message(source, &lexer_tokens, &msg);
                    diagnostics.push(Self::make_error_diag(range, "ssql-parser", msg));
                }
                Self::append_lint_diagnostics(source, &mut diagnostics);
                let mut fallback = Self::empty_analysis_with_schema(&schema);
                Self::collect_token_level_hover_symbols(
                    &lexer_tokens,
                    &enriched_tokens,
                    &schema,
                    &mut fallback.symbols,
                );
                return (Some(fallback), diagnostics);
            }
        };

        // 5. Semantic Validation
        let mut validator = Validator::new(&schema);
        if let Err(e) = validator.validate(&ast) {
            let msg = e.to_string();
            let range = Self::find_token_range_from_message(source, &lexer_tokens, &msg);
            diagnostics.push(Self::make_error_diag(range, "ssql-validator", msg));
        }
        for warn in &validator.warnings {
            let range = Self::find_token_range_from_message(source, &lexer_tokens, warn);
            diagnostics.push(Self::make_error_diag(range, "ssql-validator", warn.clone()));
        }

        // 6. Collect Symbols (including DDL column definitions, UDF params, CTEs, and token-level hover help)
        let mut collector = SymbolCollector::new(&schema);
        let _ = collector.analyze(&ast);
        Self::collect_extra_ddl_and_cte_symbols(&ast, &schema, &mut collector.symbols);
        Self::collect_token_level_hover_symbols(
            &lexer_tokens,
            &enriched_tokens,
            &schema,
            &mut collector.symbols,
        );

        // 7. AST Lowering, Resolution, Transformation & CodeGen check
        ast.lower_ast();
        let resolver = SemanticResolver::new(&schema);
        resolver.resolve(&mut ast);

        let mut transformer = AstTransformer::new(&schema, &annotator.symbol_table);
        let mut transform_err = None;
        match &mut ast {
            AstNode::QueryBatch { statements, .. } => {
                for stmt in statements {
                    if let Err(e) = transformer.transform_statement(stmt) {
                        transform_err = Some(e);
                        break;
                    }
                }
            }
            AstNode::Statement(stmt) => {
                if let Err(e) = transformer.transform_statement(stmt) {
                    transform_err = Some(e);
                }
            }
        }
        if let Some(e) = transform_err {
            let msg = e.to_string();
            let range = Self::find_token_range_from_message(source, &lexer_tokens, &msg);
            diagnostics.push(Self::make_error_diag(range, "ssql-resolver", msg));
        } else {
            let adaptor = SQLiteAdaptor::new();
            let (params, stmts) = match &ast {
                AstNode::QueryBatch { parameters, statements, .. } => (parameters.clone(), statements.clone()),
                AstNode::Statement(s) => (vec![], vec![s.clone()]),
            };
            let param_names: Vec<String> = params.iter().map(|p| p.name.text.clone()).collect();
            let mut codegen = AstCodeGen::new(&adaptor, &schema, param_names);
            for stmt in &stmts {
                let res = match stmt {
                    Statement::Dql(dql) => codegen.generate_dql(dql).map(|_| ()),
                    Statement::Dml(dml) => codegen.generate_dml(dml).map(|_| ()),
                    Statement::Ddl(ddl) => codegen.generate_ddl(ddl).map(|_| ()),
                    Statement::SetOp(set_op) => codegen.generate_set_op(set_op).map(|_| ()),
                    Statement::Tcl(tcl) => codegen.generate_tcl(tcl).map(|_| ()),
                    Statement::Explain(inner) => codegen.generate_explain(inner).map(|_| ()),
                    Statement::VarDecl { value, .. } => match &**value {
                        ssql_compiler::ast::Expression::Subquery(dql) => codegen.generate_dql_raw(dql).map(|_| ()),
                        ssql_compiler::ast::Expression::SetOp(s) => codegen.generate_set_op(s).map(|_| ()),
                        expr => codegen.generate_expression(expr, None, None).map(|_| ()),
                    },
                    _ => Ok(()),
                };
                if let Err(e) = res {
                    let msg = e.to_string();
                    let range = Self::find_token_range_from_message(source, &lexer_tokens, &msg);
                    diagnostics.push(Self::make_error_diag(range, "ssql-codegen", msg));
                }
            }
        }

        // 8. Style and Purity Linter
        Self::append_lint_diagnostics(source, &mut diagnostics);

        let analysis_data = AnalysisData {
            symbols: collector.symbols,
            schema_metadata: Self::build_schema_metadata(&schema),
        };

        (Some(analysis_data), diagnostics)
    }

    fn collect_extra_ddl_and_cte_symbols(
        ast: &AstNode,
        schema: &Schema,
        symbols: &mut Vec<SymbolInfo>,
    ) {
        let stmts = match ast {
            AstNode::QueryBatch { statements, .. } => statements.as_slice(),
            AstNode::Statement(s) => std::slice::from_ref(s),
        };
        for stmt in stmts {
            match stmt {
                Statement::Ddl(DdlStatement::Create(create_stmt)) => {
                    let table_stem = create_stmt.pratipadika.clone();
                    for col in &create_stmt.columns {
                        if let Some(span) = col.name.span {
                            let resolved = col
                                .sql_name
                                .clone()
                                .or_else(|| Some(col.name.pratipadika.clone()));
                            let col_stem = if col.pratipadika.is_empty() {
                                col.name.pratipadika.clone()
                            } else {
                                col.pratipadika.clone()
                            };
                            symbols.push(SymbolInfo {
                                name: col_stem,
                                resolved,
                                kind: "Column".to_string(),
                                span,
                                description: format!(
                                    "Column Definition in {} ({:?})",
                                    table_stem, col.data_type
                                ),
                                container: Some(table_stem.clone()),
                            });
                        }
                    }
                }
                Statement::Ddl(DdlStatement::CreateUdf(udf_stmt)) => {
                    for param in &udf_stmt.params {
                        if let Some(span) = param.name.span {
                            symbols.push(SymbolInfo {
                                name: param.name.pratipadika.clone(),
                                resolved: Some(param.name.text.clone()),
                                kind: "Parameter".to_string(),
                                span,
                                description: format!(
                                    "UDF Parameter ({:?}) of {}",
                                    param.data_type, udf_stmt.name.text
                                ),
                                container: Some(udf_stmt.name.pratipadika.clone()),
                            });
                        }
                    }
                }
                Statement::VarDecl { name, .. } => {
                    if let Some(span) = name.span {
                        let already_present = symbols
                            .iter()
                            .any(|s| s.span == span && s.name == name.pratipadika);
                        if !already_present {
                            let resolved = schema
                                .resolve_table(&name.pratipadika)
                                .map(|s| s.to_string())
                                .or_else(|| Some(name.text.clone()));
                            symbols.push(SymbolInfo {
                                name: name.pratipadika.clone(),
                                resolved,
                                kind: "CTE".to_string(),
                                span,
                                description: "Common Table Expression (सञ्ज्ञा)".to_string(),
                                container: None,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn describe_keyword_info(surface: &str) -> Option<(&'static str, &'static str, &'static str)> {
        match surface {
            "आह्रियताम्" => Some(("Verb (क्रियापदम्)", "SELECT ... (एकवचनम् — 1 column)", "Karmaṇi Loṭ singular verb: fetches exactly 1 projected item.")),
            "आह्रियेताम्" => Some(("Verb (क्रियापदम्)", "SELECT ... (द्विवचनम् — 2 columns)", "Karmaṇi Loṭ dual verb: fetches exactly 2 projected items.")),
            "आह्रियन्ताम्" => Some(("Verb (क्रियापदम्)", "SELECT ... (बहुवचनम् — 3+ columns)", "Karmaṇi Loṭ plural verb: fetches 3 or more projected items.")),
            "रच्यताम्" => Some(("Verb (क्रियापदम्)", "CREATE TABLE / VIEW / INDEX / FUNCTION", "Karmaṇi Loṭ verb: creates a schema object (`इति पटलम्`, `इति दृश्यम्`, `इति सूची`) or a pure function (`शुद्धम् कार्यम्`).")),
            "निवेश्यताम्" | "निवेश्येताम्" | "निवेश्यन्ताम्" => Some(("Verb (क्रियापदम्)", "INSERT INTO ...", "Karmaṇi Loṭ verb: inserts `<column> <value>` pairs into a locative-plural table; it agrees with the number of columns.")),
            "परिवर्त्यताम्" | "परिवर्त्येताम्" | "परिवर्त्यन्ताम्" => Some(("Verb (क्रियापदम्)", "UPDATE ... SET ...", "Karmaṇi Loṭ verb: sets `<column> <value> इति` pairs on a locative-plural table, with a filter or `सर्वत्र`.")),
            "निष्कास्यताम्" => Some(("Verb (क्रियापदम्)", "DELETE FROM ...", "Karmaṇi Loṭ verb: deletes the rows matching `इति नियमेन`.")),
            "लुप्यताम्" => Some(("Verb (क्रियापदम्)", "DROP ...", "Karmaṇi Loṭ verb: drops a table, view, index or column.")),
            "अभिधीयताम्" => Some(("Verb (क्रियापदम्)", "ALTER ... RENAME", "Karmaṇi Loṭ verb: renames a table or column (`… इति तस्मात् \"new\" इति अभिधीयताम्`).")),
            "विशदीक्रियताम्" => Some(("Verb (क्रियापदम्)", "EXPLAIN QUERY PLAN", "Karmaṇi Loṭ verb: explains the execution plan of a query.")),
            "प्रतिदीयते" => Some(("Verb (क्रियापदम्)", "RETURNS <type>", "Specifies the return type of a pure UDF (`शुद्धम् कार्यम्`).")),
            "प्रतिदीयताम्" => Some(("Verb (क्रियापदम्)", "RETURN <expr>", "Returns the result expression (`इति उत्तरम् प्रतिदीयताम्`) from a UDF.")),
            "दीयते" => Some(("Verb (क्रियापदम्)", "Parameter (एकवचनम् — 1 param)", "Declares the one parameter of a pure UDF.")),
            "दीयेते" => Some(("Verb (क्रियापदम्)", "Parameters (द्विवचनम् — 2 params)", "Declares the two parameters of a pure UDF.")),
            "दीयन्ते" => Some(("Verb (क्रियापदम्)", "Parameters (बहुवचनम् — 3+ params)", "Declares three or more parameters of a pure UDF.")),
            "प्रदाय" => Some(("Absolutive (ल्यबन्तम्)", "Passing arguments (`... प्रदाय fn`)", "Supplies positional or named arguments to a pure UDF call.")),
            "इति" => Some(("Particle (निपातः)", "Clause / Alias / Quote Marker", "Closes a clause (`… इति नियमेन`), names an alias (`<expr> <alias> इति आख्या`), or marks a type or role.")),
            "आख्या" => Some(("Noun (नाम)", "AS / SQL name", "The name a column, table or expression is known by: `<alias> इति आख्या`, `\"books\" इति आख्या`.")),
            "यत्र" => Some(("Particle (निपातः)", "Block Opener (columns / UDF body / OVER)", "Opens a table's column list, a UDF body, or a window specification.")),
            "च" => Some(("Conjunction (निपातः)", "AND / list item", "Follows every item of a list, the last one too (`शीर्षकम् च मूल्यम् च`), and every condition of an AND list.")),
            "वा" => Some(("Conjunction (निपातः)", "OR", "Follows every condition of an OR list, the last one too; group a mixed AND/OR with `यत् … इति`.")),
            "यत्" => Some(("Particle (निपातः)", "Group / Subquery Opener", "Opens a group or subquery closed by `इति` (`यत् … इति`).")),
            "नियमेन" => Some(("Clause Marker (तृतीया)", "WHERE ...", "Marks the row filter (`… इति नियमेन`).")),
            "समूहेन" => Some(("Clause Marker (तृतीया)", "GROUP BY ...", "Marks the grouping keys (`… इति समूहेन`).")),
            "समूहनियमेन" => Some(("Clause Marker (तृतीया)", "HAVING ...", "Marks the group filter (`… इति समूहनियमेन`).")),
            "क्रमेण" => Some(("Clause Marker (तृतीया)", "ORDER BY ...", "Marks the sort keys, before the select list (`मूल्यम् अवरोहः इति क्रमेण`).")),
            "मित्या" => Some(("Clause Marker (तृतीया)", "LIMIT <n>", "Limits the number of rows, before the select list (`५ इति मित्या`).")),
            "अपसारेण" => Some(("Clause Marker (तृतीया)", "OFFSET <n>", "Skips rows after a LIMIT (`५ इति मित्या २ अपसारेण`).")),
            "पुनरुक्तिम्" | "विना" => Some(("Modifier", "SELECT DISTINCT", "`पुनरुक्तिम् विना` (\"without repetition\") before the select list removes repeated rows.")),
            "विशिष्टस्य" | "विशिष्टायाः" | "विशिष्टयोः" | "विशिष्टानाम्" => Some(("Modifier (विशेषणम्)", "COUNT(DISTINCT x) …", "Inside an aggregate, agrees with its genitive argument (`विशिष्टायाः भाषायाः गणना`).")),
            "सह" => Some(("Postposition (निपातः)", "JOIN", "Follows an instrumental-plural table (`ग्राहकैः सह … इति मेलनेन`); `<table> सह सर्वमेलनेन` is a CROSS JOIN.")),
            "मेलनेन" => Some(("Clause Marker (तृतीया)", "INNER JOIN ON ...", "Completes an inner join condition (`… इति मेलनेन`).")),
            "वाममेलनेन" => Some(("Clause Marker (तृतीया)", "LEFT JOIN ON ...", "Completes a left outer join condition (`… इति वाममेलनेन`).")),
            "दक्षिणमेलनेन" => Some(("Clause Marker (तृतीया)", "RIGHT JOIN ON ...", "Completes a right outer join condition (`… इति दक्षिणमेलनेन`).")),
            "पूर्णमेलनेन" => Some(("Clause Marker (तृतीया)", "FULL OUTER JOIN ON ...", "Completes a full outer join condition (`… इति पूर्णमेलनेन`).")),
            "सर्वमेलनेन" => Some(("Clause Marker (तृतीया)", "CROSS JOIN", "Joins every row with every row (`<table> सह सर्वमेलनेन`).")),
            "सर्वम्" => Some(("Pronoun (सर्वनाम)", "SELECT *", "Projects every column of the source.")),
            "अधिकः" | "अधिका" | "अधिकम्" => Some(("Comparison (विशेषणम्)", "> (with नास्ति: <=)", "Takes an ablative target (`… इत्यस्मात् / इत्यस्याः`) and agrees in gender with the subject.")),
            "न्यूनः" | "न्यूना" | "न्यूनम्" => Some(("Comparison (विशेषणम्)", "< (with नास्ति: >=)", "Takes an ablative target (`… इत्यस्मात् / इत्यस्याः`) and agrees in gender with the subject.")),
            "समः" | "समा" | "समम्" => Some(("Comparison (विशेषणम्)", "= (with नास्ति: !=)", "Takes an instrumental target (`… इत्यनेन / इत्यनया`) and agrees in gender with the subject.")),
            "रिक्तः" | "रिक्ता" | "रिक्तम्" => Some(("Comparison (विशेषणम्)", "IS NULL (with नास्ति: IS NOT NULL)", "Checks for NULL (`रिक्ता अस्ति` / `रिक्ता नास्ति`), agreeing with the column.")),
            "इत्यस्मात्" => Some(("Pronoun (पञ्चमी पुंलिङ्गम्/नपुंसकलिङ्गम्)", "Than (Ablative Masculine/Neuter)", "Links a literal or expression to `अधिकः/अधिकम्` or `न्यूनः/न्यूनम्`.")),
            "इत्यस्याः" => Some(("Pronoun (पञ्चमी/षष्ठी स्त्रीलिङ्गम्)", "Than / Of (Feminine)", "Links a feminine literal or expression to a comparison or an aggregate.")),
            "इत्यनेन" => Some(("Pronoun (तृतीया पुंलिङ्गम्/नपुंसकलिङ्गम्)", "Equal to / By (Instrumental Masculine/Neuter)", "Links a literal or column to `समः/समम्`, or names the divisor of `विभज्य लब्धम्`.")),
            "इत्यनया" => Some(("Pronoun (तृतीया स्त्रीलिङ्गम्)", "Equal to (Instrumental Feminine)", "Links a literal or column to `समा अस्ति`.")),
            "इत्यस्य" => Some(("Pronoun (षष्ठी एकवचनम्)", "Of (Genitive Singular)", "Connects an expression to an aggregate or function (`… इत्यस्य योगः`).")),
            "इत्यनयोः" => Some(("Pronoun (षष्ठी/सप्तमी द्विवचनम्)", "Of / Among Both", "Two operands (`A च B च इत्यनयोः योगः`), BETWEEN (`… इत्यनयोः मध्ये अस्ति`), or a two-value IN.")),
            "इत्येषु" | "इत्यासु" => Some(("Pronoun (सप्तमी बहुवचनम्)", "IN (...)", "Membership in a list (`\"A\" च \"B\" च \"C\" च इत्येषु अस्ति`; feminine `इत्यासु`).")),
            "योगः" => Some(("Aggregate / Math (पुंलिङ्गम्)", "SUM(x) or (a + b)", "The sum of a column (`मूल्यस्य योगः`) or of its operands (`… इत्यनयोः योगः`).")),
            "माध्यम्" => Some(("Aggregate (नपुंसकलिङ्गम्)", "AVG(x)", "The average of a column (`मूल्यस्य माध्यम्`).")),
            "गणना" => Some(("Aggregate (स्त्रीलिङ्गम्)", "COUNT(x)", "Counts non-null values (`सङ्ख्यायाः गणना`); COUNT(*) is `पङ्क्तीनाम् गणना`.")),
            "महत्तमम्" => Some(("Aggregate (नपुंसकलिङ्गम्)", "MAX(x)", "The largest value (`मूल्यस्य महत्तमम्`).")),
            "लघुतमम्" => Some(("Aggregate (नपुंसकलिङ्गम्)", "MIN(x)", "The smallest value (`मूल्यस्य लघुतमम्`).")),
            "श्रेणी" => Some(("Window Function", "RANK()", "Rank over a window (`श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण`).")),
            "सघनश्रेणी" => Some(("Window Function", "DENSE_RANK()", "Rank without gaps over a window.")),
            "पङ्क्तिसङ्ख्या" => Some(("Window Function", "ROW_NUMBER()", "Sequential row numbers within a window partition.")),
            "पूर्वपङ्क्तेः" => Some(("Window Function", "LAG(col)", "The previous row's value (`पूर्वपङ्क्तेः मूल्यम् यत्र …`).")),
            "अनन्तरपङ्क्तेः" => Some(("Window Function", "LEAD(col)", "The next row's value (`अनन्तरपङ्क्तेः मूल्यम् यत्र …`).")),
            "चतुर्थांशाङ्कः" => Some(("Window Function", "NTILE(4)", "The quartile number within the window (`चतुर्थांशाङ्कः यत्र …`).")),
            "भागाङ्कः" => Some(("Window Function", "NTILE(n)", "The part number among n parts (`५ भागेषु भागाङ्कः यत्र …`; two parts `२ भागयोः भागाङ्कः`).")),
            "पटलम्" => Some(("DDL Keyword", "TABLE", "Declares or closes a table definition (`इति पटलम्`).")),
            "दृश्यम्" => Some(("DDL Keyword", "VIEW", "Declares a view (`इति दृश्यम्`).")),
            "सूची" => Some(("DDL Keyword", "INDEX", "Declares an index (`… आधारे \"name\" इति सूची रच्यताम्`).")),
            "शुद्धम्" => Some(("DDL Modifier", "PURE (Scalar UDF)", "Marks a side-effect-free inlined function (`शुद्धम् कार्यम्`).")),
            "कार्यम्" => Some(("DDL Keyword", "FUNCTION", "Declares or closes a user-defined function (`इति कार्यम्`).")),
            "पूर्णाङ्कः" => Some(("Data Type", "INTEGER", "Integer column or parameter type.")),
            "वास्तविकः" => Some(("Data Type", "REAL / NUMERIC", "Decimal numeric type.")),
            "वाक्यम्" => Some(("Data Type", "TEXT", "Text column or parameter type.")),
            "दिनाङ्कः" => Some(("Data Type", "DATE", "Calendar date type.")),
            "द्वैतम्" => Some(("Data Type", "BOOLEAN", "True/false type.")),
            "बद्धः" | "बद्धा" | "बद्धम्" => Some(("Constraint (विशेषणम्)", "REFERENCES <table>", "A foreign key, \"tied to\" the locative-plural table (`ग्राहकेषु बद्धा ग्राहकसङ्ख्या`); agrees with the column.")),
            _ => None,
        }
    }

    fn collect_token_level_hover_symbols(
        lexer_tokens: &[LexerToken],
        enriched_tokens: &[VibhaktiToken],
        schema: &Schema,
        symbols: &mut Vec<SymbolInfo>,
    ) {
        for (idx, vtok) in enriched_tokens.iter().enumerate() {
            let span = (idx, idx + 1);
            if symbols.iter().any(|s| idx >= s.span.0 && idx < s.span.1) {
                continue;
            }
            if let Some((kind, sql_equiv, desc)) = Self::describe_keyword_info(&vtok.text) {
                symbols.push(SymbolInfo {
                    name: vtok.text.clone(),
                    resolved: Some(sql_equiv.to_string()),
                    kind: kind.to_string(),
                    span,
                    description: desc.to_string(),
                    container: None,
                });
                continue;
            }

            let stem = vtok
                .pratipadika
                .clone()
                .unwrap_or_else(|| vtok.text.clone());

            if let Some(first) = vtok.possible_forms.first() {
                let resolved = schema
                    .resolve_table(&vtok.text)
                    .or_else(|| schema.resolve_table(&stem))
                    .map(|t| format!("table `{}`", t))
                    .or_else(|| {
                        schema
                            .resolve_column(&vtok.text, None)
                            .or_else(|| schema.resolve_column(&stem, None))
                            .map(|(c, dt, _)| format!("column `{}` ({})", c, dt))
                    });
                let linga_str = match first.linga {
                    Some(Linga::Masculine) => "पुंलिङ्गम् (Masculine)",
                    Some(Linga::Feminine) => "स्त्रीलिङ्गम् (Feminine)",
                    Some(Linga::Neuter) => "नपुंसकलिङ्गम् (Neuter)",
                    None => "—",
                };
                symbols.push(SymbolInfo {
                    name: format!("{} (stem: {})", vtok.text, stem),
                    resolved,
                    kind: "Subanta (सुबन्तम्)".to_string(),
                    span,
                    description: format!(
                        "विभक्तिः: {} | वचनम्: {} | लिङ्गम्: {}",
                        first.vibhakti, first.vachana, linga_str
                    ),
                    container: None,
                });
            } else if let Some(ltok) = lexer_tokens.get(idx) {
                match &ltok.token_type {
                    TokenType::Number(n) => {
                        symbols.push(SymbolInfo {
                            name: vtok.text.clone(),
                            resolved: Some(format!("{}", n)),
                            kind: "Number (सङ्ख्या)".to_string(),
                            span,
                            description: "Devanagari numeric literal".to_string(),
                            container: None,
                        });
                    }
                    TokenType::String(ref s) => {
                        symbols.push(SymbolInfo {
                            name: vtok.text.clone(),
                            resolved: Some(format!("'{}'", s)),
                            kind: "String Literal (अक्षरमाला)".to_string(),
                            span,
                            description: "String literal constant".to_string(),
                            container: None,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    fn build_schema_metadata(schema: &Schema) -> SchemaMetadata {
        let mut tables: Vec<TableMetadata> = schema
            .tables
            .iter()
            .map(|(name, table_def)| {
                let mut columns: Vec<String> = table_def.fields.keys().cloned().collect();
                columns.sort();
                TableMetadata {
                    name: name.clone(),
                    columns,
                }
            })
            .collect();
        tables.sort_by(|a, b| a.name.cmp(&b.name));
        SchemaMetadata { tables }
    }

    fn empty_analysis_with_schema(schema: &Schema) -> AnalysisData {
        AnalysisData {
            symbols: Vec::new(),
            schema_metadata: Self::build_schema_metadata(schema),
        }
    }

    fn make_error_diag(range: Range, source_name: &str, message: String) -> Diagnostic {
        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some(source_name.to_string()),
            message,
            related_information: None,
            tags: None,
            data: None,
        }
    }

    fn line_range(line: u32) -> Range {
        Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: 120,
            },
        }
    }

    fn extract_lexer_error_range(source: &str, error_msg: &str) -> Range {
        // The compiler reports positions as "at byte offset N".
        if let Ok(re) = Regex::new(r"byte offset (\d+)") {
            if let Some(caps) = re.captures(error_msg) {
                if let Some(m) = caps.get(1) {
                    if let Ok(byte_pos) = m.as_str().parse::<usize>() {
                        let start = Self::offset_to_position(source, byte_pos);
                        let end_byte = source[byte_pos.min(source.len())..]
                            .chars()
                            .next()
                            .map(|c| byte_pos + c.len_utf8())
                            .unwrap_or(byte_pos);
                        let end = Self::offset_to_position(source, end_byte);
                        return Range { start, end };
                    }
                }
            }
        }
        Self::line_range(0)
    }

    fn find_token_range_from_message(
        source: &str,
        tokens: &[LexerToken],
        error_msg: &str,
    ) -> Range {
        // A parse or annotator error names the byte offset of its token.
        if let Ok(re_offset) = Regex::new(r"byte offset (\d+)") {
            if let Some(offset) = re_offset.captures(error_msg).and_then(|c| c[1].parse::<usize>().ok()) {
                if let Some(idx) = tokens.iter().position(|t| t.position == offset) {
                    if let Some(r) = Self::token_span_to_range_with_tokens(source, tokens, (idx, idx + 1)) {
                        return r;
                    }
                }
                return Self::extract_lexer_error_range(source, error_msg);
            }
        }
        if let Ok(re_span) = Regex::new(r"at (\d+)\.\.(\d+)") {
            if let Some(caps) = re_span.captures(error_msg) {
                if let (Some(s), Some(e)) = (caps.get(1), caps.get(2)) {
                    if let (Ok(start_idx), Ok(end_idx)) =
                        (s.as_str().parse::<usize>(), e.as_str().parse::<usize>())
                    {
                        if let Some(r) =
                            Self::token_span_to_range_with_tokens(source, tokens, (start_idx, end_idx))
                        {
                            return r;
                        }
                    }
                }
            }
        }

        if let Ok(re_quote) = Regex::new(r#"[`'"]([^`'"]+)[`'"]"#) {
            for caps in re_quote.captures_iter(error_msg) {
                if let Some(m) = caps.get(1) {
                    let needle = m.as_str();
                    if let Some((idx, _)) = tokens.iter().enumerate().find(|(_, t)| t.text == needle) {
                        if let Some(r) =
                            Self::token_span_to_range_with_tokens(source, tokens, (idx, idx + 1))
                        {
                            return r;
                        }
                    }
                }
            }
        }

        if let Ok(re_line) = Regex::new(r"at line (\d+)") {
            if let Some(caps) = re_line.captures(error_msg) {
                if let Some(m) = caps.get(1) {
                    let line_num = m.as_str().parse::<u32>().unwrap_or(1).saturating_sub(1);
                    return Self::line_range(line_num);
                }
            }
        }

        Self::line_range(0)
    }

    fn append_lint_diagnostics(source: &str, diagnostics: &mut Vec<Diagnostic>) {
        for lint_diag in ssql_compiler::linter::lint_ssql_source(source) {
            let line_idx = lint_diag.line.saturating_sub(1) as u32;
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: line_idx,
                        character: 0,
                    },
                    end: Position {
                        line: line_idx,
                        character: 120,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::String(lint_diag.code.to_string())),
                code_description: None,
                source: Some("ssql-linter".to_string()),
                message: format!("{} (suggestion: {})", lint_diag.message, lint_diag.suggestion),
                related_information: None,
                tags: None,
                data: None,
            });
        }
    }

    async fn validate_document(&self, uri: Url, text: String) {
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| Path::new("memory.सं").to_path_buf());

        // 1. Update document cache
        {
            let mut docs = self.documents.write().unwrap();
            docs.insert(uri.clone(), text.clone());
        }

        // 2. Run quiet in-memory analysis + diagnostics (with schema.सं auto-discovery)
        let (analysis_opt, diagnostics) = Self::analyze_and_diagnose(&text, &path);
        if let Some(data) = analysis_opt {
            let mut analysis_cache = self.analysis.write().unwrap();
            analysis_cache.insert(uri.clone(), data);
        }

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }

    fn position_to_offset(source: &str, pos: Position) -> usize {
        let mut line_start = 0;
        for _ in 0..pos.line {
            if let Some(idx) = source[line_start..].find('\n') {
                line_start += idx + 1;
            } else {
                return source.len();
            }
        }

        let remainder = &source[line_start..];
        let mut char_count = 0;
        let mut byte_count = 0;

        for c in remainder.chars() {
            if c == '\n' {
                break;
            }
            let len_utf16 = c.len_utf16();
            if char_count + len_utf16 > pos.character as usize {
                break;
            }
            char_count += len_utf16;
            byte_count += c.len_utf8();
        }
        line_start + byte_count
    }

    fn offset_to_position(source: &str, offset: usize) -> Position {
        if offset >= source.len() {
            let prefix = source;
            let line = prefix.chars().filter(|&c| c == '\n').count() as u32;
            let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
            let col_str = &prefix[line_start..];
            let character = col_str.chars().map(|c| c.len_utf16() as u32).sum();
            return Position { line, character };
        }

        let prefix = &source[..offset];
        let line = prefix.chars().filter(|&c| c == '\n').count() as u32;

        let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col_str = &prefix[line_start..];
        let character = col_str.chars().map(|c| c.len_utf16() as u32).sum();

        Position { line, character }
    }

    fn get_token_index(source: &str, byte_offset: usize) -> Option<usize> {
        if let Ok(tokens) = tokenize(source) {
            for (idx, token) in tokens.iter().enumerate() {
                let start = token.position;
                let end = start + token.text.len();
                if byte_offset >= start && byte_offset < end {
                    return Some(idx);
                }
            }
        }
        None
    }

    fn token_span_to_range_with_tokens(
        source: &str,
        tokens: &[LexerToken],
        span: (usize, usize),
    ) -> Option<Range> {
        if tokens.is_empty() {
            return None;
        }
        if span.0 < tokens.len() && span.1 <= tokens.len() {
            let start_token = &tokens[span.0];
            let end_token = if span.1 > span.0 {
                &tokens[span.1 - 1]
            } else {
                start_token
            };

            let start_offset = start_token.position;
            let end_offset = end_token.position + end_token.text.len();

            let start_pos = Backend::offset_to_position(source, start_offset);
            let end_pos = Backend::offset_to_position(source, end_offset);

            return Some(Range {
                start: start_pos,
                end: end_pos,
            });
        } else if span.0 >= tokens.len() {
            let last_token = tokens.last()?;
            let start_offset = last_token.position;
            let end_offset = last_token.position + last_token.text.len();
            let start_pos = Backend::offset_to_position(source, start_offset);
            let end_pos = Backend::offset_to_position(source, end_offset);
            return Some(Range {
                start: start_pos,
                end: end_pos,
            });
        }
        None
    }

    fn token_span_to_range(source: &str, span: (usize, usize)) -> Option<Range> {
        if let Ok(tokens) = tokenize(source) {
            return Self::token_span_to_range_with_tokens(source, &tokens, span);
        }
        None
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), " ".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "SSQL Language Server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.validate_document(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            self.validate_document(uri, change.text).await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let analysis_map = self.analysis.read().unwrap();
        let docs_map = self.documents.read().unwrap();

        if let (Some(data), Some(source)) = (analysis_map.get(&uri), docs_map.get(&uri)) {
            return Ok(Backend::get_hover_logic(data, source, pos));
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let analysis_map = self.analysis.read().unwrap();
        let docs_map = self.documents.read().unwrap();

        if let (Some(data), Some(source)) = (analysis_map.get(&uri), docs_map.get(&uri)) {
            if let Some(range) = Backend::get_definition_range(data, source, pos) {
                return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: uri.clone(),
                    range,
                })));
            }

            // Cross-file fallback: if symbol is defined in ancestor `schema.सं`, jump there
            let offset = Backend::position_to_offset(source, pos);
            if let Some(t_idx) = Backend::get_token_index(source, offset) {
                if let Some(sym) = data
                    .symbols
                    .iter()
                    .find(|s| t_idx >= s.span.0 && t_idx < s.span.1)
                {
                    if let Ok(path) = uri.to_file_path() {
                        if let Some(schema_path) = Backend::find_schema_file(&path) {
                            if let (Ok(schema_src), Ok(schema_uri)) = (
                                std::fs::read_to_string(&schema_path),
                                Url::from_file_path(&schema_path),
                            ) {
                                let (schema_analysis, _) =
                                    Backend::analyze_and_diagnose(&schema_src, &schema_path);
                                if let Some(s_data) = schema_analysis {
                                    if let Some(target) = s_data
                                        .symbols
                                        .iter()
                                        .find(|t| t.name == sym.name && t.description.contains("Definition"))
                                        .or_else(|| s_data.symbols.iter().find(|t| t.name == sym.name))
                                    {
                                        if let Some(range) =
                                            Backend::token_span_to_range(&schema_src, target.span)
                                        {
                                            return Ok(Some(GotoDefinitionResponse::Scalar(
                                                Location {
                                                    uri: schema_uri,
                                                    range,
                                                },
                                            )));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let analysis_map = self.analysis.read().unwrap();

        if let Some(data) = analysis_map.get(&uri) {
            let items = Backend::get_completion_items(data);
            return Ok(Some(CompletionResponse::Array(items)));
        }

        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let docs_map = self.documents.read().unwrap();
        if let Some(source) = docs_map.get(&uri) {
            let formatted = ssql_compiler::formatter::format_ssql_source(source);
            let end_pos = Backend::offset_to_position(source, source.len());
            return Ok(Some(vec![TextEdit {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: end_pos,
                },
                new_text: formatted,
            }]));
        }
        Ok(None)
    }
}

impl Backend {
    fn get_hover_logic(data: &AnalysisData, source: &str, pos: Position) -> Option<Hover> {
        let offset = Backend::position_to_offset(source, pos);
        let tok_idx = Backend::get_token_index(source, offset);

        if let Some(t_idx) = tok_idx {
            for sym in &data.symbols {
                if t_idx >= sym.span.0 && t_idx < sym.span.1 {
                    let mut contents = Vec::new();
                    contents.push(MarkedString::String(format!("{}: {}", sym.kind, sym.name)));
                    if let Some(res) = &sym.resolved {
                        if let Some(container) = &sym.container {
                            contents.push(MarkedString::String(format!(
                                "Resolved: {}.{}",
                                container, res
                            )));
                        } else {
                            contents.push(MarkedString::String(format!("Resolved: {}", res)));
                        }
                    }
                    if !sym.description.is_empty() {
                        contents.push(MarkedString::String(sym.description.clone()));
                    }
                    return Some(Hover {
                        contents: HoverContents::Array(contents),
                        range: Backend::token_span_to_range(source, sym.span),
                    });
                }
            }
        }
        None
    }

    fn get_definition_range(data: &AnalysisData, source: &str, pos: Position) -> Option<Range> {
        let offset = Backend::position_to_offset(source, pos);
        let tok_idx = Backend::get_token_index(source, offset)?;

        for sym in &data.symbols {
            if t_idx_in_span(tok_idx, sym.span) {
                if sym.kind.starts_with("Verb")
                    || sym.kind.starts_with("Particle")
                    || sym.kind.starts_with("Conjunction")
                    || sym.kind.starts_with("Clause")
                    || sym.kind.starts_with("Postposition")
                    || sym.kind.starts_with("Modifier")
                    || sym.kind.starts_with("Pronoun")
                    || sym.kind.starts_with("Comparison")
                    || sym.kind.starts_with("Aggregate")
                    || sym.kind.starts_with("Window")
                    || sym.kind.starts_with("DDL")
                    || sym.kind.starts_with("Data Type")
                    || sym.kind.starts_with("Absolutive")
                    || sym.kind.starts_with("Number")
                    || sym.kind.starts_with("String")
                {
                    continue;
                }
                let target_name = &sym.name;
                // Prefer explicit definition symbols first, then fall back to first occurrence
                if let Some(def_sym) = data
                    .symbols
                    .iter()
                    .find(|t| &t.name == target_name && t.description.contains("Definition"))
                    .or_else(|| data.symbols.iter().find(|t| &t.name == target_name))
                {
                    return Backend::token_span_to_range(source, def_sym.span);
                }
            }
        }
        None
    }

    fn get_completion_items(data: &AnalysisData) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        // Every fixed SSQL word, from the compiler itself.
        let keywords = ssql_compiler::vocabulary();
        for kw in keywords {
            let (detail, doc) = if let Some((kind, sql_eq, desc)) = Self::describe_keyword_info(kw)
            {
                (
                    format!("{} — {}", sql_eq, kind),
                    Some(Documentation::String(desc.to_string())),
                )
            } else {
                ("SSQL Keyword".to_string(), None)
            };
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(detail),
                documentation: doc,
                ..Default::default()
            });
        }
        // Tables & Columns from discovered schema.सं and in-file DDL/CTEs (with declined forms)
        let mut morph = MorphologyEngine::new().ok();
        for table in &data.schema_metadata.tables {
            items.push(CompletionItem {
                label: table.name.clone(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some("Table Stem (प्रातिपदिकम्)".to_string()),
                documentation: Some(Documentation::String(format!(
                    "Columns: {}",
                    table.columns.join(", ")
                ))),
                ..Default::default()
            });
            if let Some(ref mut m) = morph {
                for gender in [Linga::Neuter, Linga::Masculine, Linga::Feminine] {
                    if let Ok(bahu) = m.decline_with_gender(&table.name, gender, Vachana::Bahu) {
                        if let Some(panchami) = bahu.forms.get(&Vibhakti::Panchami) {
                            items.push(CompletionItem {
                                label: panchami.clone(),
                                kind: Some(CompletionItemKind::CLASS),
                                detail: Some(format!("FROM {} (पञ्चमी बहुवचनम्)", table.name)),
                                ..Default::default()
                            });
                        }
                        if let Some(tritiya) = bahu.forms.get(&Vibhakti::Tritiya) {
                            items.push(CompletionItem {
                                label: tritiya.clone(),
                                kind: Some(CompletionItemKind::CLASS),
                                detail: Some(format!("JOIN {} (तृतीया बहुवचनम्)", table.name)),
                                ..Default::default()
                            });
                        }
                        break;
                    }
                }
                if let Ok(eka) = m.decline_with_gender(&table.name, Linga::Neuter, Vachana::Eka) {
                    if let Some(shashthi) = eka.forms.get(&Vibhakti::Shashthi) {
                        items.push(CompletionItem {
                            label: shashthi.clone(),
                            kind: Some(CompletionItemKind::CLASS),
                            detail: Some(format!("{}. (षष्ठी एकवचनम्)", table.name)),
                            ..Default::default()
                        });
                    }
                }
            }
            for col in &table.columns {
                items.push(CompletionItem {
                    label: col.clone(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(format!("Column Stem ({})", table.name)),
                    ..Default::default()
                });
            }
        }
        items
    }
}

fn t_idx_in_span(tok_idx: usize, span: (usize, usize)) -> bool {
    tok_idx >= span.0 && tok_idx < span.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_position() {
        let src = "Hello\nWorld";
        assert_eq!(
            Backend::offset_to_position(src, 0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            Backend::offset_to_position(src, 6),
            Position {
                line: 1,
                character: 0
            }
        );
    }

    #[test]
    fn test_hover_logic() {
        let sym = SymbolInfo {
            name: "परीक्षणम्".to_string(),
            resolved: Some("test_sql".to_string()),
            kind: "Variable".to_string(),
            span: (0, 1),
            description: "Desc".to_string(),
            container: None,
        };
        let data = AnalysisData {
            symbols: vec![sym],
            schema_metadata: SchemaMetadata { tables: vec![] },
        };
        let src = "परीक्षणम् क्रीडा";

        let result = Backend::get_hover_logic(
            &data,
            src,
            Position {
                line: 0,
                character: 0,
            },
        );
        assert!(result.is_some());
        let hover = result.unwrap();
        if let HoverContents::Array(arr) = hover.contents {
            assert!(arr.iter().any(|m| match m {
                MarkedString::String(s) => s.contains("Variable: परीक्षणम्"),
                _ => false,
            }));
        } else {
            panic!("Wrong hover content");
        }

        let result = Backend::get_hover_logic(
            &data,
            src,
            Position {
                line: 0,
                character: 12,
            },
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_completion_logic() {
        let table = TableMetadata {
            name: "Books".to_string(),
            columns: vec!["Price".to_string()],
        };
        let data = AnalysisData {
            symbols: vec![],
            schema_metadata: SchemaMetadata {
                tables: vec![table],
            },
        };

        let items = Backend::get_completion_items(&data);
        assert!(items.iter().any(|i| i.label == "आह्रियताम्"));
        assert!(items.iter().any(|i| i.label == "प्रदाय"));
        assert!(items.iter().any(|i| i.label == "Books"));
        assert!(items.iter().any(|i| i.label == "Price"));
    }

    #[test]
    fn test_editor_verdict_matches_the_cli() {
        let query_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("examples/university_bookstore/SAMSQL/01_books_published_after_2010.सं");
        let errors = |src: &str| -> Vec<String> {
            Backend::analyze_and_diagnose(src, &query_path)
                .1
                .into_iter()
                .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
                .map(|d| d.message)
                .collect()
        };
        // Each of these is rejected by the CLI, so the editor must flag it.
        for (src, expected) in [
            ("लेखकेभ्यः नाम ।", "Missing action verb"),
            ("लेखकेभ्यः नाम आह्रियताम्", "must end with '।'"),
            ("पुस्तकेभ्यः मूल्यस्य सरासरी आह्रियताम् ।", "Use `माध्यम्`"), // lexicon-allow
            ("पुस्तकेभ्यः निष्कास्यताम् ।", "Parse errors"),
        ] {
            let found = errors(src);
            assert!(found.iter().any(|m| m.contains(expected)), "{}: expected `{}` in {:?}", src, expected, found);
        }
        // And a query the CLI accepts shows no error.
        assert!(errors("पुस्तकेभ्यः मूल्यस्य माध्यम् आह्रियताम् ।").is_empty());
    }

    #[test]
    fn test_schema_discovery_and_multiline_diagnostics() {
        let query_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("examples/university_bookstore/SAMSQL/01_books_published_after_2010.सं");

        // 1. Valid multi-line query against discovered schema.सं should produce 0 error diagnostics
        //    and populate schema_metadata.tables!
        let valid_src = "पुस्तकेभ्यः\n    शीर्षकम् च\n    मूल्यम् च\nआह्रियेताम् ।";
        let (analysis_opt, diags) = Backend::analyze_and_diagnose(valid_src, &query_path);
        let analysis = analysis_opt.expect("analysis should succeed");
        assert!(
            !analysis.schema_metadata.tables.is_empty(),
            "Expected schema.सं tables to be auto-discovered"
        );
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .collect();
        assert!(errors.is_empty(), "Expected no errors on valid query, got: {:?}", errors);

        // 2. Verb number mismatch on line 4 (0-indexed line 3) should highlight line 3, NOT line 0!
        let bad_verb_src = "पुस्तकेभ्यः\n    शीर्षकम् च\n    मूल्यम् च\nआह्रियताम् ।";
        let (_, bad_diags) = Backend::analyze_and_diagnose(bad_verb_src, &query_path);
        let verb_err = bad_diags
            .iter()
            .find(|d| d.message.contains("Subject-Verb Agreement"))
            .expect("Expected Subject-Verb Agreement diagnostic");
        assert_eq!(
            verb_err.range.start.line, 3,
            "Expected error on line 3 (4th line where आह्रियताम् sits), got: {:?}",
            verb_err.range
        );
    }

    #[test]
    fn test_in_file_ddl_and_cte_goto_definition() {
        let src = r#"वैद्याः इति पटलम् "doctors" इति आख्या वैद्य इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    सङ्ख्या इति पूर्णाङ्कः "id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    नाम इति वाक्यम् "name" इति आख्या नामन् इति नपुंसकलिङ्गम् एकवचनम् च
इति पटलम् ।
वैद्येभ्यः नाम आह्रियताम् ।"#;
        let dummy_path = Path::new("/tmp/nonexistent_dir_ssql_lsp_test/query.सं");
        let (analysis_opt, diags) = Backend::analyze_and_diagnose(src, dummy_path);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .collect();
        assert!(errors.is_empty(), "Expected 0 errors for in-file DDL + query: {:?}", errors);

        let analysis = analysis_opt.unwrap();
        assert!(
            analysis.schema_metadata.tables.iter().any(|t| t.name == "वैद्य"),
            "In-file DDL table should be registered in schema_metadata"
        );

        // Go-to-definition on `वैद्येभ्यः` (line 4, character 2) should jump to `वैद्याः` on line 0!
        let def_range = Backend::get_definition_range(
            &analysis,
            src,
            Position {
                line: 4,
                character: 2,
            },
        )
        .expect("Expected goto_definition range for वैद्येभ्यः");
        assert_eq!(def_range.start.line, 0, "Expected definition of वैद्येभ्यः on line 0");
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Arc::new(RwLock::new(HashMap::new())),
        analysis: Arc::new(RwLock::new(HashMap::new())),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
