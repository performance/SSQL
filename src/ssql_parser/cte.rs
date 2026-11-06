//! Variable/CTE declarations (`नाम इति [सूत्रम्] expr।`), including recursive-CTE
//! flags and cycle-prevention columns, plus the external-parameter declaration.

use chumsky::prelude::*;
use crate::ast::{DqlStatement, Expression, Identifier, ParameterDecl};
use crate::types::{Vibhakti, Vachana, VibhaktiForm, SymbolType};
use super::Token;
use super::common::parse_identifier_prathama;
use super::ddl::parse_stem_gender_vachana;

pub(crate) fn parse_parameter_decl<'a>() -> impl Parser<'a, &'a [Token], ParameterDecl, extra::Err<Rich<'a, Token>>> + Clone {
    parse_identifier_prathama()
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Bahya))
        .then_ignore(just(Token::Sanjna))
        .then_ignore(just(Token::Purnadanda).or_not()) // Allow trailing Purnadanda
        .map(|name| ParameterDecl { name })
}


pub(crate) fn parse_cycle_column<'a>() -> impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Shashthi { text, pratipadika, .. } => {
            Ok(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
                pratipadika,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            })
        },
        Token::Prathama { text, pratipadika, .. } => {
            Ok(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
                pratipadika,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            })
        },
        Token::Identifier(text) => {
            let clean_text = if text.ends_with("स्य") {
                text.trim_end_matches("स्य").to_string()
            } else if text.ends_with("याः") {
                text.trim_end_matches("याः").to_string()
            } else if let Some(base) = text.strip_suffix("्नः").filter(|b| b.ends_with("नाम")) {
                // The genitive of a नामन् compound (अतिथिनाम्नः → अतिथिनाम).
                base.to_string()
            } else {
                text.clone()
            };
            Ok(Identifier {
                text: clean_text.clone(),
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
                pratipadika: clean_text,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            })
        },
        _ => Err(Rich::custom(span, "expected column identifier".to_string())),
    })
}

/// Parse Variable Declaration
/// Format: Name ITI [Aavartika Sanjna] Expression [Cycle Prevention] Purnadanda

pub(crate) fn parse_var_decl<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    dql_parser: impl Parser<'a, &'a [Token], DqlStatement, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], (Identifier, Expression, bool, Option<Identifier>), extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let rec_flag = choice((
        just(Token::Aavartika)
            .then_ignore(just(Token::Sanjna))
            .to(true),
        just(Token::Sanjna).to(false),
    ))
    .or_not()
    .map(|o| o.unwrap_or(false));

    let chakra_nivarana = just(Token::ChakraNivaranena).repeated().at_least(1).ignored();

    let cycle_prevention = parse_cycle_column()
        .then_ignore(chakra_nivarana)
        .or_not();

    let formula_marker = just(Token::Sutram).or_not();

    // Explicit stem clause: इति आख्या <stem> इति <gender> <vachana>. Same
    // family as the table/column/view clause (parse_stem_gender_vachana),
    // just without a quoted SQL-name string - a CTE's generated SQL name is
    // its Sanskrit name as-is (or an explicit override via a schema-level
    // pseudo-table declaration, which already goes through CreateStatement).
    let stem_clause = just(Token::Aakhya)
        .ignore_then(parse_stem_gender_vachana(parse_identifier_prathama()));

    // A CTE body is either a bare DQL statement (only unambiguous with its
    // own explicit आह्रियताम् - same reasoning as clauses.rs::parse_dql's
    // subquery_parser) or a general expression (e.g. a scalar सूत्रम्
    // formula). Tried as a cheap, direct alternative here rather than
    // relying on subquery_parser's यत्-gated slots deep inside
    // parse_expr_atom, which a bare (unparenthesized) DQL body never
    // reaches - expr_parser is kept as the fallback for everything else.
    let bare_set_op_value = super::set_ops::parse_set_operation(expr_parser.clone())
        .map(|s| match s {
            crate::ast::Statement::SetOp(op) => Expression::SetOp(Box::new(op)),
            _ => unreachable!(),
        });
    let bare_dql_value = dql_parser
        .try_map(|d: DqlStatement, span| if d.verb_token.is_some() {
            Ok(Expression::Subquery(Box::new(d)))
        } else {
            Err(Rich::custom(span, "Bare CTE body requires an explicit आह्रियताम्"))
        });
    let yat_dql_value = bare_dql_value
        .clone()
        .delimited_by(just(Token::Yat), just(Token::Iti));
    let value_parser = choice((bare_set_op_value, yat_dql_value, bare_dql_value, expr_parser));

    parse_identifier_prathama()
        .then_ignore(just(Token::Iti))
        .then(stem_clause)
        .then_ignore(formula_marker)
        .then(rec_flag)
        .then(value_parser)
        .then(cycle_prevention)
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|((((declared_name, (stem, gender, vachana)), is_recursive), value), cycle_col)| {
            let name = Identifier {
                text: declared_name.text,
                vibhakti_form: VibhaktiForm {
                    vibhakti: Vibhakti::Prathama,
                    vachana,
                    linga: Some(gender),
                },
                pratipadika: stem.text,
                symbol_type: declared_name.symbol_type,
                span: declared_name.span,
            };
            (name, value, is_recursive, cycle_col)
        })
}

