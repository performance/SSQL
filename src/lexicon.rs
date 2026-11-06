//! Canonical-vocabulary and orthography gate.
//!
//! `lexicon.json` lists words that are not Sanskrit (or not Sanskrit in the
//! sense SSQL would use them), with the native word to use instead. The
//! orthography rules below enforce the project's spelling standard: class
//! nasals before stops (`सङ्ख्या`, not `संख्या`), `म्` rather than a final
//! anusvāra, and no chandrabindu, nukta or underscore in a word.

use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
pub struct DeniedWord {
    pub word: String,
    pub why: String,
    #[serde(rename = "use")]
    pub replacement: String,
    /// Formerly an SSQL keyword: the lexer rejects any word containing it.
    #[serde(default)]
    pub retired_keyword: bool,
}

#[derive(Deserialize)]
struct LexiconFile {
    denied: Vec<DeniedWord>,
}

/// The denied-word table embedded from `lexicon.json`.
pub fn denied_words() -> &'static [DeniedWord] {
    static TABLE: OnceLock<Vec<DeniedWord>> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str::<LexiconFile>(include_str!("lexicon.json"))
            .expect("src/lexicon.json must be valid")
            .denied
    })
}

/// The retired keyword contained in `word`, if any: the former AVG keyword
/// itself, an inflected form of it, or a compound built on it.
pub fn retired_keyword_in(word: &str) -> Option<&'static DeniedWord> {
    denied_words().iter().find(|d| d.retired_keyword && word.contains(d.word.as_str()))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LexiconRule {
    DeniedWord,
    FinalAnusvara,
    AnusvaraBeforeStop,
    Chandrabindu,
    Nukta,
    Underscore,
}

#[derive(Debug, Clone)]
pub struct LexiconFinding {
    pub rule: LexiconRule,
    pub word: String,
    pub message: String,
}

const ANUSVARA: char = '\u{0902}';
const CHANDRABINDU: char = '\u{0901}';
const NUKTA: char = '\u{093C}';

fn is_devanagari_letter(c: char) -> bool {
    // Devanagari block minus the dandas (U+0964, U+0965) and the digits
    // (U+0966..=U+096F), which separate or stand outside words.
    ('\u{0900}'..='\u{097F}').contains(&c) && !('\u{0964}'..='\u{096F}').contains(&c)
}

/// Varga (stop and nasal) consonants क..म: before these a class nasal is
/// written, never an anusvāra.
fn is_varga_consonant(c: char) -> bool {
    ('\u{0915}'..='\u{092E}').contains(&c)
}

/// Split text into Devanagari words; an underscore between Devanagari letters
/// stays inside the word so the underscore rule can see it.
pub fn devanagari_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if is_devanagari_letter(c) || (c == '_' && !current.is_empty()) {
            current.push(c);
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
        .into_iter()
        .map(|w| w.trim_end_matches('_').to_string())
        .filter(|w| w.chars().any(is_devanagari_letter))
        .collect()
}

/// Check one Devanagari word against the denied-word table and the
/// orthography rules.
pub fn check_word(word: &str) -> Vec<LexiconFinding> {
    let mut findings = Vec::new();
    for denied in denied_words() {
        if word.contains(denied.word.as_str()) {
            findings.push(LexiconFinding {
                rule: LexiconRule::DeniedWord,
                word: word.to_string(),
                message: format!("`{}` is not Sanskrit here ({}); use `{}`", denied.word, denied.why, denied.replacement),
            });
        }
    }
    let chars: Vec<char> = word.chars().collect();
    if chars.last() == Some(&ANUSVARA) {
        findings.push(LexiconFinding {
            rule: LexiconRule::FinalAnusvara,
            word: word.to_string(),
            message: format!("`{}` ends in an anusvāra; write `म्`", word),
        });
    }
    if chars.windows(2).any(|w| w[0] == ANUSVARA && is_varga_consonant(w[1])) {
        findings.push(LexiconFinding {
            rule: LexiconRule::AnusvaraBeforeStop,
            word: word.to_string(),
            message: format!("`{}` has an anusvāra before a stop; write the class nasal (ङ् ञ् ण् न् म्)", word),
        });
    }
    if chars.contains(&CHANDRABINDU) {
        findings.push(LexiconFinding {
            rule: LexiconRule::Chandrabindu,
            word: word.to_string(),
            message: format!("`{}` contains a chandrabindu, which is Hindi orthography", word),
        });
    }
    if chars.iter().any(|&c| c == NUKTA || ('\u{0958}'..='\u{095F}').contains(&c)) {
        findings.push(LexiconFinding {
            rule: LexiconRule::Nukta,
            word: word.to_string(),
            message: format!("`{}` contains a nukta letter, which marks a foreign (Hindi/Urdu/English) word", word),
        });
    }
    if chars.contains(&'_') {
        findings.push(LexiconFinding {
            rule: LexiconRule::Underscore,
            word: word.to_string(),
            message: format!("`{}` contains `_`; write a compound", word),
        });
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(word: &str) -> Vec<LexiconRule> {
        check_word(word).into_iter().map(|f| f.rule).collect()
    }

    #[test]
    fn accepts_standard_spellings() {
        for w in ["सङ्ख्या", "क्रमाङ्कः", "संस्कृतम्", "संयोगः", "सम्यक्", "माध्यम्", "चलच्चित्राणि"] {
            assert!(check_word(w).is_empty(), "{} should be clean: {:?}", w, check_word(w));
        }
    }

    #[test]
    fn rejects_each_rule() {
        assert_eq!(rules("सरासरी"), vec![LexiconRule::DeniedWord]); // lexicon-allow
        assert_eq!(rules("विशिष्टानां"), vec![LexiconRule::FinalAnusvara]);
        assert_eq!(rules("संख्या"), vec![LexiconRule::AnusvaraBeforeStop]);
        assert_eq!(rules("माँ"), vec![LexiconRule::Chandrabindu]);
        assert_eq!(rules("पिज़्ज़ा"), vec![LexiconRule::Nukta]);
        assert_eq!(rules("पूर्व_वेतनम्"), vec![LexiconRule::Underscore]);
    }

    #[test]
    fn splits_words_keeping_inner_underscores() {
        assert_eq!(devanagari_words("x पूर्व_वेतनम् इति १२ ।"), vec!["पूर्व_वेतनम्", "इति"]);
    }
}
