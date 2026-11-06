//! Declaration agreement check.
//!
//! Every table, column and CTE declaration states its name together with an
//! explicit stem, gender and number (`पुस्तकानि इति पटलम् "books" इति आख्या
//! पुस्तक इति नपुंसकलिङ्गम् बहुवचनम्`). The declared name must be exactly the
//! nominative that the morphology engine produces for that stem, gender and
//! number; otherwise the declaration contradicts itself (a plural name declared
//! singular, a missing visarga, a stem typo, a wrong gender, …).

use crate::ast::{AstNode, DdlStatement, Statement};
use crate::lexer::tokenize;
use crate::annotator::{Annotator, TokenType};
use crate::morphology::MorphologyEngine;
use crate::schema::Schema;
use crate::ssql_parser::{convert_vibhakti_tokens, parse};
use crate::types::{Linga, Vachana, Vibhakti};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarationKind {
    Table,
    Column,
    Cte,
}

#[derive(Debug, Clone)]
pub struct DeclarationFinding {
    pub kind: DeclarationKind,
    pub name: String,
    pub stem: String,
    pub gender: Linga,
    pub number: Vachana,
    pub problem: String,
}

struct Declaration {
    kind: DeclarationKind,
    name: String,
    stem: String,
    gender: Linga,
    number: Vachana,
}

fn gender_word(word: &str) -> Option<Linga> {
    match word {
        "पुंलिङ्गम्" => Some(Linga::Masculine),
        "स्त्रीलिङ्गम्" => Some(Linga::Feminine),
        "नपुंसकलिङ्गम्" => Some(Linga::Neuter),
        _ => None,
    }
}

fn number_word(word: &str) -> Option<Vachana> {
    match word {
        "एकवचनम्" => Some(Vachana::Eka),
        "द्विवचनम्" => Some(Vachana::Dvi),
        "बहुवचनम्" => Some(Vachana::Bahu),
        _ => None,
    }
}

/// Table and column declarations, read from the parsed DDL.
fn ddl_declarations(source: &str, out: &mut Vec<Declaration>) {
    let Ok(lexer_tokens) = tokenize(source) else { return };
    let Ok(mut annotator) = Annotator::new() else { return };
    let Ok(enriched) = annotator.annotate(lexer_tokens, &Schema::default()) else { return };
    let Ok(ast) = parse(convert_vibhakti_tokens(enriched)) else { return };
    let statements = match ast {
        AstNode::QueryBatch { statements, .. } => statements,
        AstNode::Statement(stmt) => vec![stmt],
    };
    for stmt in statements {
        let Statement::Ddl(DdlStatement::Create(create)) = stmt else { continue };
        out.push(Declaration {
            kind: DeclarationKind::Table,
            name: create.name.text.clone(),
            stem: create.pratipadika.clone(),
            gender: create.gender,
            number: create.number,
        });
        for col in &create.columns {
            out.push(Declaration {
                kind: DeclarationKind::Column,
                name: col.name.text.clone(),
                stem: col.pratipadika.clone(),
                gender: col.gender,
                number: col.number,
            });
        }
    }
}

/// CTE declarations: `<name> इति आख्या <stem> इति <gender> <number>`, where the
/// name is an identifier (a physical binding would be a quoted string).
fn cte_declarations(source: &str, out: &mut Vec<Declaration>) {
    let Ok(tokens) = tokenize(source) else { return };
    for i in 2..tokens.len().saturating_sub(4) {
        if tokens[i].text != "आख्या" || tokens[i - 1].text != "इति" {
            continue;
        }
        if !matches!(tokens[i - 2].token_type, TokenType::Identifier) {
            continue;
        }
        if tokens[i + 2].text != "इति" {
            continue;
        }
        let (Some(gender), Some(number)) = (gender_word(&tokens[i + 3].text), number_word(&tokens[i + 4].text)) else {
            continue;
        };
        out.push(Declaration {
            kind: DeclarationKind::Cte,
            name: tokens[i - 2].text.clone(),
            stem: tokens[i + 1].text.clone(),
            gender,
            number,
        });
    }
}

/// Every (stem, gender) pair declared in `source`, for corpus-wide
/// consistency checks.
pub fn declared_stems(source: &str) -> Vec<(String, Linga)> {
    let mut decls = Vec::new();
    ddl_declarations(source, &mut decls);
    cte_declarations(source, &mut decls);
    decls.into_iter().map(|d| (d.stem, d.gender)).collect()
}

/// A word whose last letter is a consonant carrying its inherent अ (no vowel
/// sign, virama, visarga or anusvāra after it).
fn ends_in_inherent_a(word: &str) -> bool {
    word.chars().last().map_or(false, |c| ('\u{0915}'..='\u{0939}').contains(&c) || ('\u{0958}'..='\u{095F}').contains(&c))
}

fn is_devanagari_word(word: &str) -> bool {
    word.chars().all(|c| ('\u{0900}'..='\u{097F}').contains(&c))
}

/// Check every declaration in `source` and return the ones that contradict
/// themselves. An empty result means every declared name is the nominative of
/// its declared stem, gender and number.
pub fn check_declarations(source: &str) -> Vec<DeclarationFinding> {
    let mut decls = Vec::new();
    ddl_declarations(source, &mut decls);
    cte_declarations(source, &mut decls);

    let mut morph = match MorphologyEngine::new() {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };
    let mut findings = Vec::new();
    for d in decls {
        let mut push = |problem: String| {
            findings.push(DeclarationFinding {
                kind: d.kind.clone(),
                name: d.name.clone(),
                stem: d.stem.clone(),
                gender: d.gender,
                number: d.number,
                problem,
            })
        };
        if !is_devanagari_word(&d.name) || !is_devanagari_word(&d.stem) {
            push("name or stem contains non-Devanagari characters (use a compound, not `_`)".to_string());
            continue;
        }
        // There are no feminine अ-stems; the morphology would quietly decline
        // `कक्ष इति स्त्रीलिङ्गम्` as कक्षा, so the stem is written as declined.
        if d.gender == Linga::Feminine && ends_in_inherent_a(&d.stem) {
            push(format!("a feminine stem does not end in अ; write `{}ा`", d.stem));
            continue;
        }
        match morph.decline_with_gender(&d.stem, d.gender, d.number) {
            Ok(table) => match table.forms.get(&Vibhakti::Prathama) {
                Some(nominative) if *nominative == d.name => {}
                Some(nominative) => push(format!(
                    "declared name `{}` is not the nominative of stem `{}` ({:?}, {:?}); the nominative is `{}`",
                    d.name, d.stem, d.gender, d.number, nominative
                )),
                None => push(format!("morphology produced no nominative for stem `{}`", d.stem)),
            },
            Err(e) => push(format!("morphology cannot decline stem `{}`: {}", d.stem, e)),
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feminine_stem_in_a_is_a_finding() {
        let findings = check_declarations(
            "कक्षाः इति पटलम् \"classes\" इति आख्या कक्ष इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत्र \
             सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।",
        );
        assert_eq!(findings.len(), 1, "{:?}", findings);
        assert!(findings[0].problem.contains("write `कक्षा`"), "{}", findings[0].problem);
    }
}
