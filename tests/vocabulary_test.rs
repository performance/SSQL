//! The editors' word lists come from, or are checked against, the compiler's
//! own vocabulary (`ssql_compiler::vocabulary`), so no editor offers,
//! highlights or explains a word SSQL does not have.

use std::collections::BTreeSet;
use std::path::Path;

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e))
}

fn is_devanagari_letter(c: char) -> bool {
    ('\u{0900}'..='\u{0963}').contains(&c) || ('\u{0970}'..='\u{097F}').contains(&c)
}

/// The Devanagari words (no digits) in `text`.
fn words(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cur = String::new();
    for c in text.chars() {
        if is_devanagari_letter(c) {
            cur.push(c);
        } else if !cur.is_empty() {
            out.insert(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.insert(cur);
    }
    out
}

/// The double-quoted strings in `text` that are a single Devanagari word.
fn quoted_words(text: &str) -> BTreeSet<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .filter(|s| !s.is_empty() && s.chars().all(is_devanagari_letter))
        .map(str::to_string)
        .collect()
}

fn in_vocabulary(word: &str, vocab: &BTreeSet<&str>) -> bool {
    // The lexer splits a frame bound `…पर्यन्तम्` into its two words.
    vocab.contains(word)
        || word.strip_suffix("पर्यन्तम्").map_or(false, |base| vocab.contains(base))
}

fn check(list: &str, found: &BTreeSet<String>, vocab: &BTreeSet<&str>) -> Vec<String> {
    found
        .iter()
        .filter(|w| !in_vocabulary(w, vocab))
        .map(|w| format!("{}: `{}` is not an SSQL word", list, w))
        .collect()
}

/// The text between `start` and the next `end` after it.
fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let s = text.find(start).unwrap_or_else(|| panic!("`{}` not found", start));
    let e = text[s..].find(end).map(|e| s + e).unwrap_or(text.len());
    &text[s..e]
}

#[test]
fn spelled_words_cover_the_parser() {
    // Every word the parser matches by spelling (`word_token(&[…])`) is in
    // SPELLED_WORDS, so it reaches the vocabulary.
    let spelled: BTreeSet<&str> = ssql_compiler::ssql_parser::token::SPELLED_WORDS.iter().copied().collect();
    let mut missing = Vec::new();
    for file in ["src/ssql_parser/ddl.rs", "src/ssql_parser/expr.rs", "src/ssql_parser/clauses.rs", "src/ssql_parser/dml.rs"] {
        let src = read(file);
        for (i, _) in src.match_indices("word_token(&[") {
            let list = between(&src[i..], "[", "]");
            for w in quoted_words(list) {
                if !spelled.contains(w.as_str()) {
                    missing.push(format!("{}: `{}`", file, w));
                }
            }
        }
    }
    assert!(missing.is_empty(), "add to SPELLED_WORDS:\n{}", missing.join("\n"));
}

#[test]
fn builtin_function_words_match_codegen() {
    // reserved_words::BUILTIN_FUNCTION_WORDS is the list of names that
    // `get_builtin_function` maps; the two must name the same words.
    let codegen = read("src/ast_codegen.rs");
    let arms = between(&codegen, "fn get_builtin_function", "_ => None");
    let mapped: BTreeSet<String> = arms
        .lines()
        .filter_map(|l| l.split("=>").next().filter(|_| l.contains("=>")))
        .flat_map(quoted_words)
        .collect();
    let listed: BTreeSet<String> = ssql_compiler::reserved_words::BUILTIN_FUNCTION_WORDS.iter().map(|s| s.to_string()).collect();
    let unlisted: Vec<_> = mapped.difference(&listed).collect();
    let unmapped: Vec<_> = listed.difference(&mapped).collect();
    assert!(
        unlisted.is_empty() && unmapped.is_empty(),
        "mapped but not in BUILTIN_FUNCTION_WORDS: {:?}\nlisted but not mapped: {:?}",
        unlisted, unmapped
    );
}

#[test]
fn editor_words_are_ssql_words() {
    let vocabulary = ssql_compiler::vocabulary();
    let vocab: BTreeSet<&str> = vocabulary.iter().copied().collect();
    let mut problems = Vec::new();

    let ext = read("editors/vscode/src/extension.ts");
    let tokens: BTreeSet<String> = ext
        .split("token: '")
        .skip(1)
        .filter_map(|s| s.split('\'').next())
        .flat_map(words)
        .collect();
    problems.extend(check("editors/vscode/src/extension.ts", &tokens, &vocab));

    let grammar = read("editors/vscode/syntaxes/ssql.tmLanguage.json");
    let highlighted: BTreeSet<String> = grammar
        .split("\"match\":")
        .skip(1)
        .filter_map(|s| s.split('\n').next())
        .flat_map(words)
        .collect();
    problems.extend(check("editors/vscode/syntaxes/ssql.tmLanguage.json", &highlighted, &vocab));
    let unhighlighted: Vec<_> = vocab.iter().filter(|w| !highlighted.contains(**w)).collect();
    if !unhighlighted.is_empty() {
        problems.push(format!("ssql.tmLanguage.json does not highlight: {:?}", unhighlighted));
    }

    let lsp = read("ssql-lsp/src/main.rs");
    let hover = between(&lsp, "fn describe_keyword_info", "_ => None");
    let hover_keys: BTreeSet<String> = hover
        .lines()
        .filter_map(|l| l.split("=>").next().filter(|_| l.contains("=> Some")))
        .flat_map(quoted_words)
        .collect();
    problems.extend(check("ssql-lsp/src/main.rs (hover)", &hover_keys, &vocab));

    let app = read("website/app.js");
    let chips: BTreeSet<String> = app
        .split("label: \"")
        .skip(1)
        .filter_map(|s| s.split('"').next())
        .flat_map(words)
        .collect();
    problems.extend(check("website/app.js (chips)", &chips, &vocab));

    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
