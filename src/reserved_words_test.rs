//! Data-driven tests asserting that builtin function names and grammar
//! keywords cannot be used as a table/column/view/CTE stem or as a
//! query-level alias.
//!
//! Builtins are enforced via `reserved_words::is_reserved_builtin` (wired
//! into `parse_stem_gender_vachana` and the select-item alias parser).
//! Grammar keywords are tokenized as dedicated `Token` variants rather than
//! generic identifiers, preventing them from matching stem or alias positions.

#[cfg(test)]
mod tests {
    use crate::lexer::tokenize;
    use crate::ssql_parser::{convert_vibhakti_tokens, parse};
    use crate::annotator::Annotator;
    use crate::schema::Schema;
    use crate::reserved_words::BUILTIN_FUNCTION_WORDS;
    use crate::ssql_parser::token::ALL_KEYWORD_STRINGS;

    /// A word is rejected if it can't even make it through tokenize+annotate+
    /// parse when used as the explicit stem of a minimal table declaration.
    fn stem_is_rejected(word: &str) -> bool {
        let ssql = format!(
            r#"पुस्तकानि इति पटलम् "books" इति आख्या {} इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
                सङ्ख्या इति पूर्णाङ्कः "id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम्
            इति पटलम् ।"#,
            word
        );
        let tokens = match tokenize(&ssql) {
            Ok(t) => t,
            Err(_) => return true,
        };
        let mut annotator = match Annotator::new() {
            Ok(a) => a,
            Err(_) => return true,
        };
        let enriched = match annotator.annotate(tokens, &Schema::default()) {
            Ok(e) => e,
            Err(_) => return true,
        };
        let parser_tokens = convert_vibhakti_tokens(enriched);
        parse(parser_tokens).is_err()
    }

    /// The declaration template itself must compile with an ordinary stem,
    /// or every word above would count as "rejected" and the tests below
    /// could never fail.
    #[test]
    fn an_ordinary_stem_is_accepted() {
        assert!(!stem_is_rejected("पुस्तक"), "the stem-rejection template no longer parses");
    }

    #[test]
    fn builtin_function_names_cannot_be_used_as_a_stem() {
        let mut failures = Vec::new();
        for word in BUILTIN_FUNCTION_WORDS {
            if !stem_is_rejected(word) {
                failures.push(*word);
            }
        }
        assert!(
            failures.is_empty(),
            "these builtin function names were incorrectly accepted as a stem: {:?}",
            failures
        );
    }

    #[test]
    fn keywords_cannot_be_used_as_a_stem() {
        let mut failures = Vec::new();
        for word in ALL_KEYWORD_STRINGS {
            if !stem_is_rejected(word) {
                failures.push(*word);
            }
        }
        assert!(
            failures.is_empty(),
            "these keywords were incorrectly accepted as a stem (structural protection gap): {:?}",
            failures
        );
    }

    /// A real collision between an ordinary Sanskrit noun and a reserved
    /// word, pinned as a direct, named regression test - not just a member of
    /// the bulk loop above. विभाग ("department") was the NTILE keyword until
    /// NTILE became `<n> भागेषु भागाङ्कः`; it is an ordinary stem again.
    #[test]
    fn bahya_is_rejected_and_vibhaga_is_free() {
        assert!(stem_is_rejected("बाह्य"), "बाह्य should collide with the बाह्या सञ्ज्ञा parameter-declaration keyword");
        assert!(!stem_is_rejected("विभाग"), "विभाग (department) is no longer a keyword");
    }
}
