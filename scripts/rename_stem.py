#!/usr/bin/env python3
"""Rename a declared stem throughout an example project, form by form.

Renaming a stem is not a text substitution: `पिच्चा` (feminine आ-stem) to
`पिष्टक` (neuter अ-stem) changes every case ending (`पिच्चायाः` becomes
`पिष्टकस्य`). This tool asks the compiler's own morphology engine
(`ssql_compiler decline`) for every form of the old and the new stem, in the
singular and plural, and rewrites each old form to the new form of the same
case and number. A compound built on the old stem (`पिच्चासङ्ख्या`) keeps its
last member, which carries the inflection, so its prefix is replaced.

When one old spelling is two cases at once (`पिच्चायाः` is genitive and
ablative singular), the genitive is used - SSQL's common use - and the
choice is reported so it can be reviewed. Declaration gender words of the
renamed stem are updated.

Usage:
  uv run python3 scripts/rename_stem.py --root examples/pizza_sales \\
      --old पिच्चा --old-gender स्त्रीलिङ्गम् --new पिष्टक --new-gender नपुंसकलिङ्गम्
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
COMPILER = REPO / "target" / "release" / "ssql_compiler"
# Devanagari letters and signs, but not the dandas, which end a statement.
WORD = r"[ऀ-ॣ०-ॿ]"
# When an old spelling is several cases, prefer them in this order.
PREFERENCE = ["shashthi", "prathama", "panchami", "saptami", "tritiya", "dvitiya", "chaturthi"]


def decline(stem: str, gender: str, number: str) -> dict[str, str]:
    out = subprocess.run([str(COMPILER), "decline", stem, gender, number],
                         capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def form_map(old: str, old_gender: str, new: str, new_gender: str) -> tuple[dict[str, str], list[str]]:
    mapping: dict[str, str] = {}
    notes = []
    for number in ("एकवचनम्", "बहुवचनम्"):
        old_forms = decline(old, old_gender, number)
        new_forms = decline(new, new_gender, number)
        by_spelling: dict[str, list[str]] = {}
        for case, form in old_forms.items():
            by_spelling.setdefault(form, []).append(case)
        for form, cases in by_spelling.items():
            targets = {new_forms[c] for c in cases}
            chosen = min(cases, key=PREFERENCE.index)
            if len(targets) > 1:
                notes.append(f"`{form}` ({number}) is {', '.join(cases)}; using the {chosen} form `{new_forms[chosen]}`")
            if form in mapping and mapping[form] != new_forms[chosen]:
                notes.append(f"`{form}` is a form in both numbers; keeping `{mapping[form]}`")
                continue
            mapping[form] = new_forms[chosen]
    return mapping, notes


def rename_text(text: str, old: str, old_gender: str, new: str, new_gender: str, mapping: dict[str, str]) -> str:
    # The declaration's stem slot and gender word: `<old> इति <old_gender>`.
    text = re.sub(rf"(?<!{WORD}){re.escape(old)}(\s+इति\s+){re.escape(old_gender)}",
                  rf"{new}\g<1>{new_gender}", text)

    def word(m: re.Match) -> str:
        w = m.group(0)
        if w in mapping:
            return mapping[w]
        # A compound on the old stem: its last member carries the inflection.
        if w.startswith(old) and len(w) > len(old):
            return new + w[len(old):]
        return w

    return re.sub(rf"{WORD}+", word, text)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", required=True)
    ap.add_argument("--old", required=True)
    ap.add_argument("--old-gender", required=True)
    ap.add_argument("--new", required=True)
    ap.add_argument("--new-gender", required=True)
    args = ap.parse_args()

    mapping, notes = form_map(args.old, args.old_gender, args.new, args.new_gender)
    for n in notes:
        print("note:", n)
    changed = 0
    for path in sorted(Path(args.root).rglob("*")):
        if not path.is_file() or path.suffix not in (".सं", ".md") or "target" in path.parts:
            continue
        text = path.read_text(encoding="utf-8")
        new_text = rename_text(text, args.old, args.old_gender, args.new, args.new_gender, mapping)
        if new_text != text:
            path.write_text(new_text, encoding="utf-8")
            changed += 1
            print("renamed in", path)
    print(f"{changed} files changed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
