//! Set operations: UNION/UNION ALL/INTERSECT/EXCEPT (सङ्गमः/सर्वसङ्गमः/सम्मिलनम्/वर्जनम्).

use chumsky::prelude::*;
use crate::ast::{Statement, Expression, DqlStatement};
use super::Token;
use super::clauses::parse_dql;
use super::common::parse_atom;

pub(crate) fn parse_set_operation<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], Statement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let separator = just(Token::Cha).ignored();

    let op_parser = choice((
        just(Token::Sangamah).to((crate::ast::SetOperation::Union, false)),
        just(Token::SarvaSangamah).to((crate::ast::SetOperation::Union, true)),
        just(Token::Sammilanam).to((crate::ast::SetOperation::Intersect, false)),
        just(Token::Varjanam).to((crate::ast::SetOperation::Except, false)),
    ));

    let connector = any().filter(|tok| matches!(tok, Token::Ityayoh(_) | Token::Ityesham(_)));

    recursive(|set_op_handle| {
        // A branch that is itself a set operation is grouped with `यत् … इति`
        // (expert review Q6c): `यत् A च B च इत्यनयोः सङ्गमः इति च C च
        // इत्यनयोः प्रतिच्छेदः`.
        let set_op_expr = set_op_handle
            .delimited_by(just(Token::Yat), just(Token::Iti))
            .map(|s| {
                if let Statement::SetOp(op) = s {
                    Expression::SetOp(Box::new(op))
                } else {
                    unreachable!()
                }
            });

        // Bare subquery branch only when it carries its own explicit
        // आह्रियताम् - see the identical reasoning in clauses.rs::parse_dql.
        let bare_subquery = parse_dql(expr_parser.clone())
            .try_map(|d: DqlStatement, span| if d.verb_token.is_some() {
                Ok(d)
            } else {
                Err(Rich::custom(span, "Bare set-op branch requires an explicit आह्रियताम्"))
            });

        let item_parser = choice((
            set_op_expr,
            bare_subquery.map(|d| Expression::Subquery(Box::new(d))),
            parse_atom(),
        ));

        // The branch list itself is terminated by its own distinctive
        // connector (इत्यनयोः/इत्येषाम्), so the outer paren wrap around the
        // whole list is redundant - item_parser never starts with Cha or
        // the connector tokens, so separated_by naturally stops there. Kept
        // as an accepted (not required) alternative for existing callers
        // that still wrap the whole list in one more pair of parens.
        // In standard Sanskrit grammar (Pāṇini 2.2.29), conjunctions using च (Cha)
        // require every element in a list to be followed by च (A च B च इत्यनयोः).
        // Require strict trailing Cha after every item in the set operation.
        let raw_exprs = item_parser.clone()
            .then_ignore(separator.clone())
            .then(item_parser.clone().then_ignore(separator.clone()).repeated().collect::<Vec<_>>())
            .map(|(first, rest)| {
                let mut items = vec![first];
                items.extend(rest);
                items
            });

        let exprs = raw_exprs;

        exprs
            .then(connector)
            .then(op_parser)
            .then_ignore(any().filter(|tok| matches!(tok, Token::FetchVerb(_))).or_not())
            .then_ignore(just(Token::Purnadanda).or_not())
            // Reported without failing the parse, which another alternative's
            // "expected ablative" used to mask (final review).
            .validate(|((queries, conn_tok), (op, use_all)), e, emitter| {
                match conn_tok {
                    Token::Ityayoh(_) if queries.len() != 2 => emitter.emit(Rich::custom(e.span(), format!(
                        "the dual 'इत्यनयोः' joins exactly two branches; these are {}, so write 'इत्येषाम्'", queries.len()
                    ))),
                    Token::Ityesham(_) if queries.len() < 3 => emitter.emit(Rich::custom(e.span(), format!(
                        "the plural 'इत्येषाम्' joins three or more branches; these are {}, so write 'इत्यनयोः'", queries.len()
                    ))),
                    _ => {}
                }
                Statement::SetOp(crate::ast::SetOpStatement { queries, op, use_all })
            })
    })
}
