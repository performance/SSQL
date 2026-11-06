//! DML statements (INSERT/UPDATE/DELETE), DCL, and TCL (transaction control)
//! statements.

use chumsky::prelude::*;
use crate::ast::{
    Statement, Expression, Identifier, DclStatement, DmlStatement,
    DqlStatement, WhereClause, BinaryOperator, LogicalOperator,
};
use crate::types::{Vibhakti, Vachana, VibhaktiForm, SymbolType};
use super::Token;
use super::common::{parse_literal, parse_identifier, parse_any_identifier, conj_list};
use super::clauses::parse_source_clause;

pub(crate) fn parse_dcl<'a>() -> impl Parser<'a, &'a [Token], DclStatement, extra::Err<Rich<'a, Token>>> + Clone {
    any().filter(|_| false).map(|_| unreachable!("DCL parsing not implemented"))
}

/// Parse TCL

pub(crate) fn parse_tcl_statement<'a>() -> impl Parser<'a, &'a [Token], crate::ast::TclStatement, extra::Err<Rich<'a, Token>>> + Clone {
    let begin = parse_any_identifier()
        .then_ignore(just(Token::BeginVerb))
        .map(|_| crate::ast::TclStatement::Begin);

    let commit = parse_any_identifier()
        .then_ignore(just(Token::CommitVerb))
        .map(|_| crate::ast::TclStatement::Commit);

    let savepoint = choice((
        // Quoted string like "sp1" इति
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::StringLit(s) => Ok(Identifier {
                text: s.clone(),
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
                pratipadika: s,
                symbol_type: SymbolType::Savepoint,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected a quoted savepoint name")),
        }).then_ignore(just(Token::Iti)),
        // Native identifier like प्रथमः
        parse_any_identifier().map(|mut ident| {
            ident.symbol_type = SymbolType::Savepoint;
            ident
        }),
    ))
    .then_ignore(just(Token::Sangrahabinduh))
    .then_ignore(just(Token::SavepointVerb))
    .map(|name| crate::ast::TclStatement::Savepoint { name });

    let rollback_to_savepoint = choice((
        // Quoted string like "sp1" इति
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::StringLit(s) => Ok(Identifier {
                text: s.clone(),
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
                pratipadika: s,
                symbol_type: SymbolType::Savepoint,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected a quoted savepoint name")),
        }).then_ignore(just(Token::Iti)),
        // Native identifier like प्रथमं
        parse_any_identifier().map(|mut ident| {
            ident.symbol_type = SymbolType::Savepoint;
            ident
        }),
    ))
    // प्रति takes the accusative: "back towards the savepoint" (expert
    // review O2).
    .then_ignore(just(Token::SangrahabindumDvitiya))
    .then_ignore(just(Token::Prati))
    .then_ignore(parse_any_identifier()) // "व्यवहारः" / "व्यवहार"
    .then_ignore(just(Token::RollbackVerb))
    .map(|name| crate::ast::TclStatement::Rollback { savepoint: Some(name) });

    let rollback = parse_any_identifier() // "व्यवहारः" / "व्यवहार"
        .then_ignore(just(Token::RollbackVerb))
        .map(|_| crate::ast::TclStatement::Rollback { savepoint: None });

    choice((
        begin,
        commit,
        rollback_to_savepoint,
        rollback,
        savepoint,
    ))
}





/// An UPDATE / DELETE filter: `<condition> इति नियमेन`, with the full
/// statement-level expression grammar, so it may hold subqueries.
fn dml_filter<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], WhereClause, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    expr_parser
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Niyamena))
        .map(|condition| WhereClause { condition: Box::new(condition) })
}

pub(crate) fn parse_delete_statement<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], Statement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // What is removed is either the rows a condition selects
    // (`<table> <condition> इति नियमेन निष्कास्यताम्`) or, said
    // explicitly, everything (`<table> सर्वम् निष्कास्यताम्`, "let everything
    // be removed from <table>"). A DELETE with neither is not a sentence.
    let filter = dml_filter(expr_parser.clone())
        .map(Some)
        .or(just(Token::Sarvam).to(None));
    let inner = parse_source_clause(expr_parser.clone())
        .then(filter)
        .then_ignore(just(Token::DeleteVerb))
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|(source, where_clause)| Statement::Dml(DmlStatement::Delete {
             table: source.table,
             where_clause
        }));
        
    inner
}

/// Parse UPDATE Target Table (Saptami)

pub(crate) fn parse_update_target<'a>() -> impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone {
    any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Saptami { text, vachana, pratipadika } => Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Saptami,
                vachana,
                linga: None,
            },
            pratipadika,
            symbol_type: SymbolType::Table,
            span: Some((span.start(), span.end())),
        }),
        // Fallback for when Saptami is not detected (e.g. Panchami used erroneously but we want to be lenient? No, stick to Saptami for now)
        _ => Err(Rich::custom(span, "expected locative (UPDATE table)")),
    })
}

/// A column that is inserted or changed is the verb's subject, so it is in
/// the nominative (`नाम "गीता"`, not the locative `नाम्नि "गीता"`).
fn nominative_column<'a>() -> impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone {
    parse_identifier().try_map(|id, span| {
        if id.vibhakti_form.vibhakti == Vibhakti::Prathama {
            Ok(id)
        } else {
            Err(Rich::custom(span, format!("`{}`: an inserted or changed column is named in the nominative", id.text)))
        }
    })
}

/// Parse SET Assignment: `<col> <value> इति` - इति marks the value as the
/// column's new content ("let the price be changed to 60", as in `रामः राजा
/// इति अभिषिच्यताम्`); without it the bare pair reads as "the price that is
/// 60" (expert review F3).

pub(crate) fn parse_set_assignment<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a {
     // A column is emptied by the adjective that agrees with it
     // (`भाषा रिक्ता इति`, as in the IS NULL test `भाषा रिक्ता अस्ति`); the
     // annotator checks the agreement.
     let empty = any()
        .filter(|t| matches!(t, Token::Riktah(_)))
        .to(Expression::Literal(crate::ast::Literal::Null));
     nominative_column()
        .then(choice((empty, parse_literal(), expr_parser.boxed()))) // Value can be literal or expr
        .then_ignore(just(Token::Iti))
        .map(|(col, val)| Expression::BinaryOp {
            left: Box::new(Expression::Identifier(col)),
            op: BinaryOperator::Equal,
            right: Box::new(val),
        })
}

/// Parse SET Clause: `<col> <value> इति`, or for several columns a च-list
/// with each item's own च: `मूल्यम् ६० इति च भाषा "पालि" इति च`.

pub(crate) fn parse_set_clause<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], crate::ast::SetClause, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    parse_set_assignment(expr_parser)
        .then(just(Token::Cha).or_not())
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .try_map(|items, span| {
            let several = items.len() > 1;
            if items.iter().any(|(_, cha)| cha.is_some() != several) {
                return Err(Rich::custom(span, "Several changed columns form a च-list, each with its own च (`मूल्यम् ६० इति च भाषा \"पालि\" इति च`); a single one takes none"));
            }
            Ok(items.into_iter().map(|(a, _)| a).collect::<Vec<_>>())
        })
        .map(|assignments| {
            let mut iter = assignments.into_iter();
            let first = iter.next().unwrap();
            let folded = iter.fold(first, |acc, next| Expression::LogicalOp {
                left: Box::new(acc),
                op: LogicalOperator::And,
                right: Box::new(next),
            });
            crate::ast::SetClause { assignments: Box::new(folded) }
        })
}

fn insert_verb<'a>() -> impl Parser<'a, &'a [Token], String, extra::Err<Rich<'a, Token>>> + Clone {
    any().try_map(|tok, span| match tok {
        Token::InsertVerb(text) => Ok(text),
        _ => Err(Rich::custom(span, "expected निवेश्यताम् / निवेश्येताम् / निवेश्यन्ताम्")),
    })
}

fn update_verb<'a>() -> impl Parser<'a, &'a [Token], String, extra::Err<Rich<'a, Token>>> + Clone {
    any().try_map(|tok, span| match tok {
        Token::UpdateVerb(text) => Ok(text),
        _ => Err(Rich::custom(span, "expected परिवर्त्यताम् / परिवर्त्येताम् / परिवर्त्यन्ताम्")),
    })
}

/// Parse UPDATE Statement
/// (Table[Saptami] Where? Set UpdateVerb)

pub(crate) fn parse_update_statement<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], Statement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // What is changed is either the rows a condition selects (`<table>
    // <condition> इति नियमेन …`) or, said explicitly, every row (`<table>
    // सर्वत्र …`, "everywhere in <table>"; expert review Q5) - as DELETE
    // needs a filter or सर्वम्, so a forgotten filter never changes a table.
    let filter = dml_filter(expr_parser.clone())
        .map(Some)
        .or(just(Token::Sarvatra).to(None));
    let inner = parse_update_target()
        .then(filter)
        .then(parse_set_clause(expr_parser.clone()))
        .then(update_verb())
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|(((table, where_clause), set_clause), verb)| {
             Statement::Dml(DmlStatement::Update {
                 table,
                 where_clause,
                 set_clause,
                 verb,
             })
        });

    // Also support order: Table Set Where Verb? Flexible?
    // 22 uses: Table Where Set Verb.
    // Let's stick to that order for now.
    
    inner
}

/// Parse INSERT Statement
/// (Table[Saptami] (Col Val Cha Col Val ...) InsertVerb)

pub(crate) fn parse_insert_statement<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    dql_parser: impl Parser<'a, &'a [Token], DqlStatement, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], Statement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // Insert assignments: (Col Val Cha Col Val ...)
    // Parse as BinaryOp(Eq) pairs: Equal(Col, Val)
    let col_val_pair_std = nominative_column()
        .then(parse_literal().or(expr_parser.clone()))
        .map(|(col, val)| Expression::BinaryOp {
            left: Box::new(Expression::Identifier(col)),
            op: BinaryOperator::Equal,
            right: Box::new(val),
        });

    // INSERT gives each value as `<col> <value>` ("let title X be
    // inserted"; expert review F3): one form, a च-list for several.
    let col_val_pair = col_val_pair_std;
    
    // Parse list separated by Cha
    let values_list = conj_list(col_val_pair);

    let insert_values = parse_update_target()
        .then(values_list)
        .then(insert_verb())
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|((table, exprs), verb)| {
            let mut iter = exprs.into_iter();
            let first = iter.next().unwrap_or(Expression::Wildcard); // Fallback
            let folded = iter.fold(first, |acc, expr| Expression::LogicalOp {
                left: Box::new(acc),
                op: LogicalOperator::And,
                right: Box::new(expr),
            });
            Statement::Dml(DmlStatement::Insert {
                 table,
                 values: Box::new(folded),
                 verb,
            })
        });

    // INSERT … SELECT: `<table> [<col> च <col> च …] यत् <query> इति <verb>`.
    // The query is an embedded statement, grouped with `यत् … इति` (expert
    // review Q7); the verb agrees with the number of columns inserted.
    let columns_list = conj_list(parse_identifier());
    let grouped_query = dql_parser.clone().delimited_by(just(Token::Yat), just(Token::Iti));

    let insert_select = parse_update_target()
        .then(columns_list.or_not())
        .then(grouped_query)
        .then(insert_verb())
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|(((table, columns), dql), verb)| Statement::Dml(DmlStatement::InsertSelect {
             table,
             columns,
             query: Box::new(dql),
             verb,
        }));

    choice((insert_select, insert_values))
}
