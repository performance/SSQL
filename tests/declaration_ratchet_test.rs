//! Ratchet over `declaration_check`: every table, column and CTE declaration in
//! the example corpus must name the nominative of its declared stem, gender
//! and number, and across the corpus one stem has one gender.
//!
//! `tests/ratchets/declarations.txt` listed the findings that existed when the
//! check was introduced; it is empty, and the per-declaration check is now a
//! compiler error (`Compiler::check_declarations`). This test keeps the
//! corpus-wide part, which no single compile can see: the list may only
//! shrink, and it stays empty.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use ssql_compiler::declaration_check::{check_declarations, declared_stems};

const RATCHET: &str = "tests/ratchets/declarations.txt";

fn corpus_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if !matches!(name.as_str(), "target" | "wsc2027" | "node_modules" | ".venv") {
                corpus_files(&path, out);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("सं") {
            out.push(path);
        }
    }
}

fn current_findings() -> BTreeSet<String> {
    let mut files = Vec::new();
    corpus_files(Path::new("examples"), &mut files);
    let mut lines = BTreeSet::new();
    // Across the corpus, one stem has one gender: `सङ्केत` cannot be
    // masculine in one project and neuter in another.
    let mut genders: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for file in files {
        let source = fs::read_to_string(&file).unwrap();
        for (stem, gender) in declared_stems(&source) {
            genders.entry(stem).or_default().insert(format!("{:?}", gender));
        }
        for f in check_declarations(&source) {
            lines.insert(format!(
                "{} | {:?} | {} | {} | {:?} | {:?} | {}",
                file.display(), f.kind, f.name, f.stem, f.gender, f.number, f.problem
            ));
        }
    }
    for (stem, set) in genders {
        if set.len() > 1 {
            lines.insert(format!("(corpus) | stem `{}` is declared with more than one gender: {:?}", stem, set));
        }
    }
    lines
}

#[test]
fn declarations_only_improve() {
    let current = current_findings();
    let recorded: BTreeSet<String> = fs::read_to_string(RATCHET)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect();
    if std::env::var("SSQL_WRITE_DECLARATION_RATCHET").is_ok() {
        // Rewriting may only drop findings; a new one is fixed in the
        // declaration, not recorded (unless SSQL_RATCHET_ALLOW_NEW says this
        // rewrite is deliberate, e.g. a reworded message).
        let new: Vec<&String> = current.difference(&recorded).collect();
        if !new.is_empty() && std::env::var("SSQL_RATCHET_ALLOW_NEW").is_err() {
            panic!("refusing to record new declaration findings (fix them instead):\n{}", new.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
        }
        let body: Vec<&str> = current.iter().map(|s| s.as_str()).collect();
        fs::write(RATCHET, body.join("\n") + "\n").unwrap();
        return;
    }
    let new: Vec<&String> = current.difference(&recorded).collect();
    let fixed: Vec<&String> = recorded.difference(&current).collect();
    assert!(
        new.is_empty() && fixed.is_empty(),
        "declaration ratchet out of date.\nNEW findings (fix the declaration):\n{}\nFIXED findings (delete these lines from {}):\n{}",
        new.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"),
        RATCHET,
        fixed.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"),
    );
}
