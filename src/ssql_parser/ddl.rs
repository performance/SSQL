//! DDL statements: CREATE TABLE/VIEW/FUNCTION, DROP TABLE/VIEW, CREATE/DROP
//! INDEX, ALTER TABLE.

use chumsky::prelude::*;
use crate::ast::{Statement, Identifier, DdlStatement, CreateStatement, DropStatement};
use super::Token;
use super::common::{parse_type, parse_genitive_marker, conj_list};

/// Gender marker, shared by column/table/view stem declarations.
pub(crate) fn parse_gender<'a>() -> impl Parser<'a, &'a [Token], crate::types::Linga, extra::Err<Rich<'a, Token>>> + Clone {
    choice((
        just(Token::Pumlingam).to(crate::types::Linga::Masculine),
        just(Token::Strilingam).to(crate::types::Linga::Feminine),
        just(Token::Napumsakalingam).to(crate::types::Linga::Neuter),
    ))
}

/// Number marker, shared by column/table/view stem declarations.
pub(crate) fn parse_vachana<'a>() -> impl Parser<'a, &'a [Token], crate::types::Vachana, extra::Err<Rich<'a, Token>>> + Clone {
    choice((
        just(Token::Ekavachanam).to(crate::types::Vachana::Eka),
        just(Token::Dvivachanam).to(crate::types::Vachana::Dvi),
        just(Token::Bahuvachanam).to(crate::types::Vachana::Bahu),
    ))
}

/// Explicit stem+gender+vachana clause: `<stem> इति <gender> <vachana>`. Used
/// after a declared `"sql_name" इति आख्या` (columns/tables/views) or, for CTEs,
/// directly after `इति आख्या` with no sql_name string. The stem is taken
/// verbatim from the identifier's surface text (not its own guessed
/// `.pratipadika`) since the whole point is the author states the citation
/// form directly, bypassing morphological guessing entirely.
pub(crate) fn parse_stem_gender_vachana<'a>(
    ident_parser: impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], (Identifier, crate::types::Linga, crate::types::Vachana), extra::Err<Rich<'a, Token>>> + Clone + 'a {
    ident_parser
        .then_ignore(just(Token::Iti))
        .then(parse_gender())
        .then(parse_vachana())
        .try_map(|((stem, gender), vachana), span| {
            if crate::reserved_words::is_reserved_builtin(&stem.text) {
                return Err(Rich::custom(
                    span,
                    format!(
                        "'{}' is a reserved builtin function name and cannot be used as a stem",
                        stem.text
                    ),
                ));
            }
            // The इत्य- pronoun family (इत्यनेन/इत्यनया/इत्यनया/इत्येभिः/इत्याभिः)
            // all lex as Token::Tritiya with pratipadika "इति" - a Tritiya-case
            // noun's own pratipadika is never literally "इति", so this is an
            // unambiguous signal that ident_parser matched a pronoun keyword,
            // not a real content word (these tokens have no dedicated Token
            // variant of their own, unlike most other keywords).
            if stem.pratipadika == "इति" {
                return Err(Rich::custom(
                    span,
                    format!(
                        "'{}' is a reserved pronoun keyword and cannot be used as a stem",
                        stem.text
                    ),
                ));
            }
            Ok((stem, gender, vachana))
        })
}

/// Helper for parsing expression atoms (used in both logical and non-logical contexts)

pub(crate) fn parse_type_name<'a>(
    ident_parser: impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    choice((
        just(Token::Patalam).map(|_| Identifier {
            text: "पटलम्".to_string(),
            vibhakti_form: crate::types::VibhaktiForm { vibhakti: crate::types::Vibhakti::Prathama, vachana: crate::types::Vachana::Eka, linga: Some(crate::types::Linga::Neuter) },
            pratipadika: "पटल".to_string(),
            symbol_type: crate::types::SymbolType::Table,
            span: None,
        }),
        ident_parser
    ))
}


pub(crate) fn parse_column_def<'a>(
    ident_parser: impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], crate::ast::ColumnDef, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    // Constraints parser
    let constraint_parser = choice((
        just(Token::Prathamikah).to(crate::ast::Constraint::PrimaryKey),
        just(Token::Anivaryah).to(crate::ast::Constraint::NotNull),
        just(Token::Vaikalpikah).to(crate::ast::Constraint::Null),
        just(Token::Advitiyah).to(crate::ast::Constraint::Unique),
        just(Token::Kramikah).to(crate::ast::Constraint::AutoIncrement),
        // A foreign key: the referenced table in the locative plural with
        // बद्ध agreeing with the column, "tied to the customers"
        // (`ग्राहकेषु बद्धा ग्राहकसङ्ख्या`; expert review M1). The schema
        // checks that the table is in the locative plural.
        ident_parser.clone()
            .then_ignore(just(Token::Baddhah))
            .map(crate::ast::Constraint::ForeignKey),
        // The retired `<table> प्रति बद्धः` still parses, so it is reported
        // as such rather than as a confusing parse error.
        ident_parser.clone()
            .then_ignore(just(Token::Prati))
            .then_ignore(just(Token::Baddhah))
            .validate(|ident, e, emitter| {
                emitter.emit(Rich::custom(
                    e.span(),
                    format!("`{} प्रति बद्ध…` is no longer SSQL: a foreign key names its table in the locative plural (`ग्राहकेषु बद्धा ग्राहकसङ्ख्या`)", ident.text),
                ));
                ident
            })
            .map(crate::ast::Constraint::ForeignKey),
    ));

    // Constraints list
    let constraints_list = constraint_parser.repeated().collect::<Vec<_>>();

    // नामन् इति नपुंसकलिङ्गम् एकवचनम्
    let stem_clause = parse_stem_gender_vachana(ident_parser.clone());

    // Define Column Definition. The quoted physical-name string is optional
    // (a bare Sanskrit column can skip an English override and use its own
    // stem as the physical name), but `इति आख्या` itself is a fixed marker
    // always followed by the mandatory stem+gender+vachana clause - no more
    // guessing the stem from the declared name's surface form.
    constraints_list.clone()
        .then(ident_parser.clone())
        .then_ignore(just(Token::Iti))
        .then(constraints_list.clone())
        .then(parse_type())
        .then(constraints_list)
        .then(
            any().try_map(|tok, span: SimpleSpan| match tok {
                Token::StringLit(s) => Ok(s),
                _ => Err(Rich::custom(span, "expected a quoted string for `आख्या`")),
            })
            .or_not()
        )
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Aakhya))
        .then(stem_clause)
        .map(|((((((mut constraints, name), mid_constraints), data_type), post_constraints), sql_name), (stem, gender, number))| {
             constraints.extend(mid_constraints);
             constraints.extend(post_constraints);
             crate::ast::ColumnDef {
                 pratipadika: stem.text.clone(),
                 name,
                 data_type,
                 sql_name: sql_name.or_else(|| Some(stem.text.clone())),
                 gender,
                 number,
                 constraints,
             }
        })
}

/// Parse CREATE statement

pub(crate) fn parse_create_statement<'a>(
    ident_parser: impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    query_parser: impl Parser<'a, &'a [Token], Statement, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    expr_parser: impl Parser<'a, &'a [Token], crate::ast::Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], DdlStatement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let column_def = parse_column_def(ident_parser.clone());

    // List of columns: A Iti Type Cha B Iti Type ... - terminated by the
    // table/view's own trailing इति पटलम्/इति दृश्यम् marker; column_def
    // never starts with इति, so the list naturally stops there. Requiring
    // `at_least(1)` ensures `columns_list CreateVerb` cannot match zero
    // columns and consume `CreateVerb` on a `CREATE VIEW` statement.
    let columns_list = conj_list(column_def);

    let parse_type_name = parse_type_name(ident_parser.clone());

    // Terminated by its own trailing इति नियमेन marker.
    let table_config = conj_list(ident_parser.clone())
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Niyamena));

    let word_token = |expected: &'static [&'static str]| {
        any().try_map(move |tok, span: SimpleSpan| {
            let text = match &tok {
                Token::Prathama { text, .. }
                | Token::Dvitiya { text, .. }
                | Token::Tritiya { text, .. }
                | Token::Chaturthi { text, .. }
                | Token::Panchami { text, .. }
                | Token::Shashthi { text, .. }
                | Token::Saptami { text, .. }
                | Token::Identifier(text) => text.as_str(),
                _ => return Err(Rich::custom(span, format!("found `{}` where {} was expected", tok, one_of(&expected)))),
            };
            if expected.contains(&text) {
                Ok(text.to_string())
            } else {
                Err(Rich::custom(span, format!("found `{}` where {} was expected", text, one_of(&expected))))
            }
        })
    };

    let udf_param = ident_parser.clone()
        .then_ignore(just(Token::Iti))
        .then(parse_type())
        .map(|(name, data_type)| crate::ast::UdfParam { name, data_type });

    let udf_create = ident_parser.clone()
        .then_ignore(just(Token::Iti))
        .then_ignore(word_token(&["शुद्धम्"]))
        .then_ignore(just(Token::Karyam))
        .then(
            any().try_map(|tok, span: SimpleSpan| match tok {
                Token::StringLit(s) => Ok(s),
                _ => Err(Rich::custom(span, "expected a quoted string for Function `आख्या`")),
            })
            .then_ignore(just(Token::Iti))
            .then_ignore(just(Token::Aakhya))
            .or_not()
        )
        .then_ignore(just(Token::CreateVerb))
        .then_ignore(just(Token::Yatra))
        .then(parse_type())
        .then_ignore(word_token(&["प्रतिदीयते"]))
        .then_ignore(just(Token::Cha).or_not())
        .then(
            conj_list(udf_param)
        )
        .then(word_token(&["दीयते", "दीयेते", "दीयन्ते"]))
        .then(expr_parser)
        .then_ignore(just(Token::Iti))
        .then_ignore(word_token(&["उत्तरम्"]))
        .then_ignore(word_token(&["प्रतिदीयताम्"]))
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Karyam))
        .then_ignore(ident_parser.clone().or_not())
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|(((((name, sql_name), return_type), params), param_verb), body)| {
            DdlStatement::CreateUdf(crate::ast::CreateUdfStatement {
                name,
                sql_name,
                return_type,
                params,
                param_verb,
                body: Box::new(body),
            })
        });

    let table_create = ident_parser.clone()
        .then_ignore(just(Token::Iti))
        .then(just(Token::Tatkalikam).to(true).or_not().map(|v| v.unwrap_or(false)))
        .then(parse_type_name.clone()) // Type Name (e.g., Table/View)
        .then(
            any().try_map(|tok, span: SimpleSpan| match tok {
                Token::StringLit(s) => Ok(s),
                _ => Err(Rich::custom(span, "expected a quoted string for Table `आख्या`")),
            })
            .or_not()
        )
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Aakhya))
        .then(parse_stem_gender_vachana(ident_parser.clone()))
        .then(
            choice((
                // Style 1: [table_config] CreateVerb Yatra columns Iti Type
                table_config.or_not().map(|v| v.unwrap_or_default())
                    .then_ignore(just(Token::CreateVerb))
                    .then_ignore(just(Token::Yatra))
                    .then(columns_list.clone().or_not().map(|v| v.unwrap_or_default()))
                    .then_ignore(just(Token::Iti))
                    .then_ignore(parse_type_name.clone())
                    .map(|(config, columns)| (config, columns)),

                // Style 2: columns CreateVerb
                columns_list.clone()
                    .then_ignore(just(Token::CreateVerb))
                    .map(|columns| (vec![], columns)),
            ))
        )
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|(((((name, is_temporary), type_name), sql_name), (stem, gender, number)), (config, columns))| {
            DdlStatement::Create(CreateStatement {
                pratipadika: stem.text.clone(),
                gender,
                number,
                name,
                sql_name: sql_name.or_else(|| Some(stem.text.clone())),
                type_name,
                is_temporary,
                config,
                columns,
                query: None,
            })
        });

    let view_create = ident_parser.clone()
        .then_ignore(just(Token::Iti))
        .then(just(Token::Tatkalikam).to(true).or_not().map(|v| v.unwrap_or(false)))
        .then(parse_type_name.clone())
        .then(
            any().try_map(|tok, span: SimpleSpan| match tok {
                Token::StringLit(s) => Ok(s),
                _ => Err(Rich::custom(span, "expected a quoted string for View `आख्या`")),
            })
            .or_not()
        )
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Aakhya))
        .then(parse_stem_gender_vachana(ident_parser.clone()))
        .then_ignore(just(Token::CreateVerb))
        .then_ignore(just(Token::Yat))
        .then(query_parser.clone())
        .then_ignore(just(Token::Iti))
        .then_ignore(parse_type_name.clone())
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|(((((name, is_temporary), type_name), sql_name), (stem, gender, number)), query_stmt)| {
            DdlStatement::Create(CreateStatement {
                pratipadika: stem.text.clone(),
                gender,
                number,
                name,
                sql_name: sql_name.or_else(|| Some(stem.text.clone())),
                type_name,
                is_temporary,
                config: vec![],
                columns: vec![],
                query: Some(Box::new(query_stmt)),
            })
        });

    choice((udf_create, table_create, view_create))
}


pub(crate) fn parse_drop_statement<'a>(
    ident_parser: impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], DdlStatement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let parse_type_name_p = parse_type_name(ident_parser.clone());
    ident_parser.clone()
        .then_ignore(just(Token::Iti))
        .then(parse_type_name_p.clone())
        .then(
            just(Token::Vidyate)
                .then(just(Token::Chet))
                .to(true)
                .or_not()
                .map(|v| v.is_some())
        )
        .then_ignore(just(Token::DropVerb))
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|((name, type_name), if_exists)| {
            DdlStatement::Drop(DropStatement {
                name,
                type_name,
                is_exists_check: if_exists,
            })
        })
}


pub(crate) fn parse_create_index_statement<'a>(
    ident_parser: impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], DdlStatement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let index_column = ident_parser.clone()
        .then(
            choice((
                just(Token::Arohah).to(crate::ast::OrderDirection::Asc),
                just(Token::Avarohah).to(crate::ast::OrderDirection::Desc),
            ))
            .or_not()
        )
        .map(|(name, direction)| crate::ast::IndexColumn { name, direction });

    // The columns an index is built on (expert review Q6b): several form a
    // च-list closed by the genitive pronoun of their number (`शीर्षकम् आरोहः
    // च मूल्यम् अवरोहः च इत्यनयोः आधारे`, three or more `इत्येषाम्`); one
    // column with a direction takes `इत्यस्य` (`मूल्यम् अवरोहः इत्यस्य
    // आधारे`); a plain one is a genitive (`शीर्षकस्य आधारे`).
    let several = index_column.clone()
        .then_ignore(just(Token::Cha))
        .repeated()
        .at_least(2)
        .collect::<Vec<_>>()
        .then(any().filter(|t| matches!(t, Token::Ityayoh(_) | Token::Ityesham(_))))
        .try_map(|(cols, pronoun), span| match (cols.len(), &pronoun) {
            (2, Token::Ityayoh(_)) => Ok(cols),
            (n, Token::Ityesham(_)) if n >= 3 => Ok(cols),
            _ => Err(Rich::custom(span, "Two index columns are closed by `इत्यनयोः`, three or more by `इत्येषाम्`")),
        });
    let one_directed = ident_parser.clone()
        .then(choice((
            just(Token::Arohah).to(crate::ast::OrderDirection::Asc),
            just(Token::Avarohah).to(crate::ast::OrderDirection::Desc),
        )))
        .then_ignore(any().filter(|t| matches!(t, Token::Ityasya(_))))
        .map(|(name, direction)| vec![crate::ast::IndexColumn { name, direction: Some(direction) }]);
    let columns_spec = choice((
        several,
        one_directed,
        ident_parser.clone()
            .then_ignore(parse_genitive_marker().or_not())
            .map(|name| vec![crate::ast::IndexColumn { name, direction: None }]),
    ));

    let index_name_parser = choice((
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::StringLit(s) => Ok(Identifier {
                text: s.clone(),
                vibhakti_form: crate::types::VibhaktiForm { vibhakti: crate::types::Vibhakti::Prathama, vachana: crate::types::Vachana::Eka, linga: None },
                pratipadika: s,
                symbol_type: crate::types::SymbolType::Column,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected string literal")),
        }),
        ident_parser.clone()
    ));

    let if_not_exists = just(Token::Na)
        .then_ignore(just(Token::Vidyate))
        .then_ignore(just(Token::Chet))
        .to(true)
        .or_not()
        .map(|v| v.is_some());

    if_not_exists
        .then(ident_parser.clone()) // Table name
        .then_ignore(parse_genitive_marker().or_not()) // optional genitive on table name
        .then(columns_spec)
        .then_ignore(just(Token::Aadhare))
        .then(index_name_parser)
        .then_ignore(just(Token::Iti))
        .then(just(Token::Advitiyah).to(true).or_not().map(|v| v.is_some())) // is_unique
        .then_ignore(just(Token::Suchi)) // सूची
        .then_ignore(just(Token::CreateVerb)) // रच्यताम्
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|((((if_not_exists, table_name), columns), index_name), is_unique)| {
            DdlStatement::CreateIndex(crate::ast::CreateIndexStatement {
                name: index_name,
                table_name,
                columns,
                is_unique,
                if_not_exists,
            })
        })
}


pub(crate) fn parse_drop_index_statement<'a>(
    ident_parser: impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], DdlStatement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let index_name_parser = choice((
        any().try_map(|tok, span: SimpleSpan| match tok {
            Token::StringLit(s) => Ok(Identifier {
                text: s.clone(),
                vibhakti_form: crate::types::VibhaktiForm { vibhakti: crate::types::Vibhakti::Prathama, vachana: crate::types::Vachana::Eka, linga: None },
                pratipadika: s,
                symbol_type: crate::types::SymbolType::Column,
                span: Some((span.start(), span.end())),
            }),
            _ => Err(Rich::custom(span, "expected string literal")),
        }),
        ident_parser.clone()
    ));

    let if_exists = just(Token::Vidyate)
        .then(just(Token::Chet))
        .to(true)
        .or_not()
        .map(|v| v.is_some());

    if_exists
        .then(index_name_parser)
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Suchi))
        .then_ignore(just(Token::DropVerb))
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|(is_exists_check, name)| {
            DdlStatement::DropIndex(crate::ast::DropIndexStatement {
                name,
                is_exists_check,
            })
        })
}


pub(crate) fn parse_alter_table_statement<'a>(
    ident_parser: impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], DdlStatement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let column_def = parse_column_def(ident_parser.clone());

    let action_add = column_def
        .then_ignore(just(Token::AddVerb))
        .map(crate::ast::AlterAction::AddColumn);

    let action_drop = ident_parser.clone()
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::DropVerb))
        .map(crate::ast::AlterAction::DropColumn);

    // RENAME COLUMN/TABLE deliberately avoid नाम as a "the name" marker noun:
    // genitive+नाम is token-for-token identical to an ordinary field-access
    // noun phrase used everywhere else in the grammar (e.g. "पुस्तकस्य नाम" =
    // book.name), which the parser could only ever disambiguate by looking
    // all the way ahead to the trailing verb. तस्मात् (ablative of तद्,
    // anaphoric "from that [just-named]") carries its own unambiguous case
    // marking and is a dedicated token, so no such lookahead is needed, and
    // नाम stays completely free to use as an ordinary column name -
    // including one being renamed by these very statements.
    let action_rename_column = ident_parser.clone()
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::Tasmat))
        .then(choice((
            any().try_map(|tok, span: SimpleSpan| match tok {
                Token::StringLit(s) => Ok(Identifier {
                    text: s.clone(),
                    vibhakti_form: crate::types::VibhaktiForm { vibhakti: crate::types::Vibhakti::Prathama, vachana: crate::types::Vachana::Eka, linga: None },
                    pratipadika: s,
                    symbol_type: crate::types::SymbolType::Column,
                    span: Some((span.start(), span.end())),
                }),
                _ => Err(Rich::custom(span, "expected string literal")),
            }),
            ident_parser.clone()
        )))
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::AbhidhiyatamVerb))
        .map(|(old_name, new_name)| crate::ast::AlterAction::RenameColumn { old_name, new_name });

    // No table-name/पटलम् re-mention needed here: `target` (below) already
    // consumed "TableName इति [पटले]?" as the shared ALTER TABLE prefix, so
    // this action starts directly at तस्मात्.
    let action_rename_table = just(Token::Tasmat)
        .ignore_then(choice((
            any().try_map(|tok, span: SimpleSpan| match tok {
                Token::StringLit(s) => Ok(Identifier {
                    text: s.clone(),
                    vibhakti_form: crate::types::VibhaktiForm { vibhakti: crate::types::Vibhakti::Prathama, vachana: crate::types::Vachana::Eka, linga: None },
                    pratipadika: s,
                    symbol_type: crate::types::SymbolType::Table,
                    span: Some((span.start(), span.end())),
                }),
                _ => Err(Rich::custom(span, "expected string literal")),
            }),
            ident_parser.clone()
        )))
        .then_ignore(just(Token::Iti))
        .then_ignore(just(Token::AbhidhiyatamVerb))
        .map(crate::ast::AlterAction::RenameTable);

    let action = choice((
        action_add,
        action_drop,
        action_rename_column,
        action_rename_table,
    ));

    // Target: TableName इति Patale/Sarvayam (optional)
    let target = ident_parser.clone()
        .then_ignore(just(Token::Iti))
        .then_ignore(choice((
            just(Token::Patalam).ignored(),
            ident_parser.clone().ignored(),
        )).or_not());

    target
        .then(action)
        .then_ignore(just(Token::Purnadanda).or_not())
        .map(|(table_name, action)| {
            DdlStatement::AlterTable(crate::ast::AlterTableStatement {
                table_name,
                action,
            })
        })
}


pub(crate) fn parse_ddl<'a>(
    ident_parser: impl Parser<'a, &'a [Token], Identifier, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    query_parser: impl Parser<'a, &'a [Token], Statement, extra::Err<Rich<'a, Token>>> + Clone + 'a,
    expr_parser: impl Parser<'a, &'a [Token], crate::ast::Expression, extra::Err<Rich<'a, Token>>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], DdlStatement, extra::Err<Rich<'a, Token>>> + Clone + 'a {
    let create = parse_create_statement(ident_parser.clone(), query_parser, expr_parser);
    let drop = parse_drop_statement(ident_parser.clone());
    let create_index = parse_create_index_statement(ident_parser.clone());
    let drop_index = parse_drop_index_statement(ident_parser.clone());
    let alter_table = parse_alter_table_statement(ident_parser.clone());
    choice((create, drop, create_index, drop_index, alter_table))
}

/// "`a`", "`a` or `b`", "one of `a`, `b` or `c`", for an error message.
fn one_of(words: &[&str]) -> String {
    let quoted: Vec<String> = words.iter().map(|w| format!("`{}`", w)).collect();
    match quoted.len() {
        0 => String::new(),
        1 => quoted[0].clone(),
        n => format!("one of {} or {}", quoted[..n - 1].join(", "), quoted[n - 1]),
    }
}
