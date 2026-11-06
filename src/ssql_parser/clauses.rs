//! DQL clause parsers: SELECT/FROM/JOIN/WHERE/GROUP BY/HAVING/ORDER BY/LIMIT,
//! plus the top-level DQL statement (`parse_dql`/`parse_dql_body`) and
//! EXPLAIN wrapper.

use chumsky::prelude::*;
use crate::ast::{
    Statement, Expression, Identifier, DqlStatement, SourceClause, JoinClause,
    WhereClause, GroupByClause, HavingClause, SelectClause, OrderByClause,
    LimitClause, JoinType, OrderDirection, OrderExpr,
};
use crate::types::{Vibhakti, Vachana, VibhaktiForm, SymbolType};
use super::Token;
use super::expr::{parse_expr_atom, parse_logical_expression};
use super::common::conj_list;

pub(crate) fn parse_expression_item<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // Top-level expression item in lists (Select, Order) cannot consume 'Cha' (AND/Comma).
    // So we rebuild the atom/unary/logic hierarchy but restrict the top level 'Cha'.
    // Actually, parse_expr_atom handles Punctuation, Literals, Identifiers, Subqueries, Parens.
    // It does NOT handle Unary/Binary ops relative to other atoms (except what's inside Parens).
    // Does SSQL support `Select -a, b`? Unary minus?
    // parse_expr_atom does not include Unary.
    // We might want to support Unary.
    // But for now, let's just use parse_expr_atom(expr, dql_parser) as the item parser.
    // dql_parser passed here is the RECURSION HANDLE from parse_dql.

    parse_expr_atom(expr_parser, subquery_parser)
}

/// Parse a WHERE clause: (condition) इति नियमेन

pub(crate) fn parse_where_clause<'a>(
    _expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], WhereClause, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    parse_logical_expression(subquery_parser)
        // `चेत्` ("if") conditions a whole statement (`न विद्यते चेत्`, IF NOT
        // EXISTS); a row filter is `इति नियमेन` like the other clause markers
        // (expert review Q3).
        .then_ignore(just(Token::Iti).then(just(Token::Niyamena)))
        .map(|condition| {
            WhereClause {
            condition: Box::new(condition),
        }})
}

/// Parse GROUP BY clause: (expr list) इति समूहेन OR (expr list इति समूहेन)

pub(crate) fn parse_group_by_clause<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], GroupByClause, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let raw_items = conj_list(parse_expression_item(expr_parser, subquery_parser));
    let items = raw_items;

    let clause = items
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Samuhena))
        .map(|expressions| GroupByClause { expressions });
        
    clause
}

/// Parse ORDER BY clause: (expr list) इति क्रमेण OR (expr list इति क्रमेण)

pub(crate) fn parse_order_by_clause<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], OrderByClause, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let order_item = parse_expression_item(expr_parser, subquery_parser)
        // Every key names its direction, as a window's does: `क्रमेण` alone
        // says "in order", not which (final review).
        .then(
            just(Token::Avarohah).to(OrderDirection::Desc)
                .or(just(Token::Arohah).to(OrderDirection::Asc))
                .or_not()
        )
        .validate(|(expr, dir), e, emitter| {
            if dir.is_none() {
                emitter.emit(Rich::custom(e.span(), "an ordering key names its direction: `… आरोहः` (ascending) or `… अवरोहः` (descending), before `इति क्रमेण`"));
            }
            OrderExpr {
                expr: Box::new(expr),
                direction: dir.unwrap_or(OrderDirection::Asc),
            }
        });

    let raw_items = conj_list(order_item);
    let items = raw_items;

    let clause = items
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Kramena))
        .map(|items| OrderByClause { items });

    clause
}

/// Parse LIMIT clause: atom इति मित्या
/// Parse LIMIT clause: atom इति मित्या OR (atom) इति मित्या

pub(crate) fn parse_limit_clause<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], LimitClause, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    parse_expression_item(expr_parser.clone(), subquery_parser.clone())
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Mitya))
        .then(
             parse_expression_item(expr_parser.clone(), subquery_parser.clone())
             .then_ignore(just(Token::Apasarena))
             .or_not()
        )
        .map(|(count, offset)| LimitClause {
             count: Box::new(count),
             offset: offset.map(Box::new),
        })
}

/// Parse source clause (FROM): panchamI table

pub(crate) fn parse_source_clause<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], SourceClause, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // A derived table: `यत् <query> इति आश्रित्य` ("having taken the result
    // of <query>"). Gated on यत् so ordinary FROM sources never try it.
    let derived_source = just(Token::Yat)
        .rewind()
        .ignore_then(expr_parser)
        .then_ignore(just(Token::Ashritya))
        .try_map(|expr, span: SimpleSpan| match expr {
            Expression::Subquery(query) => Ok(SourceClause {
                derived: Some(query),
                table: Identifier {
                    text: String::new(),
                    vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Panchami, vachana: crate::types::Vachana::Bahu, linga: None },
                    pratipadika: String::new(),
                    symbol_type: SymbolType::Table,
                    span: Some((span.start(), span.end())),
                },
            }),
            _ => Err(Rich::custom(span, "'आश्रित्य' (FROM a derived table) needs a query: यत् <query> इति आश्रित्य")),
        });

    let raw_source = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Panchami { text, vachana, pratipadika } | Token::Shashthi { text, vachana, pratipadika } => Ok(SourceClause {
            derived: None,
            table: Identifier {
                text: text.clone(),
                vibhakti_form: VibhaktiForm {
                    vibhakti: Vibhakti::Panchami,
                    vachana,
                    linga: None,
                },
                pratipadika,
                symbol_type: SymbolType::Table,
                span: Some((span.start(), span.end())),
            },
        }),
        _ => Err(Rich::custom(span, "expected ablative or genitive (FROM table)")),
    });

    choice((derived_source, raw_source))
}

/// Parse SELECT clause: SelectVerb [cols]

pub(crate) fn parse_select_clause<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], SelectClause, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let safe_select_item = parse_select_item(expr_parser.clone(), subquery_parser); 

    let select_list = conj_list(safe_select_item);

    // SELECT DISTINCT: `पुनरुक्तिम् विना`, "without repetition", a row-level
    // phrase before the select list (expert review N1).
    let distinct = just(Token::Punaruktim).then(just(Token::Vina)).or_not();
    
    distinct
        .then(select_list)
        .map(|(distinct_opt, columns)| {
            SelectClause { 
                columns,
                distinct: distinct_opt.is_some(),
            }
        })
}


pub(crate) fn parse_select_item<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], crate::ast::SelectItem, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let base_item = parse_expression_item(expr_parser.clone(), subquery_parser.clone())
        .then(
            choice((
                // `इति <alias>` puts the name in the category slot (`X इति Y` is
                // "X, a Y", as in `सङ्ख्या इति पूर्णाङ्कः`); the alias is
                // `<alias> इति आख्या` (expert review G1). This branch only
                // explains the error, at the alias's own position, so it wins
                // the furthest-error choice over the clause markers' failures.
                just(Token::Iti)
                    .ignore_then(any().filter(|t| matches!(t, Token::Prathama { .. } | Token::Identifier(_) | Token::StringLit(_))))
                    .try_map(|tok, span: SimpleSpan| Err::<String, _>(Rich::custom(
                        span,
                        format!("An alias is written `<alias> इति आख्या`: `{} इति आख्या`, not `इति {}`", tok, tok),
                    ))),
                // The alias: `<alias> इति आख्या`, a name or a quoted SQL name,
                // never a reserved builtin word.
                choice((
                    any().try_map(|tok, span: SimpleSpan| match tok {
                        Token::StringLit(s) => Ok(s),
                        _ => Err(Rich::custom(span, "expected a quoted string")),
                    }),
                    any().try_map(|tok, span: SimpleSpan| match tok {
                        Token::Prathama { text, .. } | Token::Identifier(text) => {
                            if crate::reserved_words::is_reserved_builtin(&text) {
                                Err(Rich::custom(
                                    span,
                                    format!(
                                        "'{}' is a reserved builtin function name and cannot be used as an alias",
                                        text
                                    ),
                                ))
                            } else {
                                Ok(text)
                            }
                        }
                        _ => Err(Rich::custom(span, "Expected Identifier")),
                    }),
                ))
                .then_ignore(just(Token::Iti))
                .then_ignore(just(Token::Aakhya))
            ))
            .or_not()
        )
        .try_map(|(expr, alias), span| {
            if let Some(a) = &alias {
                if crate::reserved_words::is_reserved_builtin(a) {
                    return Err(Rich::custom(
                        span,
                        format!(
                            "'{}' is a reserved builtin function name and cannot be used as an alias",
                            a
                        ),
                    ));
                }
            }
            Ok(crate::ast::SelectItem { expr, alias })
        });

    // A single optional outer wrap around the WHOLE "expr (इति alias)?"
    // item - e.g. "(SUM(x) इति alias)", where the alias must attach inside
    // the same parens as the expression it names. This is deliberately flat
    // (not recursive over `item`): `expr` itself already absorbs any
    // redundant *internal* nesting on its own (see prathama_based in
    // parse_math_expression), so this only needs to handle one more layer of
    // wrapping around the complete item.
    //
    // Bare base_item MUST be tried before the delimited_by(paren) wrap, not
    // after: whenever expr itself already starts with its own parens (any
    // aggregate/window/CASE - "(SUM(x)) इति alias"), trying delimited_by
    // first makes it succeed by matching the item's *own* parens as if they
    // were the redundant outer wrap - base_item inside that attempt then
    // stops right at expr's closing paren (nothing after it looks like an
    // alias), so the choice commits to "no alias" and never gets to see the
    // real "इति alias" that follows outside those parens.
    // Bare base_item alone can't be fooled this way: parse_expression_item's
    // own parenthesized-expr handling requires a closing paren immediately
    // after expr, so for a *whole-item* wrap like "(plain_col इति alias)"
    // (no parens of its own around plain_col) it fails outright with the
    // alias still unconsumed inside the wrap, and only then does the choice
    // correctly fall back to the delimited_by(paren) alternative.
    base_item.clone()
}
/// Parse JOIN clause: (table सह (condition) इति मेलनेन)

pub(crate) fn parse_join_clause<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], JoinClause, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let tritiya_table = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Tritiya { text, vachana, pratipadika } => Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Tritiya,
                vachana,
                linga: None,
            },
            pratipadika,
            symbol_type: SymbolType::Table,
            span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "expected instrumental (Join Table)")),
    });

    // A join is `<table> सह <condition> इति मेलनेन` (INNER), or
    // वाममेलनेन / दक्षिणमेलनेन / पूर्णमेलनेन for LEFT / RIGHT / FULL; the
    // condition is a quotation closed by इति and named by its marker. A
    // CROSS JOIN has no condition to quote: `<table> सह सर्वमेलनेन`.
    let conditioned = tritiya_table.clone()
        .then_ignore(just(Token::Saha))
        .then(expr_parser)
        .then_ignore(just(Token::Iti))
        .then(choice((
            just(Token::Melanena).to(None),
            just(Token::VamaMelanena).to(Some(JoinType::Left)),
            just(Token::DakshinaMelanena).to(Some(JoinType::Right)),
            just(Token::PurnaMelanena).to(Some(JoinType::Full)),
        )))
        .map(|((table, condition), join_type)| JoinClause {
            join_type,
            table,
            condition: Some(Box::new(condition)),
        });
    let cross = tritiya_table
        .then_ignore(just(Token::Saha))
        .then_ignore(just(Token::SarvaMelanena))
        .map(|table| JoinClause {
            join_type: Some(JoinType::Cross),
            table,
            condition: None,
        });

    choice((cross, conditioned))
}

/// Parse HAVING clause: expression इति समूहनियमेन

pub(crate) fn parse_having_clause<'a>(
    _expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], HavingClause, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let clause = parse_logical_expression(subquery_parser)
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Samuhaniyamena))
        .map(|condition| HavingClause {
            condition: Box::new(condition)
        });

    clause
}

/// Parse SQL Data Types

pub fn parse_dql<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], DqlStatement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    recursive(|dql_recursive| {
        // Wrap DQL parser into Expression parser for subqueries. When embedded
        // as an expression atom (WHERE x = <subquery>, IN <subquery>, etc.)
        // a bare (unparenthesized) subquery is only unambiguous if it carries
        // its own explicit आह्रियताम् - otherwise there's nothing marking
        // where its select-list stops and the surrounding expression resumes.
        let subquery_parser = choice((
            dql_recursive.clone()
                .delimited_by(just(Token::Yat), just(Token::Iti)),
            dql_recursive.clone()
                .try_map(|d: DqlStatement, span| if d.verb_token.is_some() {
                    Ok(d)
                } else {
                    Err(Rich::custom(span, "Bare embedded subquery requires an explicit आह्रियताम्"))
                }),
        ))
            .map(|d| Expression::Subquery(Box::new(d)))
            .boxed();

        let body = parse_dql_body(expr_parser.clone(), subquery_parser);

        let fetch_verb_parser = any().filter(|tok| matches!(tok, Token::FetchVerb(_)))
            .map(|tok| match tok {
                Token::FetchVerb(s) => Identifier {
                    text: s.clone(),
                    vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
                    pratipadika: "आह्रियताम्".to_string(),
                    symbol_type: SymbolType::Variable,
                    span: None,
                },
                _ => unreachable!(),
            })
            .or_not();

        // Allow (Query) or Query with FetchVerb
        let with_fetch = body.clone()
            .then(fetch_verb_parser)
            .then_ignore(just(Token::Purnadanda).or_not())
            .map(|(mut dql, verb_opt)| {
                dql.verb_token = verb_opt;
                dql
            });
            
        with_fetch
    })
}

// Helper: Parses Core DQL Body (Source...Limit) returning DqlStatement

pub(crate) fn parse_dql_body<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], DqlStatement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let source = parse_source_clause(expr_parser.clone());
    let joins = parse_join_clause(expr_parser.clone()).repeated().collect::<Vec<_>>();
    
    let where_c = parse_where_clause(expr_parser.clone(), subquery_parser.clone()).or_not();
    let group_b = parse_group_by_clause(expr_parser.clone(), subquery_parser.clone()).or_not();
    let having_c = parse_having_clause(expr_parser.clone(), subquery_parser.clone()).or_not();
    
    // ORDER BY and LIMIT stand with the other clause markers, before the
    // select list, which is the verb's subject and sits next to it (expert
    // review H1).
    let order_b = parse_order_by_clause(expr_parser.clone(), subquery_parser.clone()).or_not();
    let limit = parse_limit_clause(expr_parser.clone(), subquery_parser.clone()).or_not();
    let select = parse_select_clause(expr_parser.clone(), subquery_parser.clone());

    source
        .then(joins)
        .then(where_c)
        .then(group_b)
        .then(having_c)
        .then(order_b)
        .then(limit)
        .then(select)
        .try_map(|(((((((source, joins), where_clause), group_by), having), order_by), limit), select), _span| {

            Ok(DqlStatement {
                source,
                joins,
                where_clause,
                group_by,
                having,
                select,
                order_by,
                limit,
                verb_token: None,
            })
        })
}

/// Parse Explain Statement (DQL Body + ExplainVerb)

pub(crate) fn parse_explain_dql<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], Statement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // We need subquery parser for the inner DQL body.
    // Subqueries inside Explain are Standard DQLs (using parse_dql).
    let dql_parser = parse_dql(expr_parser.clone()); 
    let subquery_parser = dql_parser.delimited_by(just(Token::Yat), just(Token::Iti))
        .map(|d| Expression::Subquery(Box::new(d)))
        .boxed();

    let body = parse_dql_body(expr_parser.clone(), subquery_parser);
    
    // Body + ExplainVerb
    // Also handle parens: ( Body ExplainVerb )
    let with_explain = body
        .then_ignore(just(Token::Explain))
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|d| Statement::Explain(Box::new(Statement::Dql(d))));
        
    with_explain
}
