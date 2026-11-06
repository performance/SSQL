#!/usr/bin/env python3
"""Bring the VS Code grammar's word lists in step with the compiler's own
vocabulary (`ssql_compiler vocabulary`).

Each highlighting category keeps only words SSQL has; every vocabulary word
no category lists is highlighted by one catch-all pattern, so nothing the
language has goes unhighlighted. A frame bound written as one word
(`वर्तमानपङ्क्तिपर्यन्तम्`) stays, since the lexer reads it as two SSQL words.
`tests/vocabulary_test.rs` fails when the grammar and the vocabulary differ.

Usage: uv run python3 scripts/sync_editor_vocabulary.py
"""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
COMPILER = REPO / "target" / "release" / "ssql_compiler"
GRAMMAR = REPO / "editors" / "vscode" / "syntaxes" / "ssql.tmLanguage.json"
BOUNDED = re.compile(r"^(\(\?:\^\|\(\?<=\[\\s\(\),।\]\)\)\()(.*)(\)\(\?=\[\\s\(\),।\]\|\$\))$")
PREFIX = "(?:^|(?<=[\\s(),।]))("
SUFFIX = ")(?=[\\s(),।]|$)"


def vocabulary() -> list[str]:
    out = subprocess.run([str(COMPILER), "vocabulary"], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def is_word(word: str, vocab: set[str]) -> bool:
    return word in vocab or (word.endswith("पर्यन्तम्") and word[: -len("पर्यन्तम्")] in vocab)


def alternation(words: list[str]) -> str:
    # Longest first, so no alternative is shadowed by a prefix of itself.
    return PREFIX + "|".join(sorted(set(words), key=lambda w: (-len(w), w))) + SUFFIX


def main() -> int:
    vocab = set(vocabulary())
    grammar = json.loads(GRAMMAR.read_text(encoding="utf-8"))
    listed: set[str] = set()
    def filtered(pattern: dict) -> dict | None:
        """The pattern with only SSQL words, or None if none are left;
        patterns that are not word lists pass through unchanged."""
        m = BOUNDED.match(pattern.get("match", ""))
        if not m:
            return pattern
        if pattern.get("name") == "keyword.other.ssql":
            return None  # the catch-all is rebuilt below
        words = [w for w in m.group(2).split("|") if is_word(w, vocab)]
        if not words:
            return None
        listed.update(words)
        return {**pattern, "match": alternation(words)}

    for name, rule in list(grammar["repository"].items()):
        if "patterns" in rule:
            rule["patterns"] = [p for p in map(filtered, rule["patterns"]) if p is not None]
            empty = not rule["patterns"]
        else:
            new = filtered(rule)
            empty = new is None
            if new is not None:
                grammar["repository"][name] = new
        if empty:
            del grammar["repository"][name]
            grammar["patterns"] = [p for p in grammar["patterns"] if p.get("include") != f"#{name}"]
    rest = sorted(vocab - listed)
    if rest:
        grammar["repository"]["keywords"]["patterns"].append({"name": "keyword.other.ssql", "match": alternation(rest)})
    GRAMMAR.write_text(json.dumps(grammar, ensure_ascii=False, indent=4) + "\n", encoding="utf-8")
    print(f"{len(listed)} words in categories, {len(rest)} in the catch-all")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
