//! Shared low-level parsers used across the expression, DQL-clause, and DML
//! modules: literals, plain/field-access identifiers, and the shared
//! `DataType`/genitive-marker atoms.

use chumsky::prelude::*;
use crate::ast::{Expression, Identifier, Literal, DataType, BinaryOperator};
use crate::types::{Vibhakti, Vachana, VibhaktiForm, SymbolType, Padam};
use super::Token;

pub(crate) fn parse_literal<'a>() -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone {
    any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Number(n) => Ok(Expression::Literal(Literal::Number(n))),
        Token::StringLit(s) => Ok(Expression::Literal(Literal::String(s))),
        _ => Err(Rich::custom(span, "expected literal (number or string)")),
    })
}

/// Parse an identifier reference (column, table, etc.)

pub(crate) fn parse_identifier_expr<'a>() -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone {
    // A bare Shashthi token reaching this point (as the `parse_atom()`
    // fallback, not consumed by `parse_field_access()` or any of
    // `shashthi_expr`'s suffix-marker alternatives in `expr.rs`) is a
    // standalone column reference in genitive surface form (e.g., a window
    // ORDER BY operand like "दिनाङ्कस्य अनुसारम्"), mirroring `shashthi_expr`
    // alt2 with `SymbolType::Column`.
    let shashthi_restricted = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Shashthi { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Shashthi,
                vachana,
                linga: None,
            },
            pratipadika,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        })),
        _ => Err(Rich::custom(span, "expected genitive")),
    })
    .then_ignore(
        any().filter(|tok| match tok {
            Token::Prathama { .. } => true,
            Token::Identifier(_) => true,
            Token::Number(_) => true,
            Token::StringLit(_) => true,
            _ => false,
        }).not().rewind()
    );

    let other_idents = any().try_map(|tok, span: SimpleSpan| {
        match tok {
        Token::Prathama { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Prathama,
                vachana,
                linga: None,
            },
            pratipadika,
            symbol_type: SymbolType::Column, // Default assumption in expressions
            span: Some((span.start(), span.end())),
        })),
        Token::Identifier(text) => Ok(Expression::Identifier(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Prathama,
                vachana: Vachana::Eka,
                linga: None,
            },
            pratipadika: text,
            symbol_type: SymbolType::Column, // Raw identifier in expr -> Column
            span: Some((span.start(), span.end())),
        })),
        Token::Placeholder => Ok(Expression::Identifier(Identifier {
            text: "?".to_string(),
            vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
            pratipadika: "?".to_string(),
            symbol_type: SymbolType::Variable, // ? is a parameter/variable
            span: Some((span.start(), span.end())),
        })),
        Token::Varsham => Ok(Expression::Identifier(Identifier {
            text: "वर्षम्".to_string(),
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
            pratipadika: "वर्ष".to_string(),
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        })),
        Token::Masah => Ok(Expression::Identifier(Identifier {
            text: "मासः".to_string(),
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
            pratipadika: "मास".to_string(),
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        })),
        Token::Dinam => Ok(Expression::Identifier(Identifier {
            text: "दिनम्".to_string(),
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
            pratipadika: "दिन".to_string(),
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        })),
        Token::Dinankah => Ok(Expression::Identifier(Identifier {
            text: "दिनाङ्कः".to_string(),
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
            pratipadika: "दिनाङ्क".to_string(),
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        })),
        Token::Hora => Ok(Expression::Identifier(Identifier {
            text: "होरा".to_string(),
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
            pratipadika: "होरा".to_string(),
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        })),
        Token::Nimesah => Ok(Expression::Identifier(Identifier {
            text: "निमेषः".to_string(),
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
            pratipadika: "निमेष".to_string(),
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        })),
        Token::Vipalam => Ok(Expression::Identifier(Identifier {
            text: "क्षणम्".to_string(),
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
            pratipadika: "क्षण".to_string(),
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        })),
        _ => Err(Rich::custom(span, "expected identifier")),
        }
    });

    choice((shashthi_restricted, other_idents))
}

/// Parse a comparison operator

pub(crate) fn parse_comparison_op<'a>() -> impl Parser<'a, &'a [Token], (BinaryOperator, Padam), extra::Err<Rich<'a, Token>>> + Clone {
    any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Adhikam(s) => Ok((BinaryOperator::Greater, s)),
        Token::Nyunam(s) => Ok((BinaryOperator::Less, s)),
        Token::Samam(s) => Ok((BinaryOperator::Equal, s)),
        Token::Sadrusham(s) => Ok((BinaryOperator::Like, s)),
        Token::Anurupena => Ok((BinaryOperator::Regexp, "अनुरूपेण".to_string())),
        _ => Err(Rich::custom(span, "expected comparison operator")),
    })
}



/// Parse a simple expression (atom) - no nested expressions

pub(crate) fn parse_atom<'a>() -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone {
    choice((
        just(Token::Samprati).to(Expression::FunctionCall { name: "अद्यतनक्षणः".to_string(), args: vec![], window: None, pronoun_token: None }),
        just(Token::AdyatanaDinankah).to(Expression::FunctionCall { name: "अद्यतनदिनाङ्कः".to_string(), args: vec![], window: None, pronoun_token: None }),
        just(Token::AdyatanaSamayah).to(Expression::FunctionCall { name: "अद्यतनसमयः".to_string(), args: vec![], window: None, pronoun_token: None }),
        parse_literal(),
        parse_field_access(), // Try field access (Shashthi...) first
        parse_identifier_expr(), // Then simple identifier
    ))
    .or(any().try_map(|_, span: SimpleSpan| Err(Rich::custom(span, "expected a value, a column or a name"))))
}



/// A च-list: one item, or several items each followed by its own च, the last
/// one included (`शीर्षकम् च मूल्यम् च`; expert review L1). च is enclitic, so
/// `A च B`, with the last item bare, is not a list form.
pub(crate) fn conj_list<'a, O: 'a, P>(item: P) -> impl Parser<'a, &'a [Token], Vec<O>, extra::Err<Rich<'a, Token>>> + Clone + 'a
where
    P: Parser<'a, &'a [Token], O, extra::Err<Rich<'a, Token>>> + Clone + 'a,
{
    item.separated_by(just(Token::Cha))
        .at_least(1)
        .collect::<Vec<O>>()
        .then(just(Token::Cha).or_not())
        .map_with(|(items, trailing), e| {
            let slice: &[Token] = e.slice();
            (items, trailing.is_some(), slice.last().map(|t| t.to_string()).unwrap_or_default())
        })
        // `validate` reports the missing च without failing the parse, so no
        // other alternative backtracks over the list and masks the error.
        .validate(|(items, trailing, last), e, emitter| {
            if items.len() > 1 && !trailing {
                emitter.emit(Rich::custom(e.span(), last_conjunct_error("च", &last)));
            }
            // A pure function's positional arguments share the list
            // (`शीर्षकम् च मूल्यम् च ५ च प्रदाय करसहितमूल्यम्`): the parser reads
            // one call, which the compiler splits by the function's arity
            // (`normalize_udf_select_columns`), so a lone-looking item there
            // may be several.
            let slice: &[Token] = e.slice();
            if items.len() == 1 && trailing && !slice.iter().any(|t| matches!(t, Token::Pradaya)) {
                emitter.emit(Rich::custom(e.span(), lone_conjunct_error("च")));
            }
            items
        })
}

/// The error for a च/वा list whose last item has no particle of its own.
pub(crate) fn last_conjunct_error(particle: &str, last: &str) -> String {
    format!("every item of a {0}-list takes its own {0}, the last one too: add `{0}` after `{1}`", particle, last)
}

/// The error for a single item that carries a च/वा: a particle joins items,
/// so one item takes none.
pub(crate) fn lone_conjunct_error(particle: &str) -> String {
    format!("a single item takes no {0}: remove the `{0}`, which joins two or more items", particle)
}

/// The table that owns a column: `पुस्तकस्य` in `पुस्तकस्य शीर्षकम्`.
pub(crate) fn table_qualifier<'a>() -> impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone {
    any().try_map(|tok, span: SimpleSpan| match tok {
        // A column's table is named in the genitive singular, "the book's
        // title" taken row by row (`पुस्तकस्य शीर्षकम्`); the plural reads as
        // one collective title (expert review I1).
        Token::Shashthi { text, vachana: crate::types::Vachana::Bahu, .. } => Err(Rich::custom(
            span,
            format!("`{}`: a column's table is named in the genitive singular (`पुस्तकस्य शीर्षकम्`, not `पुस्तकानाम् शीर्षकम्`)", text),
        )),
        Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Shashthi,
                vachana,
                linga: None,
            },
            pratipadika,
            symbol_type: SymbolType::Table, // Usually a Table or Object
            span: Some((span.start(), span.end())),
        }),
        // A singular ablative is the same word as the genitive for every
        // stem except अ-stems (भोक्तुः, भाषायाः, अग्नेः, भानोः, नद्याः), so such a
        // token may qualify a column. A plural ablative (-भ्यः) or an अ-stem
        // singular ablative (-आत्) is never a genitive: accepting it let
        // `आपणेभ्यः नाम` ("from-the-stores name") pass as a field access, which
        // swallowed `यत् <query> इति` subqueries containing their own इति.
        Token::Panchami { text, vachana: vachana @ crate::types::Vachana::Eka, pratipadika } if !text.ends_with("ात्") => Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Shashthi, // Treat as Shashthi for access
                vachana,
                linga: None,
            },
            pratipadika,
            symbol_type: SymbolType::Table,
            span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "expected genitive (Object)")),
    })
}

pub(crate) fn parse_field_access<'a>() -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone {
    table_qualifier()
        .then(parse_identifier())
        .map(|(object, field)| {
            Expression::FieldAccess {
                object: Box::new(Expression::Identifier(object)),
                field
            }
        })
}


pub(crate) fn strip_distinct_prefix(text: &str) -> Option<(String, bool)> {
    if text.starts_with("विशिष्ट") && text.len() > "विशिष्ट".len() {
        let stripped = &text["विशिष्ट".len()..];
        return Some((stripped.to_string(), true));
    }
    None
}

pub(crate) fn parse_type<'a>() -> impl Parser<'a, &'a [Token], DataType, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    choice((
        just(Token::Purnankah).to(DataType::Integer),
        just(Token::Vakyam).to(DataType::Text),
        just(Token::Vastavikah).to(DataType::Decimal),
        just(Token::Dinankah).to(DataType::Date),
        just(Token::Dvaitam).to(DataType::Boolean),
    ))
}

/// Parse a प्रथमा (nominative) identifier token into an `Identifier`.
pub(crate) fn parse_identifier<'a>() -> impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone {
    any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Prathama { text, vachana, pratipadika } => Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Prathama,
                vachana,
                linga: None,
            },
            pratipadika,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        }),
        // The field keeps its own case (a locative in INSERT/UPDATE, a
        // genitive under आधारेण/अनुसारम्) so the rules that govern it can
        // check it; resolution goes by the stem.
        Token::Saptami { text, vachana, pratipadika } => Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Saptami,
                vachana,
                linga: None,
            },
            pratipadika,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        }),
        Token::Identifier(text) => Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Prathama,
                vachana: Vachana::Eka,
                linga: None,
            },
            pratipadika: text,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        }),
         Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
             text: text.clone(),
             vibhakti_form: VibhaktiForm {
                 vibhakti: Vibhakti::Shashthi,
                 vachana,
                 linga: None,
             },
             pratipadika,
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
        Token::Shreni => Ok(Identifier {
             text: "श्रेणी".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "श्रेणी".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
        Token::Varsham => Ok(Identifier {
             text: "वर्षम्".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "वर्षम्".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
         Token::Masah => Ok(Identifier {
             text: "मासः".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "मास".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
         Token::Dinankah => Ok(Identifier {
             text: "दिनाङ्कः".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "दिनाङ्क".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
        Token::Mahavarnarupam => Ok(Identifier {
             text: "महावर्णरूपम्".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "महावर्णरूपम्".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
        Token::Laghuvarnarupam => Ok(Identifier {
             text: "लघुवर्णरूपम्".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "लघुवर्णरूपम्".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
        Token::Khandah => Ok(Identifier {
             text: "खण्डः".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "खण्डः".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
        Token::Samyojanam => Ok(Identifier {
             text: "संयोजनम्".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "संयोजनम्".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
        Token::Aayamah => Ok(Identifier {
             text: "दैर्घ्यम्".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "दैर्घ्यम्".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
        Token::Vikalpah => Ok(Identifier {
             text: "विकल्पः".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "विकल्पः".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
        Token::Avashistam => Ok(Identifier {
             text: "अवशिष्टम्".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "अवशिष्टम्".to_string(),
             symbol_type: SymbolType::Column,
             span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "Expected Identifier")),
    })
}

/// Parse a select item (non-logical top level, but allows logical in parens)

pub(crate) fn parse_any_identifier<'a>() -> impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone {
    any().try_map(|tok, span: SimpleSpan| {
        let (text, pratipadika) = match &tok {
            Token::Prathama { text, pratipadika, .. } => (text.clone(), pratipadika.clone()),
            Token::Dvitiya { text, pratipadika, .. } => (text.clone(), pratipadika.clone()),
            Token::Tritiya { text, pratipadika, .. } => (text.clone(), pratipadika.clone()),
            Token::Chaturthi { text, pratipadika, .. } => (text.clone(), pratipadika.clone()),
            Token::Panchami { text, pratipadika, .. } => (text.clone(), pratipadika.clone()),
            Token::Shashthi { text, pratipadika, .. } => (text.clone(), pratipadika.clone()),
            Token::Saptami { text, pratipadika, .. } => (text.clone(), pratipadika.clone()),
            Token::Identifier(text) => (text.clone(), text.clone()),
            Token::StringLit(text) => (text.clone(), text.clone()),
            Token::Dinankah => ("दिनाङ्कः".to_string(), "दिनाङ्क".to_string()),
            _ => return Err(Rich::custom(span, "Expected Identifier")),
        };
        Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Prathama, // Treat generically
                vachana: Vachana::Eka,
                linga: None,
            },
            pratipadika,
            symbol_type: SymbolType::Variable,
            span: Some((span.start(), span.end())),
        })
    })
}


/// Parse a Prathama identifier (for subjects/parameters)

pub(crate) fn parse_identifier_prathama<'a>() -> impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone {
    any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Prathama { text, vachana, pratipadika } => Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Prathama,
                vachana,
                linga: None,
            },
            pratipadika,
            symbol_type: SymbolType::Variable,
            span: Some((span.start(), span.end())),
        }),
        Token::Identifier(text) => Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Prathama,
                vachana: Vachana::Eka,
                linga: None,
            },
            pratipadika: text,
            symbol_type: SymbolType::Variable,
            span: Some((span.start(), span.end())),
        }),
        Token::Shreni => Ok(Identifier {
             text: "श्रेणी".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "श्रेणी".to_string(),
             symbol_type: SymbolType::Variable,
             span: Some((span.start(), span.end())),
        }),
        Token::Varsham => Ok(Identifier {
             text: "वर्षम्".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "वर्षम्".to_string(),
             symbol_type: SymbolType::Variable,
             span: Some((span.start(), span.end())),
        }),
         Token::Masah => Ok(Identifier {
             text: "मासः".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "मास".to_string(),
             symbol_type: SymbolType::Variable,
             span: Some((span.start(), span.end())),
        }),
         Token::Dinankah => Ok(Identifier {
             text: "दिनाङ्कः".to_string(),
             vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
             pratipadika: "दिनाङ्क".to_string(),
             symbol_type: SymbolType::Variable,
             span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, format!("found `{}` where a name in the nominative was expected", tok))),
    })
}

// =============================================================================
// Statement Parsers
// =============================================================================

/// Parse a parameter declaration
/// Grammar: identifier ITI BAHYA SANJNA PURNADANDA

pub(crate) fn parse_genitive_marker<'a>() -> impl Parser<'a, &'a [Token], Token, extra::Err<Rich<'a, Token>>> + Clone {
    any().filter(|tok| match tok {
        Token::Ityasya(_) => true,
        Token::Ityasyah(_) => true,
        _ => false,
    })
}

