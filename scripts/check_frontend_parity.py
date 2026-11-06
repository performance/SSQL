#!/usr/bin/env python3
"""Every SSQL front-end must accept and reject the same language and emit the
same SQL: the CLI (the reference), the website's WebAssembly bundle and the
Python binding.

Cases: every example query with its project's schema.सं, every tutorial SSQL
block, every quiz solution and exercise solution, every featured playground
example, and a set of statements that must be rejected. Each case is compiled
by all three front-ends, in both dialects, with a fresh compiler per case; any
difference in accept/reject or in the SQL text fails.

Usage: uv run python3 scripts/check_frontend_parity.py
(requires target/release/ssql_compiler, website/pkg built, and `ssql` importable)
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import ssql

REPO = Path(__file__).resolve().parent.parent
COMPILER = REPO / "target" / "release" / "ssql_compiler"
BOOKSTORE_SCHEMA = REPO / "examples" / "university_bookstore" / "SAMSQL" / "schema.सं"

# Teaching material (tutorial blocks, quiz and exercise solutions, featured
# examples) must compile, except these, which are wrong on purpose.
EXPECTED_TO_FAIL = {
    "featured example concordance_err": "the Strict Grammar Validator Demo shows a gender and verb-number error",
}

# Statements no front-end may accept.
MUST_REJECT = [
    "लेखकेभ्यः नाम ।",                       # no verb
    "लेखकेभ्यः नाम आह्रियताम्",              # no closing ।
    "पुस्तकेभ्यः निष्कास्यताम् ।",            # DELETE with neither subject nor filter
    "पुस्तकेभ्यः मूल्यस्य सरासरी आह्रियताम् ।",  # non-Sanskrit AVG word  # lexicon-allow
    "पुस्तकेभ्यः सर्वम् आह्रियन्ताम् ।",        # verb number disagrees with सर्वम्
    "पुस्तकेभ्यः शीर्षकम् \"अ",               # unterminated string
    "पुस्तकेभ्यः शीर्षकम् आहर ।",              # removed SELECT synonym (active voice)
    "पुस्तकेभ्यः शीर्षकम् चियताम् ।",           # removed SELECT synonym
    "पुस्तकेषु सर्वत्र मूल्यम् ६० इति नवीक्रियताम् ।",  # removed UPDATE synonym
    "पुस्तकेषु मूल्यम् ६० इति परिवर्त्यताम् ।",         # UPDATE with neither a filter nor सर्वत्र
]


def example_cases() -> list[tuple[str, Path, str]]:
    """(label, schema path, source) for every example query."""
    cases = []
    for schema in sorted((REPO / "examples").rglob("schema.सं")):
        if "wsc2027" in schema.parts or "target" in schema.parts:
            continue
        root = schema.parent
        for q in sorted(root.rglob("*.सं")):
            if q.name in ("schema.सं",) or "target" in q.parts or "wsc2027" in q.parts:
                continue
            # A nested project's queries belong to that project's schema.
            nearest = next(p / "schema.सं" for p in q.parents if (p / "schema.सं").exists())
            if nearest != schema:
                continue
            cases.append((str(q.relative_to(REPO)), schema, q.read_text(encoding="utf-8")))
    return cases


def teaching_cases() -> list[tuple[str, Path, str]]:
    sys.path.insert(0, str(REPO))
    import verify_all  # noqa: E402  (reuses its tutorial block extraction)

    cases = []
    for md in sorted((REPO / "docs" / "tutorials").rglob("*.md")):
        for n, (ssql_blocks, _) in enumerate(verify_all.tutorial_sections(md.read_text(encoding="utf-8")), 1):
            for i, block in enumerate(ssql_blocks, 1):
                cases.append((f"{md.relative_to(REPO)} §{n} #{i}", BOOKSTORE_SCHEMA, block))
    quiz = json.loads((REPO / "docs" / "tutorials" / "quizzes" / "certification_quiz.json").read_text(encoding="utf-8"))
    for item in quiz:
        cases.append((f"quiz {item['id']}", BOOKSTORE_SCHEMA, item["solution_ssql"]))
    sys.path.insert(0, str(REPO / "website"))
    import build_site  # noqa: E402

    for chap, ex in build_site.TUTORIAL_EXERCISES.items():
        cases.append((f"exercise {chap}", BOOKSTORE_SCHEMA, ex["solution_ssql"]))
    app_js = (REPO / "website" / "app.js").read_text(encoding="utf-8")
    for key, src in re.findall(r"\n  ([a-z_]+): \{.*?\bssql:\s*`([^`]*)`", app_js, flags=re.S):
        cases.append((f"featured example {key}", BOOKSTORE_SCHEMA, src))
    for i, src in enumerate(MUST_REJECT, 1):
        cases.append((f"must-reject {i}", BOOKSTORE_SCHEMA, src))
    return cases


def compile_cli(schema: Path, source: str, dialect: str) -> tuple[bool, str]:
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / "schema.सं").write_text(schema.read_text(encoding="utf-8"), encoding="utf-8")
        q = Path(tmp) / "q.सं"
        q.write_text(source, encoding="utf-8")
        out = Path(tmp) / "q.sql"
        res = subprocess.run([str(COMPILER), "--dialect", dialect, "--output", str(out), str(q)],
                             capture_output=True, text=True, env={**os.environ, "SSQL_CACHE_DIR": tmp})
        if res.returncode != 0:
            return False, ""
        return True, out.read_text(encoding="utf-8")


def compile_python(schema: Path, source: str, dialect: str) -> tuple[bool | str, str]:
    try:
        return True, ssql.Compiler(schema.read_text(encoding="utf-8"), dialect).compile(source)
    except ssql.CompileError:
        return False, ""
    except BaseException as e:  # a Rust panic surfaces as pyo3's PanicException
        if type(e).__name__ != "PanicException":
            raise
        return "PANIC", ""


def compile_wasm(batch: list[dict]) -> list[dict]:
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False, encoding="utf-8") as f:
        json.dump(batch, f, ensure_ascii=False)
        path = f.name
    try:
        res = subprocess.run(["node", str(REPO / "scripts" / "parity_wasm_runner.mjs"), path],
                             capture_output=True, text=True, check=True)
        return json.loads(res.stdout)
    finally:
        os.unlink(path)


def main() -> int:
    cases = example_cases() + teaching_cases()
    batch, refs = [], []
    for label, schema, source in cases:
        for dialect in ("sqlite", "postgres"):
            batch.append({"schema": schema.read_text(encoding="utf-8"), "source": source, "dialect": dialect})
            refs.append((label, schema, source, dialect))
    wasm = compile_wasm(batch)

    failures = []
    reject_labels = {f"must-reject {i}" for i in range(1, len(MUST_REJECT) + 1)}
    for (label, schema, source, dialect), w in zip(refs, wasm):
        cli_ok, cli_sql = compile_cli(schema, source, dialect)
        py_ok, py_sql = compile_python(schema, source, dialect)
        if label in reject_labels and cli_ok:
            failures.append(f"[{dialect}] {label}: the CLI accepted a statement that must be rejected")
        is_teaching = not label.startswith(("must-reject", "examples/"))
        if is_teaching and not cli_ok and label not in EXPECTED_TO_FAIL:
            failures.append(f"[{dialect}] {label}: teaching material does not compile")
        if label in EXPECTED_TO_FAIL and cli_ok:
            failures.append(f"[{dialect}] {label}: expected to fail ({EXPECTED_TO_FAIL[label]}) but compiled")
        if py_ok == "PANIC":
            failures.append(f"[{dialect}] {label}: the Python binding panicked (a stale build? run `uv sync --reinstall-package ssql`)")
            continue
        if not (cli_ok == w["ok"] == py_ok):
            failures.append(f"[{dialect}] {label}: accepted by CLI={cli_ok} WASM={w['ok']} Python={py_ok}"
                            + (f" (WASM error: {w['errors'][:1]})" if not w["ok"] else ""))
        elif cli_ok and not (cli_sql == w["sql"] == py_sql):
            failures.append(f"[{dialect}] {label}: SQL differs between front-ends")
    for f in failures:
        print("❌", f)
    print(f"🔁 Front-end parity: {len(refs) - len(failures)}/{len(refs)} case-dialects agree "
          f"(CLI, WASM, Python){'' if not failures else f'; {len(failures)} differ'}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
