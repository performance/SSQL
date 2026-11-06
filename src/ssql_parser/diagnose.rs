//! Plain words for mistakes the grammar's own errors describe poorly. A failed
//! parse reports its furthest failure, which for these is some alternative's
//! expectation ("expected genitive") and not the mistake. These run only after
//! the parse has failed, so they cannot reject a valid statement.

use super::Token;

/// What was wrong, and the token it was at.
type Diagnosis = (&'static str, usize);

fn is_result_noun(t: &Token) -> bool {
    matches!(t, Token::Yogah | Token::Gunanam | Token::Avashistam | Token::Labdham)
}

fn is_list_pronoun(t: &Token) -> bool {
    matches!(t, Token::Ityayoh(_) | Token::Ityesham(_) | Token::Ityetasam(_))
}

/// A remainder taken as a subtrahend without its group.
fn ungrouped_remainder(tokens: &[Token]) -> Option<Diagnosis> {
    tokens.windows(2).position(|w| matches!(w, [Token::Avashistam, Token::Nivarya]))
        .map(|i| ("a remainder as a subtrahend is grouped, with the accusative pronoun: `मूल्यात् यत् प्रकाशनवर्षात् १ विशोध्य अवशिष्टम् इति इतीदम् विशोध्य अवशिष्टम्`", i))
}

/// A result noun as an operand of a sum or product, without its group.
fn ungrouped_compound_operand(tokens: &[Token]) -> Option<Diagnosis> {
    (0..tokens.len()).find(|&i| {
        is_result_noun(&tokens[i])
            && matches!(tokens.get(i + 1), Some(Token::Cha))
            && tokens[i + 2..].iter().take(6).take_while(|t| !matches!(t, Token::Purnadanda | Token::FetchVerb(_))).any(is_list_pronoun)
    }).map(|i| ("a sum, product, remainder or quotient as an operand of a sum or product is grouped: `यत् मूल्यम् च २ च इत्यनयोः योगः इति च ३ च इत्यनयोः गुणनम्`", i))
}

/// The value of a CASE branch ends where the branch's words give out, so
/// arithmetic there needs a group; and one condition does not mix च and वा.
fn in_case(tokens: &[Token]) -> Option<Diagnosis> {
    for (i, t) in tokens.iter().enumerate() {
        let body = &tokens[i + 1..];
        if matches!(t, Token::Tasmin | Token::Anyasmin) && !matches!(body.first(), Some(Token::Yat)) {
            let end = body.iter().position(|t| matches!(t, Token::Yasmin | Token::Anyasmin | Token::Purnadanda | Token::FetchVerb(_))).unwrap_or(body.len());
            if body[..end].iter().any(|t| matches!(t, Token::Nivarya | Token::Vibhajya)) {
                return Some(("a CASE branch's value is one value, so arithmetic there is grouped: `तस्मिन् यत् मूल्यात् ५ विशोध्य अवशिष्टम् इति`", i));
            }
        }
        if matches!(t, Token::Yasmin) {
            let end = body.iter().position(|t| matches!(t, Token::Tasmin | Token::Purnadanda)).unwrap_or(body.len());
            let cond = &body[..end];
            let plain = !cond.iter().any(|t| matches!(t, Token::Yat));
            if plain && cond.iter().any(|t| matches!(t, Token::Cha)) && cond.iter().any(|t| matches!(t, Token::Va)) {
                return Some(("A list mixes 'च' (and) with 'वा' (or), which has no single reading; group the part you mean with 'यत् … इति' (e.g. 'यत् A वा B वा इति च C च')", i));
            }
        }
    }
    None
}

/// `पुनरुक्तिम् विना` anywhere but before the select list.
fn misplaced_distinct(tokens: &[Token]) -> Option<Diagnosis> {
    tokens.windows(3).position(|w| {
        matches!(w[1], Token::Punaruktim) && matches!(w[2], Token::Vina)
            && matches!(w[0], Token::Prathama { .. } | Token::Identifier(_) | Token::Cha)
    }).map(|i| ("`पुनरुक्तिम् विना` (DISTINCT) comes once, before the first selected item: `पुनरुक्तिम् विना भाषा च विधा च आह्रियेताम्`", i + 1))
}

/// A gerund's object in the wrong case, by the word just before it.
fn gerund_cases(tokens: &[Token]) -> Option<Diagnosis> {
    for i in 1..tokens.len() {
        match (&tokens[i], &tokens[i - 1]) {
            (Token::Vibhajya, Token::Prathama { .. }) => {
                return Some(("what divides is in the instrumental: `मूल्यम् सङ्ख्यया विभज्य लब्धम्`", i - 1));
            }
            (Token::Nivarya, Token::Number(_) | Token::Dvitiya { .. }) if i >= 2 => {
                let nominative = matches!(tokens[i - 2], Token::Prathama { .. });
                let a_stem_genitive = matches!(&tokens[i - 2], Token::Shashthi { text, .. } if text.ends_with("स्य"));
                if nominative || a_stem_genitive {
                    return Some(("what is subtracted from is in the ablative: `मूल्यात् ५ विशोध्य अवशिष्टम्`", i - 2));
                }
            }
            (Token::Itidam, previous) if !matches!(previous, Token::Iti) => {
                return Some(("`इतीदम्` follows a group: `यत् … अवशिष्टम् इति इतीदम्`", i));
            }
            _ => {}
        }
    }
    None
}

/// The message for a statement whose parse failed, if one of these applies.
pub(crate) fn diagnose(tokens: &[Token]) -> Option<String> {
    if tokens.is_empty() {
        return Some("there is nothing to compile: an SSQL statement ends in `।`, such as `पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।`".to_string());
    }
    ungrouped_remainder(tokens)
        .or_else(|| ungrouped_compound_operand(tokens))
        .or_else(|| in_case(tokens))
        .or_else(|| misplaced_distinct(tokens))
        .or_else(|| gerund_cases(tokens))
        .map(|(message, at)| format!("{} at {}..{}", message, at, at + 1))
}
