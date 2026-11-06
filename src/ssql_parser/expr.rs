//! The recursive expression grammar core: null-checks, comparisons, CASE
//! expressions, window functions/frames, scalar functions, arithmetic, and
//! the top-level atom/logical-expression combinators that tie the whole
//! recursive grammar together.

use chumsky::prelude::*;
use crate::ast::{
    Expression, Identifier, Literal, AggFunc, CaseArm,
    BinaryOperator, DataType,
};
use crate::types::{Vibhakti, Vachana, VibhaktiForm, SymbolType};
use super::Token;
use super::common::{
    parse_literal, parse_identifier_expr, parse_comparison_op, parse_atom,
    parse_field_access, strip_distinct_prefix, parse_type, table_qualifier,
};

/// What follows a null check's operand: `रिक्तम् अस्ति|नास्ति`. Factored out,
/// like `parse_comparison_continuation`, so an operand already parsed once can
/// look for it without being parsed again.
fn parse_null_check_continuation<'a>() -> impl Parser<'a, &'a [Token], (String, bool), extra::Err<Rich<'a, Token>>> + Clone + 'a {
    any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Riktah(s) => Ok(s),
            _ => Err(Rich::custom(span, "expected `रिक्तः`, `रिक्ता` or `रिक्तम्`")),
        })
        .then(choice((
            just(Token::Asti).to(false),
            just(Token::Nasti).to(true),
        )))
}

fn build_null_check_expr(left: Expression, rikta_str: String, negated: bool) -> Expression {
    Expression::SanskritComparison {
        left: Box::new(left),
        op: if negated { BinaryOperator::NotEqual } else { BinaryOperator::Equal },
        right: Box::new(Expression::Literal(Literal::Null)),
        pronoun: None,
        adjective: Some(Identifier {
            text: rikta_str.clone(),
            vibhakti_form: VibhaktiForm {
                vibhakti: Vibhakti::Prathama,
                vachana: Vachana::Eka,
                linga: None,
            },
            pratipadika: "रिक्त".to_string(),
            symbol_type: SymbolType::Column,
            span: None,
        }),
    }
}

/// A value and, if one follows, its null check or comparison. The value is
/// parsed once: as three alternatives (null check, comparison, plain value)
/// it was parsed three times at every nesting level, which made the parse
/// time of nested groups exponential.
fn with_optional_predicate<'a>(
    value: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    rhs_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    #[derive(Clone)]
    enum Predicate {
        Null(String, bool),
        Comparison(Box<ComparisonContinuation>),
    }
    value
        .then(choice((
            parse_null_check_continuation().map(|(rikta, negated)| Predicate::Null(rikta, negated)),
            parse_comparison_continuation(rhs_parser).map(|c| Predicate::Comparison(Box::new(c))),
        )).or_not())
        .map(|(left, predicate)| match predicate {
            Some(Predicate::Null(rikta, negated)) => build_null_check_expr(left, rikta, negated),
            Some(Predicate::Comparison(c)) => build_comparison_expr(left, *c),
            None => left,
        })
        .boxed()
}

/// Succeeds, consuming nothing, if a token `want` accepts occurs before the
/// statement's `।`: a cheap scan that spares a form the cost of re-parsing its
/// first operand, which may be a whole group, when the token it needs is not
/// there. It rewinds before it fails, so its error sits where it started: a
/// combinator scan failed at the `।` and, being the furthest error, hid every
/// real one.
fn ahead<'a>(want: fn(&Token) -> bool) -> impl Parser<'a, &'a [Token], (), extra::Err<Rich<'a, Token>>> + Clone {
    custom(move |inp| {
        let before = inp.save();
        let start = inp.cursor();
        let mut found = false;
        while let Some(t) = inp.next() {
            if want(&t) {
                found = true;
                break;
            }
            if matches!(t, Token::Purnadanda) {
                break;
            }
        }
        inp.rewind(before);
        if found { Ok(()) } else { Err(Rich::custom(inp.span_since(&start), "")) }
    })
}

/// `ahead` for a subquery, whose verb is followed by its `इति`. A plain group
/// has neither, and the statement's own verb is followed by `।`.
fn subquery_ahead<'a>() -> impl Parser<'a, &'a [Token], (), extra::Err<Rich<'a, Token>>> + Clone {
    custom(move |inp| {
        let before = inp.save();
        let start = inp.cursor();
        let mut found = false;
        let mut after_verb = false;
        while let Some(t) = inp.next() {
            if after_verb && matches!(t, Token::Iti) {
                found = true;
                break;
            }
            if matches!(t, Token::Purnadanda) {
                break;
            }
            after_verb = matches!(t, Token::FetchVerb(_));
        }
        inp.rewind(before);
        if found { Ok(()) } else { Err(Rich::custom(inp.span_since(&start), "")) }
    })
}

/// Everything that can follow a comparison LHS: the RHS (atom or IN-list),
/// then its pronoun, the predicate, and अस्ति / नास्ति. Each predicate
/// governs one case (expert review Q1, F4):
/// - `<v> इत्यस्मात्/इत्यस्याः अधिकः|न्यूनः अस्ति|नास्ति` (> < <= >=)
/// - `<v> इत्यनेन/इत्यनया समः|सदृशः अस्ति|नास्ति` (= <> LIKE NOT LIKE)
/// - `<a> च <b> च … इत्येषु/इत्यनयोः अस्ति|नास्ति` (IN, NOT IN)
/// - `<lo> च <hi> च इत्यनयोः मध्ये अस्ति|नास्ति` (BETWEEN, NOT BETWEEN)
/// A comparison without its predicate is an error, never a guessed `=`.
/// Factored out of `parse_comparison` so a LHS that has already been parsed
/// once (e.g. `suffix_part`, see `parse_expr_atom`) can check for this
/// continuation with `.or_not()` without re-deriving the LHS from scratch.
type ComparisonContinuation = (Expression, Option<Option<Token>>, Option<Token>, Option<(BinaryOperator, String)>, bool);

fn parse_comparison_continuation<'a>(
    rhs_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], ComparisonContinuation, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let connector = choice((
        any().filter(|t| matches!(t, Token::Ityasmat(_))).map(|tok| match tok {
            Token::Ityasmat(s) => Some(Token::Ityasmat(s)),
            _ => unreachable!(),
        }),
        any().filter(|t| matches!(t, Token::Ityasyah(_))).map(|tok| match tok {
            Token::Ityasyah(s) => Some(Token::Ityasyah(s)),
            _ => unreachable!(),
        }),
        any().filter(|t| matches!(t, Token::Tritiya { .. })).map(Some),
        any().filter(|t| matches!(t, Token::Ityeshu(_))).map(Some),
        any().filter(|t| matches!(t, Token::Ityayoh(_))).map(|tok| match tok {
            Token::Ityayoh(s) => Some(Token::Ityayoh(s)),
            _ => unreachable!(),
        }),
    ));

    // RHS: one value, or for IN a च-list of values.
    let atom_or_list = rhs_parser.clone();

    atom_or_list
        .then(connector.or_not())
        .then(just(Token::Madhye).or_not()) // मध्ये (BETWEEN)
        .then(parse_comparison_op().or_not())
        .then(choice((just(Token::Asti).to(false), just(Token::Nasti).to(true))))
        .try_map(|((((right, connector_opt), madhye_opt), op_opt), negated), span| {
            let ablative = matches!(connector_opt, Some(Some(Token::Ityasmat(_) | Token::Ityasyah(_))));
            let instrumental = matches!(connector_opt, Some(Some(Token::Tritiya { .. })));
            let dual = matches!(connector_opt, Some(Some(Token::Ityayoh(_))));
            let in_list = dual || matches!(connector_opt, Some(Some(Token::Ityeshu(_))));
            let op = op_opt.as_ref().map(|(op, _)| op.clone());
            let valid = match (&op, madhye_opt.is_some()) {
                (None, true) => dual,
                (None, false) => in_list,
                (Some(BinaryOperator::Greater | BinaryOperator::Less), false) => ablative,
                (Some(BinaryOperator::Equal | BinaryOperator::Like), false) => instrumental,
                (Some(BinaryOperator::Regexp), false) => connector_opt.is_none() && !negated,
                _ => false,
            };
            if !valid {
                return Err(Rich::custom(span, match (&op, connector_opt.is_some()) {
                    (None, _) if madhye_opt.is_none() && !in_list => "A comparison needs its predicate: `<value> इत्यस्मात् अधिकम् / न्यूनम् अस्ति`, `<value> इत्यनेन समम् / सदृशम् अस्ति`, or a list `… इत्येषु अस्ति`",
                    (Some(BinaryOperator::Greater | BinaryOperator::Less), _) => "अधिक / न्यून take the ablative pronoun: `<value> इत्यस्मात् अधिकम् अस्ति` (feminine `इत्यस्याः`)",
                    (Some(BinaryOperator::Equal | BinaryOperator::Like), _) => "सम / सदृश take the instrumental pronoun: `<value> इत्यनेन समम् अस्ति` (feminine `इत्यनया`)",
                    _ => "'मध्ये' (BETWEEN) is written `<low> च <high> च इत्यनयोः मध्ये अस्ति`",
                }));
            }
            // BETWEEN has exactly two bounds, as the dual इत्यनयोः says: a single
            // bound used to become `=`, and three became `BETWEEN (a AND b) AND c`.
            if madhye_opt.is_some() {
                let two_bounds = matches!(
                    &right,
                    Expression::LogicalOp { left, op: crate::ast::LogicalOperator::And, right }
                        if !matches!(**left, Expression::LogicalOp { .. }) && !matches!(**right, Expression::LogicalOp { .. })
                );
                if !two_bounds {
                    return Err(Rich::custom(span, "'मध्ये' (BETWEEN) takes exactly two bounds: '<low> च <high> च इत्यनयोः मध्ये'"));
                }
            }
            Ok((right, connector_opt, madhye_opt, op_opt, negated))
        })
}

fn build_comparison_expr(left: Expression, continuation: ComparisonContinuation) -> Expression {
    let (right, connector_opt, madhye_opt, op_opt, negated) = continuation;
            if madhye_opt.is_some() {
                // BETWEEN comparison: मूल्यम् (२० च ५० च) इत्यनयोः मध्ये अस्ति
                if let Expression::LogicalOp { left: low, op: _, right: high } = right {
                    return Expression::Between {
                        expr: Box::new(left),
                        low,
                        high,
                        negated,
                    };
                }
            }

            // The continuation has already checked the predicate; नास्ति
            // negates it (न्यूनम् नास्ति "is not less" is >=).
            let positive = match (&op_opt, &connector_opt) {
                (Some((op, _)), _) => op.clone(),
                (None, _) => BinaryOperator::In,
            };
            let final_op = match (positive, negated) {
                (op, false) => op,
                (BinaryOperator::Equal, true) => BinaryOperator::NotEqual,
                (BinaryOperator::Greater, true) => BinaryOperator::LessEqual,
                (BinaryOperator::Less, true) => BinaryOperator::GreaterEqual,
                (BinaryOperator::Like, true) => BinaryOperator::NotLike,
                (BinaryOperator::In, true) => BinaryOperator::NotIn,
                (op, true) => unreachable!("negated {:?} was rejected by the continuation", op),
            };
            
            let pronoun_id = match &connector_opt {
                Some(Some(Token::Tritiya { text, vachana, pratipadika })) => {
                    Some(crate::ast::Identifier {
                        text: text.clone(),
                        vibhakti_form: VibhaktiForm::new(Vibhakti::Tritiya, *vachana),
                        pratipadika: pratipadika.clone(),
                        symbol_type: SymbolType::Variable,
                        span: None,
                    })
                }
                Some(Some(Token::Ityeshu(s))) => {
                    Some(crate::ast::Identifier {
                        text: s.clone(),
                        vibhakti_form: VibhaktiForm::new(Vibhakti::Saptami, Vachana::Bahu),
                        pratipadika: "इति".to_string(),
                        symbol_type: SymbolType::Variable,
                        span: None,
                    })
                }
                Some(Some(Token::Ityasmat(s))) => {
                    Some(crate::ast::Identifier {
                        text: s.clone(),
                        vibhakti_form: VibhaktiForm::new(Vibhakti::Panchami, Vachana::Eka),
                        pratipadika: "इति".to_string(),
                        symbol_type: SymbolType::Variable,
                        span: None,
                    })
                }
                Some(Some(Token::Ityayoh(s))) => {
                    Some(crate::ast::Identifier {
                        text: s.clone(),
                        vibhakti_form: VibhaktiForm::new(Vibhakti::Shashthi, Vachana::Dvi),
                        pratipadika: "इति".to_string(),
                        symbol_type: SymbolType::Variable,
                        span: None,
                    })
                }
                Some(Some(Token::Ityesham(s))) => {
                    Some(crate::ast::Identifier {
                        text: s.clone(),
                        vibhakti_form: VibhaktiForm::new(Vibhakti::Shashthi, Vachana::Bahu),
                        pratipadika: "इति".to_string(),
                        symbol_type: SymbolType::Variable,
                        span: None,
                    })
                }
                Some(Some(Token::Ityetasam(s))) => {
                    let mut vib = VibhaktiForm::new(Vibhakti::Shashthi, Vachana::Bahu);
                    vib.linga = Some(crate::types::Linga::Feminine);
                    Some(crate::ast::Identifier {
                        text: s.clone(),
                        vibhakti_form: vib,
                        pratipadika: "इति".to_string(),
                        symbol_type: SymbolType::Variable,
                        span: None,
                    })
                }
                Some(Some(Token::Ityasya(s))) => {
                    Some(crate::ast::Identifier {
                        text: s.clone(),
                        vibhakti_form: VibhaktiForm::new(Vibhakti::Shashthi, Vachana::Eka),
                        pratipadika: "इति".to_string(),
                        symbol_type: SymbolType::Variable,
                        span: None,
                    })
                }
                Some(Some(Token::Ityasyah(s))) => {
                    let mut vib = VibhaktiForm::new(Vibhakti::Shashthi, Vachana::Eka);
                    vib.linga = Some(crate::types::Linga::Feminine);
                    Some(crate::ast::Identifier {
                        text: s.clone(),
                        vibhakti_form: vib,
                        pratipadika: "इति".to_string(),
                        symbol_type: SymbolType::Variable,
                        span: None,
                    })
                }
                _ => None,
            };

            let adjective_id = match &op_opt {
                Some((op, adj_spelling)) => {
                    Some(crate::ast::Identifier {
                        text: adj_spelling.clone(),
                        vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                        pratipadika: match op {
                            BinaryOperator::Equal => "सम".to_string(),
                            BinaryOperator::Greater => "अधिक".to_string(),
                            BinaryOperator::Less => "न्यून".to_string(),
                            BinaryOperator::NotEqual => "भिन्न".to_string(),
                            _ => adj_spelling.clone(),
                        },
                        symbol_type: SymbolType::Variable,
                        span: None,
                    })
                }
                _ => None,
            };
            
            Expression::SanskritComparison {
                left: Box::new(left),
                op: final_op,
                right: Box::new(right),
                pronoun: pronoun_id,
                adjective: adjective_id,
            }
}

/// The function-name prefix that carries a CAST target through the suffix
/// parsers (never a real name: it starts with a control character).
const CAST_MARK: &str = "\u{1}cast:";

/// `ऋणात्मकम् मूल्यम्`: the adjective before a name makes its value negative, and
/// agrees with it (the validator checks). A constant has its own sign (`-५`), and
/// anything longer is named first. The word is kept in `pronoun_token`.
fn negated<'a>(
    value: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> Boxed<'a, 'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> {
    any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Rinatmaka(text) => Ok(Identifier {
                pratipadika: "ऋणात्मक".to_string(),
                text,
                vibhakti_form: VibhaktiForm::new(Vibhakti::Prathama, Vachana::Eka),
                symbol_type: SymbolType::Variable,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected a form of `ऋणात्मक`")),
        })
        .then(value)
        .validate(|(word, value), e, emitter| {
            if !matches!(value, Expression::Identifier(_) | Expression::FieldAccess { .. }) {
                let message = if matches!(value, Expression::Literal(_)) {
                    "a negative constant is written with its sign, `-५`; `ऋणात्मकम्` is for a column"
                } else {
                    "`ऋणात्मकम्` goes before a column; name a longer value first, then make the name negative"
                };
                emitter.emit(Rich::custom(e.span(), message));
            }
            Expression::FunctionCall {
                name: "ऋणात्मक".to_string(),
                args: vec![value],
                window: None,
                pronoun_token: Some(word),
            }
        })
        .boxed()
}

/// A `यत् <expr> इति` group. A group around a च-list is needed only as an
/// item of a वा-list, and a वा-list's only in a च-list (expert review Q7);
/// anywhere else it is redundant, and in a select list it used to turn
/// `यत् शीर्षकम् च मूल्यम् च इति` into `title AND price`. Reported without
/// failing the parse, so nothing masks it.
///
/// Every other group on the final parse is marked (`GROUP_MARK`, its span)
/// for `redundant_group`, except one around an embedded statement, which
/// keeps its group (Q7). A second group around a subquery that has its own
/// (`यत् यत् … आह्रियताम् इति इति`) is an error: it turned INSERT…SELECT
/// into `VALUES ((SELECT …))`.
fn grouped<'a>(
    inner: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    use crate::ast::LogicalOperator::{And, Or};
    just(Token::Yat)
        .ignore_then(inner)
        .then_ignore(just(Token::Iti))
        .then(any().or_not().rewind())
        .validate(|(expr, next), e, emitter| {
            let slice: &[Token] = e.slice();
            let statement = matches!(expr, Expression::Subquery(_) | Expression::SetOp(_) | Expression::Exists { .. });
            if statement && matches!(slice.get(1), Some(Token::Yat)) {
                emitter.emit(Rich::custom(e.span(),
                    "this subquery already has its `यत् … इति`; a second group around it is redundant, so remove the outer `यत्` and `इति`"));
            } else if let Expression::LogicalOp { op, .. } = &expr {
                let (own, other) = if *op == And { ("च", "वा") } else { ("वा", "च") };
                if !matches!((op, &next), (And, Some(Token::Va)) | (Or, Some(Token::Cha))) {
                    emitter.emit(Rich::custom(e.span(), format!(
                        "a `यत् … इति` group around a {own}-list is needed only as an item of a {other}-list; here it is not, so remove `यत्` and `इति`"
                    )));
                } else {
                    emitter.emit(Rich::custom(e.span(), super::GROUP_MARK));
                }
            } else if !statement {
                emitter.emit(Rich::custom(e.span(), super::GROUP_MARK));
            }
            expr
        })
}

/// Parse CASE expression: (वरणम् यस्मिन् ... तस्मिन् ... अन्यस्मिन् ...)

pub(crate) fn parse_case_expression<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // A branch's value is a single value - a literal, a column (optionally
    // qualified), a scalar function of one - or an explicit `यत् … इति`
    // group. CASE has no end marker, so a full expression here swallowed
    // whatever followed: the next select item's `च` (`अन्यस्मिन् ० च शीर्षकम्`
    // -> ELSE (0 AND title)). The WHEN condition, which `तस्मिन्` ends, stays
    // a full expression.
    let branch_value = choice((
        grouped(expr_parser.clone()),
        negated(choice((parse_field_access(), parse_atom()))),
        parse_scalar_function(),
        parse_field_access(),
        parse_atom(),
    ));

    let when_clause = just(Token::Yasmin)
        .ignore_then(expr_parser.clone())
        .then_ignore(just(Token::Tasmin))
        .then(branch_value.clone())
        .map(|(cond, result)| CaseArm {
            condition: Some(Box::new(cond)),
            result: Box::new(result),
        });

    let else_clause = just(Token::Anyasmin)
        .ignore_then(branch_value)
        .map(|res| CaseArm {
            condition: None,
            result: Box::new(res),
        });

    just(Token::VaraNam)
        .ignore_then(when_clause.repeated().at_least(1).collect::<Vec<_>>())
        .then(else_clause.or_not())
        .map(|(mut arms, else_arm)| {
             if let Some(arm) = else_arm {
                 arms.push(arm);
             }
             Expression::Case { cases: arms }
        })
}


pub(crate) fn parse_frame_units<'a>() -> impl Parser<'a, &'a [Token], crate::ast::FrameUnits, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    choice((
        just(Token::Panktayah).to(crate::ast::FrameUnits::Rows),
        just(Token::Vyaptih).to(crate::ast::FrameUnits::Range),
        just(Token::Samuhah).to(crate::ast::FrameUnits::Groups),
    ))
}


pub(crate) fn parse_frame_bound<'a>() -> Boxed<'a, 'a, &'a [Token], crate::ast::FrameBound, extra::Err<Rich<'a, Token>>> {
    choice((
        just(Token::AparimitaPurvavartitah).to(crate::ast::FrameBound::UnboundedPreceding),
        just(Token::AparimitaAnuvartitah).to(crate::ast::FrameBound::UnboundedFollowing),
        just(Token::Vartamanapanktih).to(crate::ast::FrameBound::CurrentRow),
        parse_atom()
            .then_ignore(just(Token::Purvavartitah))
            .map(|e| crate::ast::FrameBound::Preceding(Box::new(e))),
        parse_atom()
            .then_ignore(just(Token::Anuvartitah))
            .map(|e| crate::ast::FrameBound::Following(Box::new(e))),
    ))
    .boxed()
}


pub(crate) fn parse_frame_exclude<'a>() -> impl Parser<'a, &'a [Token], crate::ast::FrameExclude, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    just(Token::Varjayitva).ignore_then(choice((
        just(Token::Vartamanapanktih).to(crate::ast::FrameExclude::CurrentRow),
        just(Token::Samuham).to(crate::ast::FrameExclude::Group),
        just(Token::Sambaddhan).to(crate::ast::FrameExclude::Ties),
        just(Token::Na).then(just(Token::Anyan)).to(crate::ast::FrameExclude::NoOthers),
    )))
}


pub(crate) fn parse_window_frame<'a>() -> Boxed<'a, 'a, &'a [Token], crate::ast::WindowFrame, extra::Err<Rich<'a, Token>>> {
    let between_frame = parse_frame_bound()
        .then_ignore(just(Token::Aarabhya))
        .then(parse_frame_bound())
        .then_ignore(just(Token::Paryantam))
        .map(|(start, end)| (start, Some(end)));

    let single_frame = parse_frame_bound()
        .then_ignore(just(Token::Aarabhya).or_not())
        .map(|start| (start, None));

    parse_frame_units()
        .then(choice((between_frame, single_frame)))
        .then(parse_frame_exclude().or_not())
        .map(|((units, (start, end)), exclude)| crate::ast::WindowFrame {
            units,
            start,
            end,
            exclude,
        })
        .boxed()
}

/// The window of a window function, after its यत्र:
/// `<genitive keys> आधारेण विभाजनम्` (PARTITION BY) and
/// `<genitive keys> अनुसारम् आरोहक्रमेण|अवरोहक्रमेण` (ORDER BY), in either
/// order, then an optional frame. Every segment ends in its own marker
/// (विभाजनम्, the direction, the frame's पर्यन्तम्/आरभ्य), so a select-item
/// alias can follow directly: `श्रेणी यत्र … अवरोहक्रमेण इति क्रमः`.

pub(crate) fn parse_window_spec<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> Boxed<'a, 'a, &'a [Token], crate::ast::WindowSpec, extra::Err<Rich<'a, Token>>> {
    // Natural Sanskrit Syntax (Aadharena / Anusaram). Partition and Order clauses
    // may appear in either order, so each is parsed as a single "segment" tried
    // once per position (not as two full alternative sequences retried against
    // each other) - this avoids re-invoking the full recursive expression parser
    // in overlapping, backtracking ways for the same input.

    // A key is a genitive column, or any expression put into the genitive
    // by the pronoun (`समीक्षाश्रेणीमाध्यम् इत्यस्य अनुसारम्`, `… योगः इत्यस्य
    // अनुसारम्`), as aggregate arguments are; the bool marks the pronoun form.
    //
    // An aggregate key declines its own noun (`सकलमूल्यम् इत्यस्य योगस्य
    // अनुसारम्`, `मूल्यस्य योगस्य अनुसारम्`) rather than stacking a second
    // इति-pronoun on it (expert review G2).
    let genitive_pronoun = any().filter(|t| matches!(t, Token::Ityasya(_) | Token::Ityasyah(_)));
    let genitive_agg = choice((
        just(Token::YogasyaGen).to(AggFunc::Sum),
        just(Token::GananayahGen).to(AggFunc::Count),
        just(Token::MadhyasyaGen).to(AggFunc::Avg),
        just(Token::MahattamasyaGen).to(AggFunc::Max),
        just(Token::LaghutamasyaGen).to(AggFunc::Min),
    ));
    let genitive_column = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
            text,
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
            pratipadika,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "expected a genitive column")),
    });
    let aggregate_argument = choice((
        expr_parser.clone().then_ignore(genitive_pronoun.clone()),
        genitive_column.clone()
            .then(genitive_column.clone().or_not())
            .map(|(first, second)| match second {
                Some(field) => Expression::FieldAccess { object: Box::new(Expression::Identifier(first)), field },
                None => Expression::Identifier(first),
            }),
    ));
    let aggregate_key = aggregate_argument
        .then(genitive_agg)
        .map(|(arg, func)| Expression::Aggregate {
            expr: Box::new(arg),
            func,
            distinct: false,
            distinct_token: None,
            pronoun_token: None,
        });
    let key = choice((
        aggregate_key.map(|e| (e, false)),
        expr_parser.clone().then_ignore(genitive_pronoun).map(|e| (e, true)),
        expr_parser.clone().map(|e| (e, false)),
    ));

    let partition_clause = key.clone()
        .then_ignore(just(Token::Aadharena))
        .then_ignore(just(Token::Vibhajanam));

    let order_direction = choice((
        just(Token::ArohaKramena).to(crate::ast::OrderDirection::Asc),
        just(Token::AvarohaKramena).to(crate::ast::OrderDirection::Desc),
    ));

    let order_clause = key
        .then_ignore(just(Token::Anusaram))
        .then(order_direction)
        .map(|((e, by_pronoun), direction)| (crate::ast::OrderExpr { expr: Box::new(e), direction }, by_pronoun));

    let frame_clause = parse_window_frame();

    #[derive(Clone)]
    enum WindowSegment {
        Partition((Expression, bool)),
        Order((crate::ast::OrderExpr, bool)),
    }

    // Each segment (partition clause or order clause) is tried once per
    // position; the segment list naturally supports either ordering without
    // needing two full alternative parses of the same content.
    let segment = choice((
        partition_clause.map(WindowSegment::Partition),
        order_clause.map(WindowSegment::Order),
    ));

    just(Token::Yatra)
        .ignore_then(segment.repeated().collect::<Vec<_>>())
        .then(frame_clause.or_not())
        .try_map(|(segments, frame), span| {
            if segments.is_empty() && frame.is_none() {
                return Err(Rich::custom(span, "expected partition, order, or frame clause after Yatra"));
            }
            // `भाषायाः च विधायाः च आधारेण` is a list of keys, not a boolean
            // conjunction: the expression parser reads a च-list as AND, so
            // split it back into one key per item (an ordering's direction
            // applies to each of its keys).
            fn list_items(expr: Expression, out: &mut Vec<Expression>) {
                match expr {
                    Expression::LogicalOp { left, op: crate::ast::LogicalOperator::And, right } => {
                        list_items(*left, out);
                        list_items(*right, out);
                    }
                    other => out.push(other),
                }
            }
            // आधारेण and अनुसारम् govern the genitive: भाषायाः आधारेण,
            // मूल्यस्य अनुसारम् (not भाषा / मूल्यम्).
            fn is_nominative_column(e: &Expression) -> bool {
                match e {
                    Expression::Identifier(id) => id.vibhakti_form.vibhakti != Vibhakti::Shashthi,
                    Expression::FieldAccess { field, .. } => field.vibhakti_form.vibhakti != Vibhakti::Shashthi,
                    Expression::LogicalOp { left, right, .. } => is_nominative_column(left) || is_nominative_column(right),
                    _ => false,
                }
            }
            for seg in &segments {
                let (key, by_pronoun) = match seg {
                    WindowSegment::Partition((e, p)) => (e, *p),
                    WindowSegment::Order((o, p)) => (&*o.expr, *p),
                };
                if !by_pronoun && is_nominative_column(key) {
                    return Err(Rich::custom(span, "आधारेण / अनुसारम् take the genitive: `भाषायाः आधारेण विभाजनम्`, `मूल्यस्य अनुसारम् अवरोहक्रमेण`"));
                }
            }
            let mut partition_by = vec![];
            let mut order_by = vec![];
            for seg in segments {
                match seg {
                    WindowSegment::Partition((e, _)) => list_items(e, &mut partition_by),
                    WindowSegment::Order((o, _)) => {
                        let mut keys = Vec::new();
                        list_items(*o.expr, &mut keys);
                        for key in keys {
                            order_by.push(crate::ast::OrderExpr { expr: Box::new(key), direction: o.direction.clone() });
                        }
                    }
                }
            }
            Ok(crate::ast::WindowSpec { partition_by, order_by, frame })
        })
        .boxed()
}


pub(crate) fn parse_window_function<'a>(
    expr_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // Ranking functions: श्रेणी -> RANK(), सघनश्रेणी -> DENSE_RANK(),
    // पङ्क्तिसङ्ख्या -> ROW_NUMBER()
    let rank = choice((
        just(Token::Shreni).to("RANK".to_string()),
        just(Token::SaghanaShreni).to("DENSE_RANK".to_string()),
        just(Token::PanktiSankhya).to("ROW_NUMBER".to_string()),
    ))
        .then(parse_window_spec(expr_parser.clone()).map(Some)) // a window function needs its window
        .map(|(name, window)| Expression::FunctionCall {
            name,
            args: vec![],
            window,
            pronoun_token: None,
        });

    // LAG / LEAD: the previous / next row's value, `पूर्वपङ्क्तेः मूल्यम्`,
    // `अनन्तरपङ्क्तेः पुस्तकस्य मूल्यम्` (expert review F6).
    let lead_lag = choice((
        just(Token::PurvaPankteh).to(("LAG".to_string(), None)),
        just(Token::AnantaraPankteh).to(("LEAD".to_string(), None)),
        // By n rows: `द्वितीयपूर्वपङ्क्तेः मूल्यम्` is LAG(price, 2) (pcf.md item 6).
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::PurvaPanktehN(n) => Ok(("LAG".to_string(), Some(n))),
            Token::AnantaraPanktehN(n) => Ok(("LEAD".to_string(), Some(n))),
            _ => Err(Rich::custom(span, "expected the previous or next row")),
        }),
    ))
        // The column takes the case of its position: nominative as a
        // select item, accusative as a subtrahend (`… पूर्वपङ्क्तेः
        // सम्पर्कसमयम् … विशोध्य`).
        .then(choice((
            parse_atom(),
            any().try_map(|tok, span: SimpleSpan| match tok {
                Token::Dvitiya { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
                    text,
                    vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Dvitiya, vachana, linga: None },
                    pratipadika,
                    symbol_type: SymbolType::Column,
                    span: Some((span.start(), span.end())),
                })),
                _ => Err(Rich::custom(span, "expected the column of the previous / next row")),
            }),
        )))
        .then(parse_window_spec(expr_parser.clone()).map(Some)) // a window function needs its window
        .map(|(((name, offset), expr), window)| Expression::FunctionCall {
            name,
            args: std::iter::once(expr)
                .chain(offset.map(|n: u32| Expression::Literal(Literal::Number(n.to_string()))))
                .collect(),
            window,
            pronoun_token: None,
        });

    // NTILE: `चतुर्थांशाङ्कः` (quartile, NTILE(4)) and `<n> भागेषु भागाङ्कः`
    // ("the part-number among n parts"; two parts: `२ भागयोः भागाङ्कः`).
    let quartile = just(Token::Chaturthamshankah)
        .to(Expression::Literal(Literal::Number("4".to_string())));
    let n_parts = any()
        .try_map(|tok, span: SimpleSpan| match tok {
            Token::Number(n) => Ok(n),
            _ => Err(Rich::custom(span, "expected the number of parts")),
        })
        .then(any().try_map(|tok: Token, span: SimpleSpan| match tok.to_string().as_str() {
            "भागेषु" => Ok(false),
            "भागयोः" => Ok(true),
            _ => Err(Rich::custom(span, "expected भागेषु (or भागयोः for two parts)")),
        }))
        .then_ignore(just(Token::Bhagankah))
        .try_map(|(n, dual), span| {
            let ascii: String = n.chars().map(|c| match c {
                '०'..='९' => char::from_digit(c as u32 - '०' as u32, 10).unwrap(),
                other => other,
            }).collect();
            match ascii.parse::<u32>() {
                Ok(2) if dual => Ok(Expression::Literal(Literal::Number(n))),
                Ok(k) if k >= 3 && !dual => Ok(Expression::Literal(Literal::Number(n))),
                _ => Err(Rich::custom(span, "`<n> भागेषु भागाङ्कः` needs n ≥ 3, and two parts are `२ भागयोः भागाङ्कः`")),
            }
        });
    let ntile = choice((quartile, n_parts))
        .then(parse_window_spec(expr_parser.clone()).map(Some)) // a window function needs its window
        .map(|(parts, window)| Expression::FunctionCall {
            name: "NTILE".to_string(),
            args: vec![parts],
            window,
            pronoun_token: None,
        });

    choice((rank, lead_lag, ntile))
}


pub(crate) fn parse_scalar_function<'a>() -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    scalar_function_after(|column| column)
}

/// A function of several operands, whose first is a genitive: a column, or what
/// `first_operand` makes of that (`parse_expr_atom` adds the genitive of an
/// aggregate or arithmetic result).
fn scalar_function_after<'a, F, P>(first_operand: F) -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
where
    F: FnOnce(Boxed<'a, 'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>>) -> P,
    P: Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
{
    let genitive_field_access = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
            text,
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
            pratipadika,
            symbol_type: SymbolType::Table,
            span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "expected genitive")),
    })
    .then(
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
                pratipadika,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected genitive field")),
        })
    )
    .map(|(tbl, col)| Expression::FieldAccess {
        object: Box::new(Expression::Identifier(tbl)),
        field: col,
    })
    .or(
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Shashthi { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
                pratipadika,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            })),
            _ => Err(Rich::custom(span, "expected genitive")),
        })
        .then_ignore(
            any().filter(|tok| matches!(
                tok,
                Token::Prathama { .. }
                    | Token::Identifier(_)
                    | Token::Saptami { .. }
                    | Token::Panchami { .. }
                    | Token::Dvitiya { .. }
            )).not().rewind()
        )
    );

    let case_marker = choice((
        any().filter(|tok| match tok {
            Token::Tritiya { .. } => true,
            Token::Panchami { .. } => true,
            Token::Shashthi { .. } => true,
            Token::Saptami { .. } => true,
            Token::Ityasya(_) => true,
            Token::Ityasyah(_) => true,
            Token::Ityasmat(_) => true,
            Token::Ityesham(_) => true,
            Token::Ityetasam(_) => true,
            Token::Ityayoh(_) => true,
            _ => false,
        }),
        any().filter(|t| matches!(t, Token::Ityeshu(_))),
    ));

    let tritiya_ident_arg = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Tritiya { text, vachana, pratipadika } if pratipadika != "इति" => Ok(Expression::Identifier(Identifier {
            text,
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana, linga: None },
            pratipadika,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        })),
        _ => Err(Rich::custom(span, "expected instrumental identifier argument")),
    });

    // Each further operand keeps its marker, for the check below.
    let case_of = |marker: Option<Token>| match marker {
        Some(Token::Tritiya { text, pratipadika, .. }) if pratipadika == "इति" => OperandCase::Instrumental(Some(text)),
        Some(Token::Ityasya(_) | Token::Ityasyah(_)) => OperandCase::Genitive,
        _ => OperandCase::Other,
    };
    let case_arg = choice((
        parse_literal().then(case_marker.clone().or_not()).map(move |(e, m)| (e, case_of(m))),
        parse_field_access().then(case_marker.clone().or_not()).map(move |(e, m)| {
            let inherent = matches!(&e, Expression::FieldAccess { field, .. } if field.vibhakti_form.vibhakti == Vibhakti::Tritiya);
            let case = if inherent && m.is_none() { OperandCase::Instrumental(None) } else { case_of(m) };
            (e, case)
        }),
        tritiya_ident_arg.map(|e| (e, OperandCase::Instrumental(None))),
        parse_identifier_expr().then(case_marker).map(move |(e, m)| (e, case_of(Some(m)))),
    ));

    let function_name = choice((
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Identifier(name) if crate::reserved_words::is_reserved_builtin(&name) => Ok(name),
            Token::Prathama { text, pratipadika, .. }
                if crate::reserved_words::is_reserved_builtin(&text)
                    || crate::reserved_words::is_reserved_builtin(&pratipadika) =>
            {
                Ok(text)
            }
            _ => Err(Rich::custom(span, "expected function name")),
        }),
        just(Token::Mahavarnarupam).to("महावर्णरूपम्".to_string()),
        just(Token::Laghuvarnarupam).to("लघुवर्णरूपम्".to_string()),
        just(Token::Samyojanam).to("संयोजनम्".to_string()),
        just(Token::Aayamah).to("दैर्घ्यम्".to_string()),
        just(Token::Vikalpah).to("विकल्पः".to_string()),
        just(Token::Avashistam).to("अवशिष्टम्".to_string()),
        just(Token::Yogah).to("योगः".to_string()),
        just(Token::Ganana).to("गणना".to_string()),
        just(Token::Madhyam).to("माध्यम्".to_string()),
        just(Token::Mahattam).to("महत्तमम्".to_string()),
        just(Token::Laghutamam).to("लघुतमम्".to_string()),
    ));

    first_operand(genitive_field_access.boxed())
        .then(case_arg.repeated().at_least(1).collect::<Vec<_>>())
        .then(function_name)
        // A further operand is the instrumental, "with": `मूल्यस्य २ इत्यनेन
        // शेषः`, by a column's own instrumental or by the pronoun, which agrees
        // with the first operand (the validator checks that). Only what
        // `प्रतिस्थापनम्` replaces is a genitive: `शीर्षकस्य "a" इत्यस्य "o" इत्यनेन`.
        .validate(|((first_arg, rest_args), name), e, emitter| {
            let replaces = name == "प्रतिस्थापनम्" && rest_args.len() == 2;
            let mut pronouns = Vec::new();
            for (i, (_, case)) in rest_args.iter().enumerate() {
                match case {
                    OperandCase::Genitive if replaces && i == 0 => {}
                    OperandCase::Instrumental(p) if !(replaces && i == 0) => pronouns.extend(p.clone()),
                    _ if replaces && i == 0 => emitter.emit(Rich::custom(e.span(),
                        "what `प्रतिस्थापनम्` replaces is in the genitive: `शीर्षकस्य \"a\" इत्यस्य \"o\" इत्यनेन प्रतिस्थापनम्`")),
                    _ => emitter.emit(Rich::custom(e.span(),
                        "a function's further operand is in the instrumental: `मूल्यस्य २ इत्यनेन शेषः`, with `इत्यनया` for a feminine column, or the column's own instrumental")),
                }
            }
            let pronoun_token = pronouns.pop().map(|text| Identifier {
                text,
                vibhakti_form: VibhaktiForm::new(Vibhakti::Tritiya, Vachana::Eka),
                pratipadika: "इति".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            });
            let mut args = vec![first_arg];
            args.extend(rest_args.into_iter().map(|(arg, _)| arg));
            Expression::FunctionCall {
                name,
                args,
                window: None,
                pronoun_token,
            }
        })
}

/// The case a function's further operand is in.
#[derive(Clone)]
enum OperandCase {
    /// `इत्यनेन` / `इत्यनया` (their text), or a column's own instrumental.
    Instrumental(Option<String>),
    Genitive,
    Other,
}

/// Kāraka-marked locative string operations on a `[Shashthi] Saptami` source:
/// 1. `विवरणे "ERR" इत्यस्य स्थानात् आरभ्य ६ वर्णानाम् खण्डः`
///    => `SUBSTR(description, INSTR(description, 'ERR'), 6)`
/// 2. `विक्रयाधिकारिणः नाम्नि १ स्थानात् आरभ्य ३ वर्णानाम् खण्डः` (or `१ आरभ्य ३ वर्णानाम् खण्डः`)
///    => `SUBSTR(sales_reps.name, 1, 3)`
/// 3. `विवरणे "ERR" इत्यस्य स्थानम्`
///    => `INSTR(description, 'ERR')`
pub(crate) fn parse_locative_substr<'a>() -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let saptami_col = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Saptami { text, vachana, pratipadika } => Ok(Identifier {
            text,
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana, linga: None },
            pratipadika,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "expected locative source")),
    });

    let qualified_saptami = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
            text,
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
            pratipadika,
            symbol_type: SymbolType::Table,
            span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "expected genitive qualifier")),
    })
    .then(saptami_col.clone())
    .map(|(tbl, col)| Expression::FieldAccess {
        object: Box::new(Expression::Identifier(tbl)),
        field: col,
    });

    let source = choice((
        qualified_saptami,
        saptami_col.map(Expression::Identifier),
    ));

    let search_target = parse_literal();

    let ityasya = any().filter(|tok| matches!(tok, Token::Ityasya(_) | Token::Ityasyah(_)));

    let num_tok = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Number(n) => Ok(n),
        _ => Err(Rich::custom(span, "expected a bare numeral")),
    });

    let sthaanam_tok = any().filter(|tok| match tok {
        Token::Prathama { text, .. } | Token::Identifier(text) => text == "स्थानम्" || text == "स्थान",
        _ => false,
    });

    enum LocativeStringOp {
        ChainedSubstrInstr { needle: Expression, length: String },
        DirectSubstr { start: String, length: String },
        StandaloneInstr { needle: Expression },
    }

    let chained_substr_instr = search_target.clone()
        .then_ignore(ityasya.clone())
        .then_ignore(just(Token::Sthaanaat))
        .then_ignore(just(Token::Aarabhya))
        .then(num_tok.clone())
        .then_ignore(just(Token::Varnaanaam))
        .then_ignore(just(Token::Khandah))
        .map(|(needle, length)| LocativeStringOp::ChainedSubstrInstr { needle, length });

    let direct_substr = num_tok.clone()
        .then_ignore(just(Token::Sthaanaat).or_not())
        .then_ignore(just(Token::Aarabhya))
        .then(num_tok)
        .then_ignore(just(Token::Varnaanaam))
        .then_ignore(just(Token::Khandah))
        .map(|(start, length)| LocativeStringOp::DirectSubstr { start, length });

    let standalone_instr = search_target
        .then_ignore(ityasya)
        .then_ignore(sthaanam_tok)
        .map(|needle| LocativeStringOp::StandaloneInstr { needle });

    source
        .then(choice((chained_substr_instr, direct_substr, standalone_instr)))
        .map(|(source_expr, op)| match op {
            LocativeStringOp::ChainedSubstrInstr { needle, length } => {
                // Where the text is absent the position is 0, and `substr` from
                // 0 would return the start of the text; NULL says it is absent.
                let position_call = Expression::FunctionCall {
                    name: "अनावृत्तम्".to_string(),
                    args: vec![
                        Expression::FunctionCall {
                            name: "स्थानम्".to_string(),
                            args: vec![source_expr.clone(), needle],
                            window: None,
                            pronoun_token: None,
                        },
                        Expression::Literal(Literal::Number("0".to_string())),
                    ],
                    window: None,
                    pronoun_token: None,
                };
                Expression::FunctionCall {
                    name: "खण्डः".to_string(),
                    args: vec![
                        source_expr,
                        position_call,
                        Expression::Literal(Literal::Number(length)),
                    ],
                    window: None,
                    pronoun_token: None,
                }
            }
            LocativeStringOp::DirectSubstr { start, length } => Expression::FunctionCall {
                name: "खण्डः".to_string(),
                args: vec![
                    source_expr,
                    Expression::Literal(Literal::Number(start)),
                    Expression::Literal(Literal::Number(length)),
                ],
                window: None,
                pronoun_token: None,
            },
            LocativeStringOp::StandaloneInstr { needle } => Expression::FunctionCall {
                name: "स्थानम्".to_string(),
                args: vec![source_expr, needle],
                window: None,
                pronoun_token: None,
            },
        })
}

/// Outside the a-stems the ablative singular is spelled like the genitive
/// (`सङ्ख्यायाः`, `नाम्नः`, `मतेः`, `गुरोः`), and the lexer reads it as the
/// genitive. An a-stem has its own ablative, `-आत्`, so its `-स्य` is no minuend.
fn genitive_is_also_ablative(text: &str, vachana: Vachana) -> bool {
    vachana == Vachana::Eka && !text.ends_with("स्य")
}

/// Returns the arithmetic expression, and the arithmetic forms whose result
/// noun is a genitive, the operand of an aggregate or function: `मूल्यात् १
/// विशोध्य अवशिष्टस्य योगः`.
pub(crate) fn parse_math_expression<'a>(
    recurse_expr: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> (
    impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    Boxed<'a, 'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>>,
) {
    // यत् expr इति - see `grouped`.
    let parenthesized_expr = grouped(recurse_expr.clone());

    let plain_prathama_ident = choice((
        parenthesized_expr.clone(),
        parse_field_access(),
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Prathama { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana, linga: None },
                pratipadika,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            })),
            Token::Identifier(text) => Ok(Expression::Identifier(Identifier {
                text: text.clone(),
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
                pratipadika: text,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            })),
            Token::Number(n) => Ok(Expression::Literal(Literal::Number(n))),
            _ => Err(Rich::custom(span, "expected nominative Identifier, Number or a number")),
        })
    ));
    let prathama_ident = choice((negated(plain_prathama_ident.clone()), plain_prathama_ident.boxed())).boxed();

    // Markers for सम्/गुणनम्-terminated list continuations:
    // - Dual (इत्यनयोः) strictly binds 2 operands (first + 1 `च second`), preventing
    //   a preceding bare SELECT column (`शीर्षकम् च`) from being greedily absorbed
    //   into a following binary math expression (`मूल्यम् च २ च इत्यनयोः गुणनम्`).
    // - Plural (इत्येषाम्/इत्यासाम्) binds 3+ operands.
    let sum_or_product_op = choice((
        just(Token::Yogah).to(BinaryOperator::Add),
        just(Token::Gunanam).to(BinaryOperator::Multiply),
    ));
    let dual_sum_or_product_marker = any()
        .filter(|tok| matches!(tok, Token::Ityayoh(_)))
        .ignore_then(sum_or_product_op.clone());
    let plural_sum_or_product_marker = any()
        .filter(|tok| matches!(tok, Token::Ityesham(_) | Token::Ityetasam(_)))
        .ignore_then(sum_or_product_op);

    // 2. Subtraction: Panchami Dvitiya Nivarya Avashistam
    //
    // Both operand slots also accept a Shashthi-qualified form (Shashthi
    // object + Panchami/Dvitiya-Prathama field, e.g. "पिज्जाप्रकाराणां
    // मूल्यात्" - "from pizza-types' price"), tried first via a dedicated
    // inline alternative rather than by reusing the shared
    // `parse_field_access()`. Requiring Shashthi specifically as the
    // qualifier keeps this alternative unambiguous with the bare-token
    // fallback.
    let qualified_panchami_source = any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
                pratipadika,
                symbol_type: SymbolType::Table,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected genitive qualifier")),
        })
        .then(any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Panchami { text, vachana, pratipadika } => Ok(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Panchami, vachana, linga: None },
                pratipadika,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            }),
            Token::Shashthi { text, vachana, pratipadika } if genitive_is_also_ablative(&text, vachana) => Ok(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Panchami, vachana, linga: None },
                pratipadika,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected ablative field")),
        }))
        .map(|(object, field)| Expression::FieldAccess {
            object: Box::new(Expression::Identifier(object)),
            field,
        });

    let bare_panchami_source = any().try_map(|tok, span: SimpleSpan| {
            match tok {
                Token::Panchami { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
                    text,
                    vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Panchami, vachana, linga: None },
                    pratipadika,
                    symbol_type: SymbolType::Column,
                    span: Some((span.start(), span.end())),
                })),
                Token::Shashthi { text, vachana, pratipadika } if genitive_is_also_ablative(&text, vachana) => Ok(Expression::Identifier(Identifier {
                    text,
                    vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Panchami, vachana, linga: None },
                    pratipadika,
                    symbol_type: SymbolType::Column,
                    span: Some((span.start(), span.end())),
                })),
                Token::Number(n) => Ok(Expression::Literal(Literal::Number(n))),
                _ => Err(Rich::custom(span, "expected ablative Source or Number")),
            }
        });

    let qualified_subtrahend = any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
                pratipadika,
                symbol_type: SymbolType::Table,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected genitive qualifier")),
        })
        .then(any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Dvitiya { text, vachana, pratipadika } => Ok(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Dvitiya, vachana, linga: None },
                pratipadika,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            }),
            Token::Prathama { text, vachana, pratipadika } => Ok(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Dvitiya, vachana, linga: None },
                pratipadika,
                symbol_type: SymbolType::Column,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected accusative or nominative field")),
        }))
        .map(|(object, field)| Expression::FieldAccess {
            object: Box::new(Expression::Identifier(object)),
            field,
        });

    let bare_subtrahend = any().try_map(|tok, span: SimpleSpan| {
            match tok {
                Token::Dvitiya { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
                     text,
                     vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Dvitiya, vachana, linga: None },
                     pratipadika,
                     symbol_type: SymbolType::Column,
                     span: Some((span.start(), span.end())),
                })),
                Token::Prathama { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
                     text,
                     vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Dvitiya, vachana, linga: None },
                     pratipadika,
                     symbol_type: SymbolType::Column,
                     span: Some((span.start(), span.end())),
                })),
                Token::Number(n) => Ok(Expression::Literal(Literal::Number(n))),
                _ => Err(Rich::custom(span, "expected accusative or nominative Operand or Number")),
            }
        });

    // A window-function subtrahend (e.g. `पूर्वपङ्क्तेः सम्पर्कसमयम् यत्र …`) -
    // needed for "current value minus the next/previous row's value"
    // patterns (funnel drop-off, readmission gaps). Gated on the window
    // function words (श्रेणी, पूर्वपङ्क्तेः, चतुर्थांशाङ्कः, …), none of which
    // overlap with qualified_subtrahend/bare_subtrahend's Shashthi/Dvitiya/
    // Prathama/Number gates, so this introduces no choice()-ordering
    // ambiguity with them.
    let window_subtrahend = parse_window_function(recurse_expr.clone());

    // Result nouns in the case of their slot (expert review, grouped
    // operands): a sum or product is an operand by its head's case -
    // ablative minuend, accusative subtrahend or dividend, instrumental
    // divisor (`मूल्यम् च २ च इत्यनयोः योगात् १ विशोध्य अवशिष्टम्`) - with no
    // `यत् … इति` group.
    let operand_list = prathama_ident.clone()
        .then(just(Token::Cha).ignore_then(prathama_ident.clone()).repeated().at_least(1).collect::<Vec<_>>())
        .then_ignore(just(Token::Cha).or_not());
    let list_pronoun = any()
        .filter(|t| matches!(t, Token::Ityayoh(_) | Token::Ityesham(_) | Token::Ityetasam(_)))
        .map(|t| matches!(t, Token::Ityayoh(_)));
    // These forms are tried at every operand position; `ahead` spares them
    // re-parsing a first operand (20_churned_customers went from 0.4 s to 1.4 s
    // without it).
    let sum_product_in = |head: Boxed<'a, 'a, &'a [Token], BinaryOperator, extra::Err<Rich<'a, Token>>>, needed: fn(&Token) -> bool| {
        ahead(needed).ignore_then(operand_list.clone())
            .then(list_pronoun.clone())
            .then(head)
            // The dual binds exactly two operands, so an earlier item of a
            // subtrahend list stays out of it (`प्रकाशनवर्षम् च मूल्यम् च २ च
            // इत्यनयोः योगम् च विशोध्य`); the plural takes three or more.
            .try_map(|(((first, rest), dual), op), span| {
                if dual && rest.len() != 1 {
                    return Err(Rich::custom(span, "`इत्यनयोः` is for two operands"));
                }
                Ok((first, rest, op))
            })
            .validate(|(first, rest, op), e, emitter| {
                if rest.len() == 1 && !matches!(e.slice().iter().rev().nth(1), Some(Token::Ityayoh(_))) {
                    emitter.emit(Rich::custom(e.span(),
                        "two operands take the dual pronoun `इत्यनयोः`; `इत्येषाम्` / `इत्यासाम्` is for three or more"));
                }
                std::iter::once(first).chain(rest)
                    .reduce(|acc, next| Expression::BinaryOp { left: Box::new(acc), op: op.clone(), right: Box::new(next) })
                    .expect("at least two operands")
            })
            .boxed()
    };
    let ablative_head = choice((
        just(Token::YogatAbl).to(BinaryOperator::Add),
        just(Token::GunanatAbl).to(BinaryOperator::Multiply),
    )).boxed();
    let accusative_head = choice((
        just(Token::YogamAcc).to(BinaryOperator::Add),
        just(Token::Gunanam).to(BinaryOperator::Multiply),
    )).boxed();
    let instrumental_head = choice((
        just(Token::YogenaIns).to(BinaryOperator::Add),
        just(Token::GunanenaIns).to(BinaryOperator::Multiply),
    )).boxed();

    // An aggregate as an operand, by its noun's case: `मूल्यस्य महत्तमात्
    // मूल्यस्य लघुतमम् विशोध्य अवशिष्टम्`, `मूल्यस्य योगम् पङ्क्तीनाम् गणनया विभज्य
    // लब्धम्`, optionally over a window.
    let aggregate_window = parse_window_spec(recurse_expr.clone());
    let genitive_operand = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
            text,
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
            pratipadika,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "expected a genitive column")),
    });
    let aggregate_in = |head: Boxed<'a, 'a, &'a [Token], AggFunc, extra::Err<Rich<'a, Token>>>| {
        let distinct = any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Vishishtasya(text) => Ok(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana: Vachana::Eka, linga: None },
                pratipadika: "विशिष्ट".to_string(),
                symbol_type: SymbolType::Variable,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected distinct prefix")),
        });
        let argument = choice((
            just(Token::Panktinam).to(Expression::Wildcard),
            genitive_operand.clone()
                .then(genitive_operand.clone().or_not())
                .map(|(first, second)| match second {
                    Some(field) => Expression::FieldAccess { object: Box::new(Expression::Identifier(first)), field },
                    None => Expression::Identifier(first),
                }),
        ));
        distinct.or_not()
            .then(argument)
            .then(head)
            .then(aggregate_window.clone().or_not())
            .try_map(|(((distinct, argument), func), window), span| {
                if matches!(argument, Expression::Wildcard) && !matches!(func, AggFunc::Count) {
                    return Err(Rich::custom(span, "rows are counted: `पङ्क्तीनाम् गणना`"));
                }
                Ok(match window {
                    Some(win) if distinct.is_none() => Expression::FunctionCall {
                        name: func.sql_name().to_string(),
                        args: vec![argument],
                        window: Some(win),
                        pronoun_token: None,
                    },
                    Some(_) => return Err(Rich::custom(span, "`विशिष्ट-` (DISTINCT) is not said of a windowed aggregate")),
                    None => Expression::Aggregate {
                        expr: Box::new(argument),
                        func,
                        distinct: distinct.is_some(),
                        distinct_token: distinct,
                        pronoun_token: None,
                    },
                })
            })
            .boxed()
    };
    let aggregate_abl = aggregate_in(choice((
        just(Token::YogatAbl).to(AggFunc::Sum),
        just(Token::GananayahGen).to(AggFunc::Count),
        just(Token::MadhyatAbl).to(AggFunc::Avg),
        just(Token::MahattamatAbl).to(AggFunc::Max),
        just(Token::LaghutamatAbl).to(AggFunc::Min),
    )).boxed());
    let aggregate_acc = aggregate_in(choice((
        just(Token::YogamAcc).to(AggFunc::Sum),
        just(Token::GananamAcc).to(AggFunc::Count),
        just(Token::Madhyam).to(AggFunc::Avg),
        just(Token::Mahattam).to(AggFunc::Max),
        just(Token::Laghutamam).to(AggFunc::Min),
    )).boxed());
    let aggregate_ins = aggregate_in(choice((
        just(Token::YogenaIns).to(AggFunc::Sum),
        just(Token::GananayaIns).to(AggFunc::Count),
        just(Token::MadhyenaIns).to(AggFunc::Avg),
        just(Token::MahattamenaIns).to(AggFunc::Max),
        just(Token::LaghutamenaIns).to(AggFunc::Min),
    )).boxed());

    // 2. Subtraction: `<minuend-ablative> <subtrahends-accusative> विशोध्य
    // अवशिष्टम्`. Several subtrahends are one च-list (`मूल्यात् प्रकाशनवर्षम्
    // च १ च विशोध्य अवशिष्टम्`, price - year - 1), and a chain goes on from
    // the remainder in the ablative (`… विशोध्य अवशिष्टात् १ विशोध्य अवशिष्टम्`).
    // 3. Division: `<dividend-accusative> <divisor-instrumental> विभज्य
    // लब्धम्`, chained by another divisor (`मूल्यम् २ विभज्य लब्धम् ३ विभज्य
    // लब्धम्`). A subtraction may be a dividend or divisor and a division a
    // subtrahend; a division as minuend would make the two recurse at one
    // position, so it is not offered.
    let dividend_tok = any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Dvitiya { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
                 text,
                 vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Dvitiya, vachana, linga: None },
                 pratipadika,
                 symbol_type: SymbolType::Column,
                 span: Some((span.start(), span.end())),
            })),
            Token::Prathama { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
                 text,
                 vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana, linga: None },
                 pratipadika,
                 symbol_type: SymbolType::Column,
                 span: Some((span.start(), span.end())),
            })),
            Token::Number(n) => Ok(Expression::Literal(Literal::Number(n))),
            _ => Err(Rich::custom(span, "expected accusative or nominative Dividend or Number")),
        }).boxed();
    let dividend_tok = choice((negated(dividend_tok.clone()), dividend_tok)).boxed();
    let divisor_tok = any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Tritiya { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
                 text,
                 vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Tritiya, vachana, linga: None },
                 pratipadika,
                 symbol_type: SymbolType::Column,
                 span: Some((span.start(), span.end())),
            })),
            Token::Number(n) => Ok(Expression::Literal(Literal::Number(n))),
            _ => Err(Rich::custom(span, "expected instrumental Divisor or Number")),
        }).boxed();
    let divisor_tok = choice((negated(divisor_tok.clone()), divisor_tok)).boxed();
    // Subtraction and division are built with `recursive`, whose handle does
    // not own the parser: two `Recursive::declare`s holding each other were a
    // reference cycle, and the grammar, built for every parse, leaked about
    // 7 MB a compile (the playground ran out of memory at its 545th).
    // `division_over` builds division over a given subtraction parser: the
    // handle inside the recursion, the finished parser outside it.
    // `head` is the quotient's case: `लब्धम्` as a value or accusative,
    // `लब्धात्` as a minuend. A minuend's division takes no subtraction as
    // its dividend: that subtraction's minuend would be this division again,
    // at the same position.
    let division_over = |subtraction_core: Boxed<'a, 'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>>, head: Token, subtraction_dividend: bool| {
        let subtraction = ahead(|t| matches!(t, Token::Nivarya))
            .ignore_then(subtraction_core.clone().then_ignore(just(Token::Avashistam)));
        let dividend_sub = if subtraction_dividend {
            subtraction.boxed()
        } else {
            any().filter(|_| false).map(|_| Expression::Wildcard).boxed()
        };
        // A quotient as divisor, by its own instrumental: `मूल्यम् भण्डारम् २
        // विभज्य लब्धेन विभज्य लब्धम्`, price / (quantity / 2).
        let core = recursive(|inner| {
            let dividend = choice((
                aggregate_acc.clone(),
                dividend_sub.clone(),
                sum_product_in(accusative_head.clone(), |t| matches!(t, Token::YogamAcc | Token::Gunanam)),
                dividend_tok.clone(),
            ));
            let divisor = choice((
                ahead(|t| matches!(t, Token::LabdhenaIns)).ignore_then(inner.then_ignore(just(Token::LabdhenaIns))).boxed(),
                aggregate_ins.clone(),
                subtraction_core.clone().then_ignore(just(Token::AvashistenaIns)).boxed(),
                sum_product_in(instrumental_head.clone(), |t| matches!(t, Token::YogenaIns | Token::GunanenaIns)),
                divisor_tok.clone(),
            )).boxed();
            dividend
                .then(divisor.clone())
                .then_ignore(just(Token::Vibhajya))
                .then(
                    just(Token::Labdham)
                        .ignore_then(divisor)
                        .then_ignore(just(Token::Vibhajya))
                        .repeated()
                        .collect::<Vec<_>>(),
                )
                .map(|((dividend, first), rest)| {
                    std::iter::once(first).chain(rest).fold(dividend, |acc, d| Expression::BinaryOp {
                        left: Box::new(acc),
                        op: BinaryOperator::Divide,
                        right: Box::new(d),
                    })
                })
        });
        ahead(|t| matches!(t, Token::Vibhajya))
            .ignore_then(core.then_ignore(just(head)))
            .boxed()
    };

    let subtraction_core = recursive(|sub_core| {
        let sub_core = sub_core.boxed();
        // A remainder as subtrahend (`a - (b - c)`) is grouped, its case
        // carried by the accusative pronoun `इतीदम्` (expert review: two
        // ablatives in a row read poorly), as is anything else with no
        // accusative of its own, such as a function or a subquery. What has
        // one takes it, and a group around it is redundant.
        let grouped_subtrahend = choice((
            just(Token::Yat).rewind().ignore_then(subquery_ahead()).ignore_then(subquery_parser.clone()).try_map(|e, span| match e {
                Expression::Subquery(ref d) if d.verb_token.is_some() => Ok(e),
                _ => Err(Rich::custom(span, "not a verb-bearing subquery")),
            }).boxed(),
            parenthesized_expr.clone().boxed(),
        ))
            .then_ignore(just(Token::Itidam))
            .validate(|expr, e, emitter| {
                let direct = match &expr {
                    Expression::Identifier(_) | Expression::FieldAccess { .. } | Expression::Literal(_) =>
                        Some("a column or number is a subtrahend as it stands, in the accusative (`मूल्यात् प्रकाशनवर्षम् विशोध्य`)"),
                    Expression::BinaryOp { op: BinaryOperator::Add, .. } => Some("a sum is a subtrahend by its accusative, `योगम्`"),
                    Expression::BinaryOp { op: BinaryOperator::Multiply, .. } => Some("a product is a subtrahend by its accusative, `गुणनम्`"),
                    Expression::BinaryOp { op: BinaryOperator::Divide, .. } => Some("a quotient is a subtrahend by its accusative, `लब्धम्`"),
                    Expression::Aggregate { .. } => Some("an aggregate is a subtrahend by its accusative (`मूल्यस्य योगम्`, `पङ्क्तीनाम् गणनाम्`)"),
                    Expression::FunctionCall { window: Some(_), .. } => Some("a window function is a subtrahend as it stands (`मूल्यस्य योगम् यत्र …`, `पूर्वपङ्क्तेः मूल्यम् यत्र …`)"),
                    _ => None,
                };
                if let Some(direct) = direct {
                    emitter.emit(Rich::custom(e.span(), format!("{direct}, so this `यत् … इति इतीदम्` group is redundant")));
                }
                expr
            });
        let subtrahend = choice((
            window_subtrahend,
            grouped_subtrahend.boxed(),
            division_over(sub_core.clone(), Token::Labdham, true),
            sum_product_in(accusative_head.clone(), |t| matches!(t, Token::YogamAcc | Token::Gunanam)),
            aggregate_acc.clone(),
            negated(choice((qualified_subtrahend.clone(), bare_subtrahend.clone()))),
            qualified_subtrahend,
            bare_subtrahend,
        )).boxed();
        let subtrahends = subtrahend.clone()
            .then(just(Token::Cha).ignore_then(subtrahend).repeated().collect::<Vec<_>>())
            .then(just(Token::Cha).or_not())
            .validate(|((first, rest), trailing), e, emitter| {
                let slice: &[Token] = e.slice();
                if !rest.is_empty() && trailing.is_none() {
                    let last = slice.last().map(|t| t.to_string()).unwrap_or_default();
                    emitter.emit(Rich::custom(e.span(), super::common::last_conjunct_error("च", &last)));
                }
                if rest.is_empty() && trailing.is_some() {
                    emitter.emit(Rich::custom(e.span(), super::common::lone_conjunct_error("च")));
                }
                std::iter::once(first).chain(rest).collect::<Vec<_>>()
            })
            .boxed();
        choice((
            aggregate_abl.clone(),
            negated(choice((qualified_panchami_source.clone(), bare_panchami_source.clone()))),
            qualified_panchami_source,
            bare_panchami_source,
            sum_product_in(ablative_head.clone(), |t| matches!(t, Token::YogatAbl | Token::GunanatAbl)),
            // A quotient as minuend: `मूल्यम् २ विभज्य लब्धात् ५ विशोध्य …`.
            division_over(sub_core.clone(), Token::LabdhatAbl, false),
        ))
            .then(subtrahends.clone())
            .then_ignore(just(Token::Nivarya))
            .then(
                just(Token::AvashistatAbl)
                    .ignore_then(subtrahends)
                    .then_ignore(just(Token::Nivarya))
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|((minuend, first), steps)| {
                std::iter::once(first).chain(steps).flatten().fold(minuend, |acc, s| Expression::BinaryOp {
                    left: Box::new(acc),
                    op: BinaryOperator::Subtract,
                    right: Box::new(s),
                })
            })
    }).boxed();
    let subtraction = ahead(|t| matches!(t, Token::Nivarya))
        .ignore_then(subtraction_core.clone().then_ignore(just(Token::Avashistam)))
        .boxed();
    let division = division_over(subtraction_core.clone(), Token::Labdham, true);
    let genitive_math = choice((
        ahead(|t| matches!(t, Token::AvashistasyaGen))
            .ignore_then(subtraction_core.clone().then_ignore(just(Token::AvashistasyaGen)))
            .boxed(),
        ahead(|t| matches!(t, Token::LabdhasyaGen))
            .ignore_then(division_over(subtraction_core, Token::LabdhasyaGen, true))
            .boxed(),
        sum_product_in(
            choice((
                just(Token::YogasyaGen).to(BinaryOperator::Add),
                just(Token::GunanasyaGen).to(BinaryOperator::Multiply),
            )).boxed(),
            |t| matches!(t, Token::YogasyaGen | Token::GunanasyaGen),
        ),
    )).boxed();
        
    // Unified "prathama_ident-led" expression: parses the shared leading
    // operand exactly ONCE via `prathama_ident`, then branches on whatever
    // follows (a च-separated list with a योगः/गुणनम् marker, or a प्रदाय/आदाय
    // Pure UDF invocation). Parsing `prathama_ident` once and branching on
    // a token-level lookahead avoids redundant re-derivation of nested
    // sub-expressions.
    enum MathTail {
        List(Vec<Expression>, Option<BinaryOperator>),
        PradayaPositional(Vec<Expression>, String),
        PradayaNamed(Vec<Expression>),
    }

    let dvitiya_or_str_arg = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Dvitiya { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
            text,
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana, linga: None },
            pratipadika,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        })),
        Token::StringLit(s) => Ok(Expression::Literal(Literal::String(s))),
        _ => Err(Rich::custom(span, "expected accusative identifier or string literal")),
    });

    let pradaya_arg = choice((
        prathama_ident.clone(),
        dvitiya_or_str_arg.clone(),
    ));

    let udf_ident_tok = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Prathama { text, vachana, pratipadika } => Ok(Identifier {
            text,
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana, linga: None },
            pratipadika,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        }),
        Token::Identifier(text) => Ok(Identifier {
            text: text.clone(),
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
            pratipadika: text,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "expected UDF or parameter identifier")),
    });

    // A named argument is the quotation `<param> इत्यस्य मूल्यम् <value> इति`
    // ("the value of <param> is <value>"), so it always closes with इति.
    let named_arg_iti = just(Token::Iti);

    let named_arg_val_tail = any()
        .filter(|tok| matches!(tok, Token::Ityasya(_) | Token::Ityasyah(_)))
        .ignore_then(any().filter(|tok| match tok {
            Token::Prathama { text, .. } | Token::Identifier(text) => text == "मूल्यम्",
            _ => false,
        }))
        .ignore_then(pradaya_arg.clone())
        .then_ignore(named_arg_iti);

    let subsequent_named_arg = just(Token::Cha)
        .ignore_then(udf_ident_tok.clone())
        .then(named_arg_val_tail.clone())
        .map(|(param, val)| Expression::NamedArg {
            param,
            value: Box::new(val),
        });

    let udf_ident_tok_for_list = udf_ident_tok.clone();

    let single_or_named_pradaya_tail = just(Token::Pradaya)
        .ignore_then(udf_ident_tok.clone())
        .then(
            named_arg_val_tail
                .then(subsequent_named_arg.repeated().collect::<Vec<_>>())
                .or_not()
        )
        .map(|(ident2, named_opt)| match named_opt {
            Some((first_val, mut rest_named)) => {
                let mut named = vec![Expression::NamedArg {
                    param: ident2,
                    value: Box::new(first_val),
                }];
                named.append(&mut rest_named);
                MathTail::PradayaNamed(named)
            }
            None => MathTail::PradayaPositional(vec![], ident2.text),
        });

    let dvitiya_or_str_pradaya = dvitiya_or_str_arg
        .then(
            any().filter(|t| matches!(t, Token::Cha))
                .ignore_then(pradaya_arg.clone())
                .repeated()
                .collect::<Vec<_>>()
        )
        .then_ignore(any().filter(|t| matches!(t, Token::Cha)).or_not())
        .then_ignore(just(Token::Pradaya))
        .then(udf_ident_tok)
        .map(|((first, rest), func_id)| {
            let mut args = vec![first];
            args.extend(rest);
            Expression::FunctionCall {
                name: func_id.text,
                args,
                window: None,
                pronoun_token: None,
            }
        });

    // Replicates prathama_list's old `separated_by(Cha).allow_trailing()`
    // semantics by hand (item (Cha item)* (Cha)? marker?), so "A च B च
    // इत्यनयोः गुणनम्"-shaped lists (each item, including the last, followed
    // by च) still parse identically to before.
    //
    // The "trailing Cha" must only be absorbed when at least one "Cha item"
    // repetition already matched (a genuine list) - not unconditionally.
    // A bare prathama_ident with *zero* list items (e.g. a parenthesized
    // value like a window function or aggregate) followed by an unrelated
    // Cha - a select-item separator, an AND/OR fold connector, and so on -
    // must leave that Cha for whatever encloses this expression to consume;
    // greedily eating it here as a "trailing comma with no list" would
    // silently swallow the very next select-list item.
    // If a set-operation suffix (इत्यनयोः/इत्येषाम् + सङ्गमः/सम्मिलनम्/वर्जनम्)
    // immediately follows, this isn't a bare math-expression value at all -
    // it's the tail of a set operation whose branches merely happen to look
    // like a parenthesized list from prathama_ident's point of view (each
    // branch is itself a parenthesized DQL subquery, which prathama_ident's
    // own parenthesized_expr alternative happily recognizes as an atom).
    // Without this check, "((subquery1) च (subquery2)) इत्यनयोः सङ्गमः"
    // gets fully consumed as "two subqueries folded with AND" right here,
    // and suffix_eligible's subquery_check (which would otherwise recognize
    // the whole thing, इत्यनयोः सङ्गमः included, as one Statement::SetOp)
    // never gets a chance to run.
    let no_set_op_suffix_follows = any().filter(|t| matches!(t, Token::Ityayoh(_) | Token::Ityesham(_)))
        .then(choice((
            just(Token::Sangamah).ignored(),
            just(Token::SarvaSangamah).ignored(),
            just(Token::Sammilanam).ignored(),
            just(Token::Varjanam).ignored(),
        )))
        .not()
        .rewind();

    // `च <item>` repeated is parsed once, then the word after it decides what
    // the list was: arguments of a `प्रदाय` call, or the operands of a sum or
    // product. As separate alternatives each of these re-parsed every item, and
    // an item may be a whole group. The dual binds exactly one more operand, so
    // that `शीर्षकम् च मूल्यम् च २ च इत्यनयोः गुणनम्` leaves `शीर्षकम्` a select item.
    enum ListEnd {
        Pradaya(Identifier),
        Dual(BinaryOperator),
        Plural(BinaryOperator),
    }
    let pradaya_item = choice((
        prathama_ident.clone().map(|e| (e, true)),
        dvitiya_or_str_arg.clone().map(|e| (e, false)),
    ));
    let list_with_end = just(Token::Cha)
        .ignore_then(pradaya_item)
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then_ignore(just(Token::Cha).or_not())
        .then(choice((
            just(Token::Pradaya).ignore_then(udf_ident_tok_for_list).map(ListEnd::Pradaya),
            dual_sum_or_product_marker.map(ListEnd::Dual),
            plural_sum_or_product_marker.map(ListEnd::Plural),
        )))
        .try_map(|(items, end), span| {
            let operands = items.iter().all(|(_, operand)| *operand);
            let values = items.into_iter().map(|(e, _)| e).collect::<Vec<_>>();
            match end {
                ListEnd::Pradaya(func_id) => Ok((MathTail::PradayaPositional(values, func_id.text), false)),
                ListEnd::Dual(op) if operands && values.len() == 1 => Ok((MathTail::List(values, Some(op)), false)),
                ListEnd::Plural(op) if operands => {
                    let misnumbered = values.len() < 2;
                    Ok((MathTail::List(values, Some(op)), misnumbered))
                }
                _ => Err(Rich::custom(span, "expected the operands of a sum or product")),
            }
        })
        // Two operands are a dual (`मूल्यम् च ५० च इत्यनयोः योगः`); the
        // plural pronoun is for three or more.
        .validate(|(tail, misnumbered), e, emitter| {
            if misnumbered {
                emitter.emit(Rich::custom(e.span(),
                    "two operands take the dual pronoun `इत्यनयोः`; `इत्येषाम्` / `इत्यासाम्` is for three or more"));
            }
            tail
        });

    let list_tail = choice((
        list_with_end,
        single_or_named_pradaya_tail,
        no_set_op_suffix_follows
            .map(|_| MathTail::List(vec![], None)),
    ));

    let prathama_based = prathama_ident.clone()
        .then(list_tail)
        .try_map(|(first, tail), span| match tail {
            MathTail::PradayaPositional(rest, func_name) => {
                let mut args = vec![first];
                args.extend(rest);
                Ok(Expression::FunctionCall {
                    name: func_name,
                    args,
                    window: None,
                    pronoun_token: None,
                })
            }
            MathTail::PradayaNamed(named_args) => {
                let func_name = match first {
                    Expression::Identifier(id) => id.text,
                    _ => return Err(Rich::custom(span, "expected UDF name before प्रदाय in named-argument call")),
                };
                Ok(Expression::FunctionCall {
                    name: func_name,
                    args: named_args,
                    window: None,
                    pronoun_token: None,
                })
            }
            MathTail::List(rest, marker) => match (rest.len(), marker) {
                (0, None) => Ok(first),
                (n, Some(op)) if n >= 1 => {
                    let mut iter = std::iter::once(first).chain(rest.into_iter());
                    let acc0 = iter.next().unwrap();
                    Ok(iter.fold(acc0, |acc, next| Expression::BinaryOp {
                        left: Box::new(acc),
                        op: op.clone(),
                        right: Box::new(next),
                    }))
                }
                _ => Err(Rich::custom(span, "expected 2+ items with a सम्/गुणनम् marker, or exactly 1 item alone")),
            },
        });

    let math = choice((
        // subtraction/division need first shot at their specific multi-token
        // markers (Panchami...Nivarya Avashistam / Vibhajya Labdham) -
        // prathama_based's leading operand is ambiguous with subtraction's
        // Panchami-cased source (parse_field_access deliberately treats
        // Panchami as ambiguous with Shashthi genitive - see its own doc
        // comment), so if prathama_based were tried first it would greedily
        // accept a bare "object field" pass-through and never give
        // subtraction/division a chance to recognize the fuller pattern.
        // Division first: a subtraction is also a dividend, and division
        // falls back here when no divisor follows it.
        division,
        subtraction,
        dvitiya_or_str_pradaya,
        prathama_based,
    ));
    (math, genitive_math)
}

/// Helper for parsing expression atoms (used in both logical and non-logical contexts)
/// Parse Field Access: (Object's Field) e.g. "User's Name"
/// Handles explicit Shashthi (Genitive) and ambiguous Panchami/Shashthi forms (like R-stems)

pub(crate) fn parse_expr_atom<'a>(
    recurse_expr: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> Boxed<'a, 'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> {
    // If a set-operation suffix (इत्यनयोः/इत्येषाम् + सङ्गमः/सम्मिलनम्/वर्जनम्)
    // immediately follows, a bare parenthesized_expr match here is wrong:
    // "((subquery1) च (subquery2)) इत्यनयोः सङ्गमः" would otherwise get
    // fully consumed as "two subqueries folded with AND" by recurse_expr's
    // own foldl (each branch is itself a parenthesized DQL subquery, which
    // recurse_expr's own atom machinery happily recognizes), and the
    // subquery_check alternative below - which would otherwise recognize
    // the whole thing, इत्यनयोः सङ्गमः included, as one Statement::SetOp -
    // never gets a chance to run. See the identical guard in
    // parse_math_expression's list_tail.
    let no_set_op_suffix_follows = any().filter(|t| matches!(t, Token::Ityayoh(_) | Token::Ityesham(_)))
        .then(choice((
            just(Token::Sangamah).ignored(),
            just(Token::SarvaSangamah).ignored(),
            just(Token::Sammilanam).ignored(),
            just(Token::Varjanam).ignored(),
        )))
        .not()
        .rewind();

    // General grouping: यत् expr इति.
    let parenthesized_expr = grouped(recurse_expr.clone())
        .then_ignore(no_set_op_suffix_follows);

    // The CAST form before the paṇḍit-checklist review, `<operand> इत्यस्य
    // <type> इति परिणामः`, still parsed so it can name the form that replaced it.
    let cast_expr = ahead(|t| matches!(t, Token::Parinamah))
        .ignore_then(choice((parse_field_access(), parse_atom(), parenthesized_expr.clone())))
        .then_ignore(any().filter(|t| matches!(t, Token::Ityasya(_))))
        .then(parse_type())
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Parinamah))
        .validate(|(expr, data_type), e, emitter| {
            emitter.emit(Rich::custom(e.span(),
                "a cast is now written with the operand in the genitive and the type as its form: `मूल्यस्य पूर्णाङ्करूपेण परिणामः` (`वाक्यरूपेण`, `वास्तविकरूपेण`, `दिनाङ्करूपेण`, `द्वैतरूपेण`)"));
            Expression::Cast { expr: Box::new(expr), data_type }
        });

    // --- Math Expressions ---
    let (math_expr, genitive_math) = parse_math_expression(recurse_expr.clone(), subquery_parser.clone());

    let agg_func = choice((
        just(Token::Yogah).to(AggFunc::Sum),
        just(Token::Ganana).to(AggFunc::Count),
        just(Token::Madhyam).to(AggFunc::Avg),
        just(Token::Mahattam).to(AggFunc::Max),
        just(Token::Laghutamam).to(AggFunc::Min),
    ));

    let function_name = choice((
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Identifier(name) => Ok(name),
            Token::Prathama { text, .. } => Ok(text),
            _ => Err(Rich::custom(span, "expected function name")),
        }),
        just(Token::Mahavarnarupam).to("महावर्णरूपम्".to_string()),
        just(Token::Laghuvarnarupam).to("लघुवर्णरूपम्".to_string()),
        just(Token::Samyojanam).to("संयोजनम्".to_string()),
        just(Token::Aayamah).to("दैर्घ्यम्".to_string()),
        just(Token::Vikalpah).to("विकल्पः".to_string()),
        just(Token::Avashistam).to("अवशिष्टम्".to_string()),
    ));

    let distinct_prefix = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Vishishtasya(s) => Ok(Identifier {
            text: s.clone(),
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana: Vachana::Eka, linga: None },
            pratipadika: "विशिष्ट".to_string(),
            symbol_type: SymbolType::Variable,
            span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "expected distinct prefix")),
    }).or_not();

    let genitive_pronoun_parser_mandatory = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Ityasya(ref s) | Token::Ityasyah(ref s) | Token::Ityayoh(ref s) | Token::Ityesham(ref s) | Token::Ityetasam(ref s) => {
            let (vachana, linga) = match &tok {
                Token::Ityasya(_) => (Vachana::Eka, Some(crate::types::Linga::Masculine)),
                Token::Ityasyah(_) => (Vachana::Eka, Some(crate::types::Linga::Feminine)),
                Token::Ityayoh(_) => (Vachana::Dvi, None),
                Token::Ityesham(_) => (Vachana::Bahu, Some(crate::types::Linga::Masculine)),
                Token::Ityetasam(_) => (Vachana::Bahu, Some(crate::types::Linga::Feminine)),
                _ => (Vachana::Eka, None),
            };
            Ok(Identifier {
                text: s.clone(),
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga },
                pratipadika: "इति".to_string(),
                symbol_type: SymbolType::Variable,
                span: Some((span.start(), span.end())),
            })
        }
        _ => Err(Rich::custom(span, "expected genitive pronoun")),
    });

    let genitive_pronoun_parser_optional = genitive_pronoun_parser_mandatory.clone().or_not();

    // A function of several operands whose first is an aggregate or arithmetic
    // result, in the genitive: `मूल्यस्य माध्यस्य २ इत्यनेन सन्निकटनम्` is
    // ROUND(AVG(price), 2).
    let genitive_column = any().try_map(|tok, span: SimpleSpan| match tok {
        Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
            text,
            vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
            pratipadika,
            symbol_type: SymbolType::Column,
            span: Some((span.start(), span.end())),
        }),
        _ => Err(Rich::custom(span, "expected a genitive column")),
    });
    let genitive_aggregate = genitive_column.clone()
        .then(genitive_column.or_not())
        .then(choice((
            just(Token::YogasyaGen).to(AggFunc::Sum),
            just(Token::GananayahGen).to(AggFunc::Count),
            just(Token::MadhyasyaGen).to(AggFunc::Avg),
            just(Token::MahattamasyaGen).to(AggFunc::Max),
            just(Token::LaghutamasyaGen).to(AggFunc::Min),
        )))
        .map(|((first, second), func)| Expression::Aggregate {
            expr: Box::new(match second {
                Some(field) => Expression::FieldAccess { object: Box::new(Expression::Identifier(first)), field },
                None => Expression::Identifier(first),
            }),
            func,
            distinct: false,
            distinct_token: None,
            pronoun_token: None,
        });
    let genitive_result = genitive_math.clone();
    let function_of_a_result = scalar_function_after(move |column| {
        choice((genitive_aggregate, genitive_result, column))
    });

    let suffix_eligible = choice((
        // A `यत् … आह्रियताम् इति` that carries its own verb is always a
        // subquery - no expression ends in a verb - so it is tried first,
        // before math_expr and the generic `यत् <expr> इति` group, which
        // otherwise read a subquery whose filter starts with a qualified
        // column (`यत् आपणेभ्यः आपणस्य नाम … इति नियमेन …`) as a group ending
        // at its own इति.
        just(Token::Yat).rewind().ignore_then(subquery_ahead()).ignore_then(subquery_parser.clone()).try_map(|e, span| match e {
            Expression::Subquery(ref d) if d.verb_token.is_some() => Ok(e),
            _ => Err(Rich::custom(span, "not a verb-bearing subquery")),
        }),
        cast_expr,
        parse_locative_substr(),
        function_of_a_result,
        // math_expr must be tried before this function's own parenthesized_expr:
        // math_expr's `summation` now absorbs the "just a single wrapped atom"
        // case itself (prathama_list relaxed to at_least(1) with an optional
        // sum/product marker - see prathama_list's doc comment in
        // parse_math_expression), so it produces the *same* result as
        // parenthesized_expr would for a bare wrapped atom, but *also*
        // correctly recognizes a longer "(A) च B च इत्यनयोः गुणनम्"-shaped
        // continuation starting with a parenthesized first operand.
        math_expr,
        parse_field_access(),
        // Try "just a parenthesized expression" before "this is a nested
        // subquery" - a bracketed expression (e.g. an aggregate+window-spec
        // group) is structurally valid as both, since a genitive-case word
        // like a table's property name can also look like a (mistaken)
        // FROM-source to the subquery grammar. Preferring the expression
        // interpretation avoids that ambiguity recursing into itself.
        parenthesized_expr,
        // Cheap lookahead guard: only invoke the (self-referential, expensive)
        // subquery parser when a यत् is actually present, instead of paying
        // its full construction/dispatch cost on every failed attempt.
        just(Token::Yat).rewind().ignore_then(subquery_parser.clone()),
        parse_atom(),
        just(Token::Sarvam).to(Expression::Wildcard),
    ));

    let suffix_eligible_atom = with_optional_predicate(suffix_eligible, recurse_expr.clone());

    // Suffix modifiers with mandatory pronoun (any expression)
    // CAST (pcf.md item 4): the operand in the genitive and the type as
    // "in <type> form", `मूल्यस्य पूर्णाङ्करूपेण परिणामः` ("the transformation of
    // the value into integer form"). Carried through the suffix machinery as
    // a function name, then built as Expression::Cast.
    let cast_target = choice((
        just(Token::PurnankaRupena).to(format!("{}integer", CAST_MARK)),
        just(Token::VakyaRupena).to(format!("{}text", CAST_MARK)),
        just(Token::VastavikaRupena).to(format!("{}decimal", CAST_MARK)),
        just(Token::DinankaRupena).to(format!("{}date", CAST_MARK)),
        just(Token::DvaitaRupena).to(format!("{}boolean", CAST_MARK)),
    ))
    .then_ignore(just(Token::Parinamah));

    let suffix_modifiers_mandatory = choice((
        // Aggregate: Ityasya Yogah
        genitive_pronoun_parser_mandatory.clone()
            .then(agg_func.clone())
            .then(
                choice((
                    parse_window_spec(recurse_expr.clone()).map(Some),
                    just(Token::Yatra).not().rewind().to(None),
                ))
            )
            .map(|((pronoun, func), window)| (Some((func, window, Some(pronoun.clone()))), None::<String>, Some(pronoun))),
        
        // CAST: `"५" इत्यस्य पूर्णाङ्करूपेण परिणामः` (a literal or alias by its pronoun).
        genitive_pronoun_parser_mandatory.clone()
            .then(cast_target.clone())
            .map(|(pronoun, f)| (None, Some(f), Some(pronoun))),

        // Date: Ityasya Masah
        genitive_pronoun_parser_mandatory.clone()
            .then(
                choice((
                    just(Token::Varsham).to("YEAR".to_string()),
                    just(Token::Masah).to("MONTH".to_string()),
                    just(Token::Dinam).to("DAY".to_string()),
                    just(Token::Hora).to("HOUR".to_string()),
                    just(Token::Nimesah).to("MINUTE".to_string()),
                    just(Token::Vipalam).to("SECOND".to_string()),
                    just(Token::Vaarah).to("DAYNAME".to_string()),
                ))
            )
            .map(|(pronoun, f)| (None, Some(f), Some(pronoun))),

        // General Unary Function Call: Ityasya function_name
        genitive_pronoun_parser_mandatory.clone()
            .then(function_name.clone())
            .map(|(pronoun, f)| (None, Some(f), Some(pronoun))),
    ));

    // Suffix modifiers with optional pronoun (only for Shashthi expressions)
    let suffix_modifiers_optional = choice((
        // Aggregate: [Ityasya] Yogah
        genitive_pronoun_parser_optional.clone()
            .then(agg_func.clone())
            .then(
                choice((
                    parse_window_spec(recurse_expr.clone()).map(Some),
                    just(Token::Yatra).not().rewind().to(None),
                ))
            )
            .map(|((pronoun_opt, func), window)| (Some((func, window, pronoun_opt.clone())), None::<String>, pronoun_opt)),
        
        // CAST: `मूल्यस्य पूर्णाङ्करूपेण परिणामः`.
        genitive_pronoun_parser_optional.clone()
            .then(cast_target.clone())
            .map(|(pronoun_opt, f)| (None, Some(f), pronoun_opt)),

        // Date: [Ityasya] Masah
        genitive_pronoun_parser_optional.clone()
            .then(
                choice((
                    just(Token::Varsham).to("YEAR".to_string()),
                    just(Token::Masah).to("MONTH".to_string()),
                    just(Token::Dinam).to("DAY".to_string()),
                    just(Token::Hora).to("HOUR".to_string()),
                    just(Token::Nimesah).to("MINUTE".to_string()),
                    just(Token::Vipalam).to("SECOND".to_string()),
                    just(Token::Vaarah).to("DAYNAME".to_string()),
                ))
            )
            .map(|(pronoun_opt, f)| (None, Some(f), pronoun_opt)),

        // General Unary Function Call: [Ityasya] function_name
        genitive_pronoun_parser_optional.clone()
            .then(function_name)
            .map(|(pronoun_opt, f)| (None, Some(f), pronoun_opt)),
    ));


    let shashthi_obj_parser = table_qualifier();

    let shashthi_expr = choice((
        // An arithmetic result in the genitive; being a genitive, it takes no
        // pronoun: `मूल्यात् १ विशोध्य अवशिष्टस्य योगः`.
        genitive_math.then_ignore(genitive_pronoun_parser_mandatory.clone().not().rewind()),
        // Field access ending in Shashthi
        shashthi_obj_parser.then(
            any().try_map(|tok, span: SimpleSpan| match tok {
                Token::Shashthi { text, vachana, pratipadika } => Ok(Identifier {
                    text,
                    vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
                    pratipadika,
                    symbol_type: SymbolType::Column,
                    span: Some((span.start(), span.end())),
                }),
                _ => Err(Rich::custom(span, "expected genitive field")),
            })
        ).map(|(obj, fld)| Expression::FieldAccess {
            object: Box::new(Expression::Identifier(obj)),
            field: fld,
        }),
        // Simple Shashthi identifier (not followed by Prathama or Identifier, which would form a field access)
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::Shashthi { text, vachana, pratipadika } => Ok(Expression::Identifier(Identifier {
                text,
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Shashthi, vachana, linga: None },
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
        )
    ));

    let shashthi_suffix_branch_raw = distinct_prefix.clone()
        .then(shashthi_expr)
        .then(suffix_modifiers_optional)
        .map(|((distinct_id, expr), suffix)| (distinct_id, expr, Some(suffix)));
    let shashthi_suffix_branch = shashthi_suffix_branch_raw;

    // Parses `suffix_eligible_atom` once, then checks for an optional
    // trailing suffix marker (aggregate, date part, or window spec).
    let general_or_plain_branch = distinct_prefix.clone()
        .then(suffix_eligible_atom)
        // A non-genitive operand of an aggregate, date part or function takes
        // the genitive pronoun (`दिनाङ्कः इत्यस्य वर्षम्`); a genitive one is the
        // shashthi branch above. A bare nominative (`दिनानि माध्यम्`) is not.
        .then(suffix_modifiers_mandatory.or_not())
        .map(|((distinct_id, expr), suffix)| (distinct_id, expr, suffix));

    let suffix_part = choice((
        shashthi_suffix_branch,
        general_or_plain_branch,
    ))
    // `विशिष्ट-` (DISTINCT) is said of an aggregate's values (`विशिष्टायाः
    // भाषायाः गणना`); anywhere else it used to be dropped from the SQL, so
    // it is reported there (without failing the parse, so nothing masks it).
    .validate(|(distinct_id, expr, suffix_opt), e, emitter| {
        let mut final_expr = expr;
        let mut compound_distinct = false;
        match &mut final_expr {
            Expression::Identifier(ident) => {
                if let Some((stripped, is_distinct)) = strip_distinct_prefix(&ident.text) {
                    ident.text = stripped;
                    if let Some((p_stripped, _)) = strip_distinct_prefix(&ident.pratipadika) {
                        ident.pratipadika = p_stripped;
                    } else {
                        ident.pratipadika = ident.text.clone();
                    }
                    compound_distinct = is_distinct;
                }
            }
            Expression::FieldAccess { object, field: _ } => {
                if let Expression::Identifier(obj_id) = object.as_mut() {
                    if let Some((stripped, is_distinct)) = strip_distinct_prefix(&obj_id.text) {
                        obj_id.text = stripped;
                        if let Some((p_stripped, _)) = strip_distinct_prefix(&obj_id.pratipadika) {
                            obj_id.pratipadika = p_stripped;
                        } else {
                            obj_id.pratipadika = obj_id.text.clone();
                        }
                        compound_distinct = is_distinct;
                    }
                }
            }
            _ => {}
        }
        let final_distinct_id = if let Some(id) = distinct_id {
            Some(id)
        } else if compound_distinct {
            Some(Identifier {
                text: "विशिष्ट".to_string(),
                vibhakti_form: VibhaktiForm { vibhakti: Vibhakti::Prathama, vachana: Vachana::Eka, linga: None },
                pratipadika: "विशिष्ट".to_string(),
                symbol_type: SymbolType::Variable,
                span: None,
            })
        } else {
            None
        };

        let span = e.span();
        let misplaced = |what: &str| Rich::custom(span, format!(
            "`विशिष्ट-` (DISTINCT) is said of an aggregate's values (`विशिष्टायाः भाषायाः गणना`), not of {}", what
        ));
        match suffix_opt {
            Some((Some((func, window, pronoun_id)), _, _)) => {
                if let Some(win) = window {
                    if final_distinct_id.is_some() {
                        emitter.emit(misplaced("a windowed aggregate, which neither SQLite nor PostgreSQL computes over distinct values"));
                    }
                    // Same aggregate as the un-windowed form; this match used
                    // to send COUNT (and anything unlisted) to SUM.
                    let func_name = func.sql_name();
                    Expression::FunctionCall {
                        name: func_name.to_string(),
                        args: vec![final_expr],
                        window: Some(win),
                        pronoun_token: pronoun_id,
                    }
                } else {
                     Expression::Aggregate {
                        expr: Box::new(final_expr),
                        func,
                        distinct: final_distinct_id.is_some(),
                        distinct_token: final_distinct_id,
                        pronoun_token: pronoun_id,
                    }
                }
            },
            Some((_, Some(func_name), _)) if func_name.starts_with(CAST_MARK) => {
                let data_type = match &func_name[CAST_MARK.len()..] {
                    "integer" => DataType::Integer,
                    "text" => DataType::Text,
                    "decimal" => DataType::Decimal,
                    "date" => DataType::Date,
                    _ => DataType::Boolean,
                };
                Expression::Cast { expr: Box::new(final_expr), data_type }
            }
            Some((_, Some(func_name), pronoun_id)) => {
                if final_distinct_id.is_some() {
                    emitter.emit(misplaced("a function or date part"));
                }
                Expression::FunctionCall {
                    name: func_name.to_string(),
                    args: vec![final_expr],
                    window: None,
                    pronoun_token: pronoun_id,
                }
            }
            _ => {
                if final_distinct_id.is_some() {
                    emitter.emit(misplaced("a plain value; distinct rows are `पुनरुक्तिम् विना`"));
                }
                final_expr
            }
        }
    });

    // COUNT(*) counts rows: `पङ्क्तीनाम् गणना` ("the count of the rows"),
    // optionally over a window (expert review F5).
    let count_rows = just(Token::Panktinam)
        .ignore_then(just(Token::Ganana))
        .ignore_then(parse_window_spec(recurse_expr.clone()).or_not())
        .map(|window| match window {
            Some(win) => Expression::FunctionCall {
                name: AggFunc::Count.sql_name().to_string(),
                args: vec![Expression::Wildcard],
                window: Some(win),
                pronoun_token: None,
            },
            None => Expression::Aggregate {
                expr: Box::new(Expression::Wildcard),
                func: AggFunc::Count,
                distinct: false,
                distinct_token: None,
                pronoun_token: None,
            },
        });

    let other_eligible = choice((
        parse_case_expression(recurse_expr.clone()),
        parse_window_function(recurse_expr.clone()),
        count_rows,
    ));

    let other_eligible_atom = with_optional_predicate(other_eligible, recurse_expr.clone());

    let other_part = distinct_prefix
        .then(other_eligible_atom)
        .validate(|(prefix_distinct, expr), e, emitter| {
            // A CASE, window function, row count or subquery has no values
            // of its own for DISTINCT to apply to; it used to be dropped.
            if prefix_distinct.is_some() {
                emitter.emit(Rich::custom(e.span(),
                    "`विशिष्ट-` (DISTINCT) is said of an aggregate's values (`विशिष्टायाः भाषायाः गणना`), not of this; count distinct values of a column, or select distinct rows with `पुनरुक्तिम् विना`"));
            }
            expr
        });

    // suffix_part's own internal choice (shashthi_suffix_branch tried before
    // general_or_plain_branch) already commits to a fully-built aggregate/
    // function-call expression - e.g. "संस्थायाः गणना" (COUNT(company)) - the
    // moment that reading succeeds on its own terms, the same way
    // suffix_eligible_atom's plain-atom choice does above. That's fine for a
    // standalone aggregate (a SELECT-list item), but it means a *comparison*
    // built on top of an aggregate LHS (a HAVING clause: "COUNT(company) >
    // 5") never gets a chance: comp_chk above only ever sees the pre-suffix
    // suffix_eligible as its LHS, and by the time suffix_part exists, the
    // choice has already returned.
    //
    // Fixed by parsing suffix_part exactly once and then checking for an
    // *optional* trailing comparison continuation (mirrors
    // parse_comparison_continuation, factored out of parse_comparison for
    // exactly this reuse) - not by wrapping suffix_part in parse_comparison
    // and falling back to a second, independent bare-suffix_part attempt on
    // failure. That naive "try wrapped, then retry bare" version was tried
    // first and reintroduced the same exponential-backtracking blowup fixed
    // elsewhere in this grammar (suffix_part recurses through the whole
    // expression grammar, so deriving it twice at every atom position
    // compounds badly under recursion - see 41_cte.सं, which timed out under
    // that version) - this left-factored version pays for suffix_part once
    // per atom, same as before this fix.
    //
    // EXISTS is left-factored the same way: `यत् <subquery> इति [न] विद्यते`
    // ("… is [not] found"). The subquery is parsed once, as the ordinary
    // atom suffix_part already produces, and a trailing `[न] विद्यते` turns
    // it into EXISTS / NOT EXISTS.
    let exists_continuation = just(Token::Na).or_not().then_ignore(just(Token::Vidyate));
    let suffix_part_with_optional_comparison = suffix_part.clone()
        .then(choice((
            exists_continuation.map(|na| Err(na.is_some())),
            parse_comparison_continuation(recurse_expr.clone()).map(Ok),
        )).or_not())
        .try_map(|(left, continuation_opt), span| match continuation_opt {
            Some(Ok(continuation)) => Ok(build_comparison_expr(left, continuation)),
            Some(Err(negated)) => match left {
                Expression::Subquery(subquery) => Ok(Expression::Exists { subquery, negated }),
                _ => Err(Rich::custom(span, "'विद्यते' (EXISTS) needs a subquery: यत् <query> इति विद्यते")),
            },
            None => Ok(left),
        });

    // other_part (CASE, window functions) goes first: each of its
    // alternatives is gated by a keyword (वरणम्, श्रेणी, पूर्वपङ्क्तेः,
    // चतुर्थांशाङ्कः, …) or by `<number> भागेषु/भागयोः`, so it fails at once on
    // anything else, while suffix_part would commit to the `५` of
    // `५ भागेषु भागाङ्कः` as a literal.
    choice((
        other_part,
        suffix_part_with_optional_comparison,
    )).boxed()
}

/// Parse a full expression (with optional logical operators)

pub fn parse_logical_expression<'a>(
    subquery_parser: impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a
) -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    recursive(|expr| {
        let atom = parse_expr_atom(expr.clone(), subquery_parser.clone());

        // Handle च (AND) and वा (OR) connectors. Each item carries its own
        // particle (`A च B च`, `A वा B वा`). One list uses one connective: a
        // list mixing them (`C च A वा B वा`) has no single reading - is it
        // C and (A or B), or (C and A) or B? - so it is an error asking for
        // the intended group with `यत् … इति`.
        let connective = choice((
            just(Token::Cha).to(crate::ast::LogicalOperator::And),
            just(Token::Va).to(crate::ast::LogicalOperator::Or),
        ));
        atom.clone()
            .then(connective.clone().then(atom.clone()).repeated().collect::<Vec<_>>())
            .then(connective.or_not())
            .map_with(|parts, e| {
                let slice: &[Token] = e.slice();
                (parts, slice.last().map(|t| t.to_string()).unwrap_or_default())
            })
            .validate(|(parts, last), e, emitter| {
                // Several conditions: each takes its own particle, the last
                // one too (`… अस्ति च … अस्ति च इति नियमेन`; expert review L1).
                // Reported without failing the parse, so nothing masks it.
                let ((_, rest), trailing) = &parts;
                if let (Some((op, _)), None) = (rest.last(), trailing) {
                    let particle = if *op == crate::ast::LogicalOperator::Or { "वा" } else { "च" };
                    emitter.emit(Rich::custom(e.span(), super::common::last_conjunct_error(particle, &last)));
                }
                if let (true, Some(op)) = (rest.is_empty(), trailing) {
                    let particle = if *op == crate::ast::LogicalOperator::Or { "वा" } else { "च" };
                    emitter.emit(Rich::custom(e.span(), super::common::lone_conjunct_error(particle)));
                }
                parts
            })
            // Reported without failing the parse: as a failure it was masked
            // by another alternative's "add च" (final review).
            .validate(|((first, rest), trailing), e, emitter| {
                let mut ops = rest.iter().map(|(op, _)| op.clone()).collect::<Vec<_>>();
                ops.extend(trailing);
                if ops.contains(&crate::ast::LogicalOperator::And) && ops.contains(&crate::ast::LogicalOperator::Or) {
                    emitter.emit(Rich::custom(
                        e.span(),
                        "A list mixes 'च' (and) with 'वा' (or), which has no single reading; group the part you mean with 'यत् … इति' (e.g. 'यत् A वा B वा इति च C च')",
                    ));
                }
                rest.into_iter().fold(first, |left, (op, right)| Expression::LogicalOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                })
            })
    })
    // The recursive expression grammar is re-entered at the same cursor
    // position many times over during backtracking (a select-item alias,
    // a HAVING comparison, a window spec, etc. all probe "is there an
    // expression here?" independently). chumsky has no result cache of its
    // own, so a failed sub-parse gets fully re-derived on every such probe -
    // `.memoized()` caches failed attempts at a given cursor for this parser
    // instance and short-circuits repeated derivations at the same position.
    .memoized()
}

/// Parse a select item (non-logical top level, but allows logical in parens)

pub fn parse_expression<'a>() -> impl Parser<'a, &'a [Token], Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // Non-subquery fallback parser for standalone expression parsing
        let subquery_stub = any().filter(|_| false).map(|_| Expression::Literal(Literal::Number("0".to_string()))).boxed();
    parse_logical_expression(subquery_stub)
}
