//! # SSQL Parser
//!
//! A chumsky-based combinator parser for SSQL that produces AST nodes.
//!
//! ## Architecture
//! ```text
//! LexerToken → ParserToken → chumsky parser → AST (DqlStatement, etc.)
//! ```
//!
//! The grammar is split across sibling modules by logical area:
//! - `token`: the `Token` enum + LexerToken/VibhaktiToken conversion (no chumsky dependency)
//! - `common`: shared low-level atom/identifier parsers used by 3+ other modules
//! - `expr`: the recursive expression grammar core (owns the main `recursive(|expr| ...)` knot)
//! - `clauses`: DQL clause parsers (SELECT/FROM/JOIN/WHERE/GROUP BY/HAVING/ORDER BY/LIMIT)
//! - `ddl`: CREATE/DROP TABLE/VIEW/INDEX, ALTER TABLE, and Pure UDF definitions
//! - `dml`: INSERT/UPDATE/DELETE, DCL, and TCL statements
//! - `set_ops`: UNION/INTERSECT/EXCEPT (owns its own `recursive(...)` knot)
//! - `cte`: variable/CTE declarations
//!
//! This file (`mod.rs`) holds the top-level entry points that tie the whole
//! grammar together: `parse_statement`, `parse_query_batch` (where the
//! `expr_parser`/`dql` recursive handles actually get constructed and handed
//! down to every other module), and the public `parse()` API.

use chumsky::prelude::*;
use crate::ast::{Statement, Expression, DqlStatement};

mod diagnose;
pub mod token;
mod common;
mod expr;
mod clauses;
mod ddl;
mod dml;
mod set_ops;
mod cte;

pub use token::{convert_tokens, convert_vibhakti_tokens, Token};
pub use expr::{parse_expression, parse_logical_expression};
pub use clauses::parse_dql;

use common::parse_any_identifier;
use clauses::parse_explain_dql;
use ddl::parse_ddl;
use dml::{parse_delete_statement, parse_update_statement, parse_insert_statement, parse_dcl, parse_tcl_statement};
use set_ops::parse_set_operation;
use cte::{parse_var_decl, parse_parameter_decl};

// PARSE_TIMEOUT: wall-clock budget enforced around the top-level parse() call.
// Acts as a process-boundary safeguard against deeply nested backtracking.
#[cfg(not(target_arch = "wasm32"))]
const PARSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub fn parse_statement<'a>(
      expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], Statement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // Variable Declaration
    let var_decl = parse_var_decl(expr_parser.clone(), parse_dql(expr_parser.clone()))
        .map(|(name, value, is_recursive, cycle_col)| Statement::VarDecl {
            name, 
            value: Box::new(value),
            is_recursive,
            cycle_col,
        });

    // Set Operations
    let set_op = parse_set_operation(expr_parser.clone());

    // DQL - instantiated with fresh expression parser
    let dql = parse_dql(expr_parser.clone())
        .map(Statement::Dql);

    let query_parser = choice((
        set_op.clone(),
        dql.clone(),
    ));

    // DML
    let delete = parse_delete_statement(expr_parser.clone());
    let update = parse_update_statement(expr_parser.clone());
    let insert = parse_insert_statement(expr_parser.clone(), parse_dql(expr_parser.clone()));

    // DDL
    let ddl = parse_ddl(parse_any_identifier(), query_parser, expr_parser.clone())
        .map(Statement::Ddl);

    // DCL
    let dcl = parse_dcl()
        .map(Statement::Dcl);

    // TCL
    let tcl = parse_tcl_statement()
        .map(Statement::Tcl);

    // Explain (Postfix)
    let explain = parse_explain_dql(expr_parser.clone());

    choice((
        explain,
        set_op,
        delete,
        update,
        insert,
        ddl,
        dcl,
        tcl,
        var_decl,
        dql,
    ))
}


pub fn parse_query_batch<'a>() -> impl Parser<'a, &'a [Token], crate::ast::AstNode, extra::Err<Rich<'a, Token>>> + Clone {
    let params = parse_parameter_decl().repeated().collect::<Vec<_>>();
    
    // Construct properly recursive DQL parser
    let dql = recursive(|dql_handle| {
        // See the identical reasoning in clauses.rs's parse_dql: a bare
        // (unparenthesized) embedded subquery is only unambiguous when it
        // carries its own explicit आह्रियताम्.
        let subquery = choice((
            dql_handle.clone()
                .delimited_by(just(Token::Yat), just(Token::Iti)),
            dql_handle.clone()
                .try_map(|d: DqlStatement, span| if d.verb_token.is_some() {
                    Ok(d)
                } else {
                    Err(Rich::custom(span, "Bare embedded subquery requires an explicit आह्रियताम्"))
                }),
        ))
            .map(|d| Expression::Subquery(Box::new(d)))
            .boxed();
        let expr = parse_logical_expression(subquery);
        parse_dql(expr)
    });

    // Construct Expression parser that uses the recursive DQL parser (for top-level statements like SetOp)
    let expr_parser = recursive(|expr_handle| {
        // A set operation used as a value (e.g. an IN list) is grouped like
        // any embedded statement: `यत् A च B च इत्यनयोः सङ्गमः इति`.
        let set_op_expr = parse_set_operation(expr_handle.clone())
            .delimited_by(just(Token::Yat), just(Token::Iti))
            .map(|s| {
                if let Statement::SetOp(op) = s {
                    Expression::SetOp(Box::new(op))
                } else {
                    unreachable!()
                }
            });

        let set_op_expr_raw = parse_set_operation(expr_handle.clone())
            .map(|s| {
                if let Statement::SetOp(op) = s {
                    Expression::SetOp(Box::new(op))
                } else {
                    unreachable!()
                }
            });

        let subquery_final = choice((
            dql.clone()
                .delimited_by(just(Token::Yat), just(Token::Iti))
                .map(|d| Expression::Subquery(Box::new(d))),
            // Bare form only when the subquery carries its own explicit
            // आह्रियताम् - see the identical reasoning in clauses.rs::parse_dql.
            dql.clone()
                .try_map(|d: DqlStatement, span| if d.verb_token.is_some() {
                    Ok(d)
                } else {
                    Err(Rich::custom(span, "Bare embedded subquery requires an explicit आह्रियताम्"))
                })
                .map(|d| Expression::Subquery(Box::new(d))),
            set_op_expr,
            set_op_expr_raw,
        )).boxed();

        parse_logical_expression(subquery_final)
    });
    
    // Parse one or more top-level SSQL statements (DQL, DML, DDL, TCL, CTEs, SetOps)
    let statements = parse_statement(expr_parser)
        .then_ignore(just(Token::Purnadanda).or_not())
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>();

    params
        .then(statements)
        .then_ignore(end())
        .map(|(parameters, statements)| crate::ast::AstNode::QueryBatch {
            parameters,
            statements,
        })
}


// =============================================================================
// Public API
// =============================================================================

/// Parse SSQL source into an AST
///
/// Runs on a dedicated thread with a hard wall-clock budget (`PARSE_TIMEOUT`)
/// so that the caller receives a clean error within `PARSE_TIMEOUT` even on
/// pathological inputs.


/// The message of the marker `grouped` emits for each group on the final
/// parse; it carries the group's token span and is never shown.
pub(crate) const GROUP_MARK: &str = "\u{1}group";

/// The parse and the token spans of its `यत् … इति` groups (`GROUP_MARK`).
/// chumsky drops emitted errors on backtracking, so only the groups of the
/// parse that succeeded are left.
fn parse_with_groups(tokens: &[Token]) -> Result<(crate::ast::AstNode, Vec<(usize, usize)>), Vec<String>> {
    let (output, errors) = parse_query_batch().parse(tokens).into_output_errors();
    let (marks, errors): (Vec<_>, Vec<_>) = errors.into_iter().partition(
        |e| matches!(e.reason(), chumsky::error::RichReason::Custom(m) if m == GROUP_MARK),
    );
    match output {
        Some(mut ast) if errors.is_empty() => {
            ast.desugar_derived_tables();
            Ok((ast, marks.iter().map(|e| (e.span().start, e.span().end)).collect()))
        }
        _ => match diagnose::diagnose(tokens) {
            Some(message) => Err(vec![message]),
            None => Err(errors.iter().map(describe_error).collect()),
        },
    }
}

fn parse_on_current_thread(tokens: &[Token]) -> Result<crate::ast::AstNode, Vec<String>> {
    parse_with_groups(tokens).map(|(ast, _)| ast)
}

/// The token index of the `यत्` of a group that changes nothing: with it and
/// its `इति` removed, the rest parses to the same AST (approved decision
/// "Grouping": a group is an error wherever removing it yields an identical
/// AST). One re-parse per group of the parse (`parse_with_groups`).
fn redundant_group(tokens: &[Token], ast: &crate::ast::AstNode, groups: &[(usize, usize)]) -> Option<usize> {
    let shape = without_spans(&format!("{:?}", ast));
    groups.iter().map(|&(start, end)| (start, end - 1)).find(|&(start, last)| {
        if !matches!(tokens.get(start), Some(Token::Yat)) || !matches!(tokens.get(last), Some(Token::Iti)) {
            return false;
        }
        let mut rest = tokens.to_vec();
        rest.remove(last);
        rest.remove(start);
        parse_on_current_thread(&rest).is_ok_and(|other| without_spans(&format!("{:?}", other)) == shape)
    }).map(|(start, _)| start)
}

/// An AST's debug text without its source spans, which shift when tokens
/// are removed.
fn without_spans(debug: &str) -> String {
    let mut out = String::with_capacity(debug.len());
    let mut rest = debug;
    while let Some(k) = rest.find("span: Some((") {
        out.push_str(&rest[..k]);
        let tail = &rest[k..];
        rest = &tail[tail.find("))").map_or(tail.len(), |e| e + 2)..];
    }
    out.push_str(rest);
    out
}

/// Parse, then reject a group that changes nothing (`redundant_group`).
fn parse_checked_on_current_thread(tokens: &[Token]) -> Result<crate::ast::AstNode, Vec<String>> {
    let (ast, groups) = parse_with_groups(tokens)?;
    match redundant_group(tokens, &ast, &groups) {
        Some(at) => Err(vec![format!(
            "this `यत् … इति` group changes nothing, since the query reads the same without it: remove `यत्` and its `इति` at {}..{}",
            at, at + 1
        )]),
        None => Ok(ast),
    }
}

/// One parse error in words: the expected and found tokens as the SSQL words
/// they are, and the token range ` at a..b`, which the compiler turns into a
/// byte offset.
fn describe_error(e: &Rich<'_, Token>) -> String {
    use chumsky::error::{RichPattern, RichReason};
    let at = format!(" at {}..{}", e.span().start, e.span().end);
    match e.reason() {
        RichReason::Custom(msg) => {
            let reserved = msg.contains("Expected Identifier");
            match (reserved, e.found()) {
                (true, Some(found)) => format!(
                    "`{}` is an SSQL keyword, which cannot be used as a table, column or alias name here{}",
                    found, at
                ),
                _ => format!("{}{}", msg, at),
            }
        }
        RichReason::ExpectedFound { expected, found } => {
            let found = found.as_ref().map(|t| format!("`{}`", **t)).unwrap_or_else(|| "the end of the text".to_string());
            let mut wanted: Vec<String> = expected
                .iter()
                .filter_map(|p| match p {
                    RichPattern::Token(t) => Some(format!("`{}`", **t)),
                    RichPattern::Label(l) => Some(l.to_string()),
                    RichPattern::EndOfInput => Some("the end of the statement".to_string()),
                    _ => None,
                })
                .collect();
            wanted.sort();
            wanted.dedup();
            match wanted.len() {
                0 => format!("unexpected {}{}", found, at),
                1 => format!("found {} where {} was expected{}", found, wanted[0], at),
                n => format!("found {} where one of {} or {} was expected{}", found, wanted[..n - 1].join(", "), wanted[n - 1], at),
            }
        }
    }
}

/// WebAssembly has no threads, so the browser runs the same parser inline
/// (without the wall-clock budget); the grammar and the error text are
/// identical to the native build.
#[cfg(target_arch = "wasm32")]
pub fn parse(tokens: Vec<Token>) -> Result<crate::ast::AstNode, Vec<String>> {
    parse_on_current_thread(&tokens)
}

/// `parse`, also rejecting a group that changes nothing. The compiler's
/// final parse uses it; the schema pass does not need it.
#[cfg(target_arch = "wasm32")]
pub fn parse_checked(tokens: Vec<Token>) -> Result<crate::ast::AstNode, Vec<String>> {
    parse_checked_on_current_thread(&tokens)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse(tokens: Vec<Token>) -> Result<crate::ast::AstNode, Vec<String>> {
    parse_with_budget(tokens, parse_on_current_thread)
}

/// `parse`, also rejecting a group that changes nothing. The compiler's
/// final parse uses it; the schema pass does not need it.
#[cfg(not(target_arch = "wasm32"))]
pub fn parse_checked(tokens: Vec<Token>) -> Result<crate::ast::AstNode, Vec<String>> {
    parse_with_budget(tokens, parse_checked_on_current_thread)
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_with_budget(
    tokens: Vec<Token>,
    run: fn(&[Token]) -> Result<crate::ast::AstNode, Vec<String>>,
) -> Result<crate::ast::AstNode, Vec<String>> {
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(run(&tokens));
    });
    match rx.recv_timeout(PARSE_TIMEOUT) {
        Ok(result) => result,
        Err(_) => Err(vec![format!(
            "Parsing this statement took more than {}s. Name its parts with a CTE (`<name> इति आख्या …`) and read them from there",
            PARSE_TIMEOUT.as_secs()
        )]),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::parse_logical_expression; // Defined in this file
    use crate::ast::Literal;
    use crate::types::Vachana;
    use clauses::parse_where_clause;
    
    #[test]
    fn test_multi_column_with_trailing_cha() {
        // Multi-column: source col1 च col2 च verb
        let tokens = vec![
            Token::Panchami { text: "पुस्तकेभ्यः".into(), vachana: Vachana::Bahu, pratipadika: "पुस्तकेभ्यः".into() },
            Token::Prathama { text: "शीर्षकम्".into(), vachana: Vachana::Eka, pratipadika: "शीर्षकम्".into() },
            Token::Cha,
            Token::Prathama { text: "मूल्यम्".into(), vachana: Vachana::Eka, pratipadika: "मूल्यम्".into() },
            Token::Cha,
            Token::FetchVerb("आह्रियेताम्".to_string()),
        ];
        
        let result = parse(tokens);
        assert!(result.is_ok(), "Multi-column parse failed: {:?}", result.err());
        
        let ast = result.unwrap();
        match ast {
            crate::ast::AstNode::QueryBatch { statements, .. } => {
                if let crate::ast::Statement::Dql(stmt) = &statements[0] {
                    assert_eq!(stmt.select.columns.len(), 2, "Should have 2 columns");
                } else {
                    panic!("Expected DQL statement");
                }
            },
            _ => panic!("Expected QueryBatch"),
        }
    }

    #[test]
    fn test_parse_parameter_decl() {
        // Param: target_price ITI BAHYA SANJNA
        let _tokens = vec![
            Token::Prathama { text: "लक्ष्यमूल्यम्".into(), vachana: Vachana::Eka, pratipadika: "लक्ष्यमूल्यम्".into() },
            Token::Iti,
            Token::Bahya,
            Token::Sanjna,
            Token::Purnadanda,
            Token::Eof,
        ];
        
        // Note: parse_parameter_decl is internal, but parse_query_batch uses it.
        // We'll test via parse which expects at least one statement, so let's add a dummy statement
        let full_tokens = vec![
            Token::Prathama { text: "लक्ष्यमूल्यम्".into(), vachana: Vachana::Eka, pratipadika: "लक्ष्यमूल्यम्".into() },
            Token::Iti,
            Token::Bahya,
            Token::Sanjna,
            Token::Purnadanda,
            // Dummy Select
            Token::Panchami { text: "पुस्तकेभ्यः".into(), vachana: Vachana::Bahu, pratipadika: "पुस्तकेभ्यः".into() },
            Token::Prathama { text: "शीर्षकम्".into(), vachana: Vachana::Eka, pratipadika: "शीर्षकम्".into() },
            Token::FetchVerb("आह्रियताम्".to_string()),
        ];
        
        let result = parse(full_tokens);
        assert!(result.is_ok(), "Parameter parse failed: {:?}", result.err());
        
        match result.unwrap() {
            crate::ast::AstNode::QueryBatch { parameters, .. } => {
                assert_eq!(parameters.len(), 1);
                assert_eq!(parameters[0].name.text, "लक्ष्यमूल्यम्");
            },
            _ => panic!("Expected QueryBatch"),
        }
    }

    #[test]
    fn test_parse_tritiya_comparison() {
        // अङ्कः ? इत्यनेन समः अस्ति इति नियमेन
        let tokens = vec![
            Token::Prathama { text: "अङ्कः".into(), vachana: Vachana::Eka, pratipadika: "अङ्कः".into() },
            Token::Prathama { text: "?".into(), vachana: Vachana::Eka, pratipadika: "?".into() },
            Token::Tritiya { text: "इत्यनेन".into(), vachana: Vachana::Eka, pratipadika: "इत्यनेन".into() },
            Token::Samam("समः".to_string()),
            Token::Asti,
            Token::Iti,
            Token::Niyamena,
        ];
        
        // Ensure checking against where clause parser or expression parser in test
        // But parse_where_clause now requires expr_parser arg.
        // So we must supply it.
        // And now parse_logical_expression requires subquery parser.
            let subquery_stub = any().filter(|_| false).map(|_| Expression::Literal(Literal::Number("0".to_string()))).boxed();
        let result = parse_where_clause(parse_logical_expression(subquery_stub.clone()), subquery_stub).parse(&tokens[..]).into_result();
        assert!(result.is_ok(), "Tritiya comparison failed: {:?}", result.err());
    }
        #[test]
    fn test_parse_union_with_trailing_cha() {
        // A Cha B Cha Ityanayoh Sangamah
        let tokens = vec![
            Token::Prathama { text: "A".into(), vachana: Vachana::Eka, pratipadika: "A".into() },
            Token::Cha,
            Token::Number("100".into()),
            Token::Cha,
            Token::Ityayoh("इत्यनयोः".to_string()),
            Token::Sangamah,
            Token::FetchVerb("आह्रियताम्".to_string()),
            Token::Purnadanda,
        ];
        
        let result = parse(tokens);
        assert!(result.is_ok(), "Union with trailing Cha failed: {:?}", result.err());

        match result.unwrap() {
            crate::ast::AstNode::QueryBatch { statements, .. } => {
                if let crate::ast::Statement::SetOp(op) = &statements[0] {
                    assert_eq!(op.queries.len(), 2);
                } else {
                    panic!("Expected SetOp statement");
                }
            },
            _ => panic!("Expected QueryBatch"),
        }
    }

    #[test]
    fn test_parse_aggregate_select_item_with_string_alias() {
        // `मूल्यस्य योगः "total_valuation" इति आख्या` -- SUM(price) AS "total_valuation":
        // the alias after an aggregate must not be dropped.
        let tokens = vec![
            Token::Shashthi { text: "मूल्यस्य".into(), vachana: Vachana::Eka, pratipadika: "मूल्य".into() },
            Token::Yogah,
            Token::StringLit("total_valuation".into()),
            Token::Iti,
            Token::Aakhya,
        ];
        let subquery_stub = any().filter(|_| false).map(|_| Expression::Literal(Literal::Number("0".to_string()))).boxed();
        let expr_parser = parse_logical_expression(subquery_stub.clone());
        let item_parser = super::clauses::parse_select_item(expr_parser, subquery_stub);
        let result = item_parser.parse(&tokens[..]).into_result();
        assert!(result.is_ok(), "select item with string alias failed: {:?}", result.err());
        assert_eq!(result.unwrap().alias, Some("total_valuation".to_string()));
    }

}
