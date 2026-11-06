//! Ratchet over the lexicon gate: every Devanagari word in user-facing text
//! (examples, docs, README, website sources, editor grammar, compiler and LSP
//! sources including keyword tables and messages) must pass
//! `ssql_compiler::lexicon::check_word`.
//!
//! `tests/ratchets/lexicon.txt` records `file | rule | word | count` for the
//! findings present when the gate was introduced. The test fails on any
//! finding not in that list and on any listed finding that has gone away, so
//! the list can only shrink until it is empty and this ratchet is removed.
//! A source line that must name a denied word (a hint table) carries the
//! marker `lexicon-allow`; the marker is honoured only in .rs and .py files.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use ssql_compiler::lexicon::{check_word, devanagari_words, LexiconRule};

const RATCHET: &str = "tests/ratchets/lexicon.txt";

const ROOTS: &[&str] = &[
    "examples",
    "docs",
    "README.md",
    "src",
    "ssql-lsp/src",
    "ssql-py/src",
    "editors/vscode/src",
    "editors/vscode/syntaxes",
    "website/index.html",
    "website/app.js",
    "website/build_site.py",
    "website/ssql-wasm/src",
];
const EXTENSIONS: &[&str] = &["सं", "md", "rs", "py", "js", "ts", "html", "json"];
const SKIP_DIRS: &[&str] = &["target", "wsc2027", "papers", "node_modules", ".venv", "out", "__pycache__", "results", "generated_sql"];

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        out.push(path.to_path_buf());
        return;
    }
    let Ok(entries) = fs::read_dir(path) else { return };
    for entry in entries.flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if !SKIP_DIRS.contains(&name.as_str()) {
                collect(&p, out);
            }
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            // The lexicon's own table necessarily names every denied word.
            if EXTENSIONS.contains(&ext) && !p.ends_with("src/lexicon.json") {
                out.push(p);
            }
        }
    }
}

/// Remove the `.सं` file extension (a filename suffix, not a word) so
/// `schema.सं` is not read as the word `सं`.
fn strip_extension(line: &str) -> String {
    line.replace(".सं", ".")
}

/// Split a Markdown/HTML line into (text, is_code) runs: inline backtick spans
/// are code, everything else is prose. Fenced blocks are handled by the caller.
fn prose_and_code_runs(line: &str) -> Vec<(String, bool)> {
    line.split('`').enumerate().map(|(i, part)| (part.to_string(), i % 2 == 1)).collect()
}

fn starts_with_consonant(word: &str) -> bool {
    word.chars().next().map_or(false, |c| ('\u{0915}'..='\u{0939}').contains(&c))
}

/// Check the Devanagari words of one run. In Sanskrit prose a final anusvāra
/// is the correct sandhi of `म्` before a consonant (8.3.23 मोऽनुस्वारः), so
/// it is accepted there; SSQL code is written in pausal forms and never is.
fn check_run(text: &str, prose: bool, mut record: impl FnMut(&str, LexiconRule)) {
    let words = devanagari_words(text);
    for (i, word) in words.iter().enumerate() {
        for f in check_word(word) {
            if prose
                && f.rule == LexiconRule::FinalAnusvara
                && words.get(i + 1).map_or(false, |next| starts_with_consonant(next))
            {
                continue;
            }
            record(word, f.rule);
        }
    }
}

fn current_findings() -> BTreeSet<String> {
    let mut files = Vec::new();
    for root in ROOTS {
        collect(Path::new(root), &mut files);
    }
    let mut counts: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for file in files {
        let Ok(text) = fs::read_to_string(&file) else { continue };
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        let code = matches!(ext, "rs" | "py");
        let has_prose = matches!(ext, "md" | "html");
        let mut in_fence = false;
        for raw_line in text.lines() {
            if code && raw_line.contains("lexicon-allow") {
                continue;
            }
            if has_prose && raw_line.trim_start().starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            let line = strip_extension(raw_line);
            let mut record = |word: &str, rule: LexiconRule| {
                *counts
                    .entry((file.display().to_string(), format!("{:?}", rule), word.to_string()))
                    .or_default() += 1;
            };
            if has_prose && !in_fence {
                for (run, is_code) in prose_and_code_runs(&line) {
                    check_run(&run, !is_code, &mut record);
                }
            } else {
                check_run(&line, false, &mut record);
            }
        }
    }
    counts
        .into_iter()
        .map(|((file, rule, word), n)| format!("{} | {} | {} | {}", file, rule, word, n))
        .collect()
}

#[test]
fn lexicon_only_improves() {
    let current = current_findings();
    let recorded: BTreeSet<String> = fs::read_to_string(RATCHET)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect();
    if std::env::var("SSQL_WRITE_LEXICON_RATCHET").is_ok() {
        // Rewriting may only drop findings or lower their counts; a new or
        // increased finding is fixed in the source, not recorded (unless
        // SSQL_RATCHET_ALLOW_NEW says this rewrite is deliberate).
        let count_of = |set: &BTreeSet<String>, key: &str| -> Option<usize> {
            set.iter().find_map(|l| l.rsplit_once(" | ").filter(|(k, _)| *k == key).and_then(|(_, n)| n.parse().ok()))
        };
        let grown: Vec<&String> = current.iter().filter(|l| {
            let (key, n) = l.rsplit_once(" | ").map(|(k, n)| (k, n.parse::<usize>().unwrap_or(usize::MAX))).unwrap_or((l.as_str(), usize::MAX));
            count_of(&recorded, key).map_or(true, |old| n > old)
        }).collect();
        if !grown.is_empty() && std::env::var("SSQL_RATCHET_ALLOW_NEW").is_err() {
            panic!("refusing to record new or increased lexicon findings (fix them instead):\n{}", grown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
        }
        let body: Vec<&str> = current.iter().map(|s| s.as_str()).collect();
        fs::write(RATCHET, body.join("\n") + "\n").unwrap();
        return;
    }
    let new: Vec<&String> = current.difference(&recorded).collect();
    let fixed: Vec<&String> = recorded.difference(&current).collect();
    assert!(
        new.is_empty() && fixed.is_empty(),
        "lexicon ratchet out of date.\nNEW or increased findings (use the native word / standard spelling):\n{}\nFIXED or reduced findings (update these lines in {}):\n{}",
        new.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"),
        RATCHET,
        fixed.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"),
    );
}
