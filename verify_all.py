#!/usr/bin/env python3
import os
import sys
import glob
import json
import re
import sqlite3
import subprocess
import shutil
import time
import argparse
import csv
import io
from datetime import datetime

# Root Directories & Configuration
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
EXAMPLES_DIR = os.path.join(BASE_DIR, "examples")
TESTS_DIR = os.path.join(BASE_DIR, "tests")

PROGRESS_JSON = os.path.join(BASE_DIR, "verification_progress.json")
PROGRESS_MD = os.path.join(BASE_DIR, "verification_progress.md")

SQLITE_DB_DIR = os.path.join(TESTS_DIR, "db", "sqlite")
# Override with SSQL_PG_URL if your local Postgres role/database differs.
PG_DB_URL = os.environ.get("SSQL_PG_URL", "postgres://localhost/bookstore_pg")

COMPILER_BIN = os.path.join(BASE_DIR, "target", "release", "ssql_compiler")
if not os.path.exists(COMPILER_BIN):
    COMPILER_BIN = os.path.join(BASE_DIR, "target", "debug", "ssql_compiler")

# Test bind values for the corpus's few genuinely parameterized queries
# (SSQL's बाह्य/external-parameter syntax, compiled to a literal "?" per
# placeholder). Keyed by path relative to BASE_DIR, one entry per "?" in
# file order. This harness has no real application to supply these values
# at runtime, so it substitutes a plausible, hardcoded test value directly
# into the compiled SQL text before executing it - not real parameter
# binding (there's no driver-level bind step involved for either dialect),
# just enough to actually exercise these queries end-to-end instead of
# permanently skipping them. Only ever fabricate a value here, never treat
# a query as parameterized without one - see the "?" in
# compiled-SQL check below for the fallback (still skips silently for any
# future parameterized query that isn't listed here).
TEST_BIND_VALUES = {
    # university_bookstore/SAMSQL now has a real seed.सं (25 books) -
    # use values that actually match seeded rows, so these return a
    # genuine non-empty result rather than just "didn't error on an
    # empty table".
    "examples/university_bookstore/SAMSQL/03_book_details_by_isbn.सं": ["978-1234567890"],  # Data Structures and Algorithms
    "examples/university_bookstore/SAMSQL/test_external.सं": [50],  # matches 4 seeded books priced above $50
}

def format_sql_literal(value):
    if isinstance(value, str):
        return "'" + value.replace("'", "''") + "'"
    return str(value)

# Helper: Atomic JSON Write
def atomic_write_json(filepath, data):
    tmp_path = filepath + ".tmp"
    with open(tmp_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    os.replace(tmp_path, filepath)

# Helper: Render Live Progress Markdown Dashboard
def render_progress_md(state):
    completed = state["completed_steps"]
    total = state["total_steps"]
    pct = round((completed / total * 100), 1) if total > 0 else 0.0

    bar_len = 30
    filled_len = int(bar_len * completed // total) if total > 0 else 0
    bar = "█" * filled_len + "░" * (bar_len - filled_len)

    md_content = f"""# SSQL Native Incremental Verification Dashboard

**Last Updated:** `{state['last_updated']}`  
**Current Status:** `{state['status']}`  
**Target Engine:** `{state['current_dialect'].upper()}`  
**Active Project:** `{state['current_project']}`  
**Active Micro-Step:** `{state['current_step']}`  

## 📊 Overall Progress: {pct}%
```
[{bar}] {completed}/{total} steps ({pct}%)
```

| Metric | Count |
|---|---|
| ✅ **Passed Micro-Steps** | **{state['stats']['passed']}** |
| ❌ **Failed Micro-Steps** | **{state['stats']['failed']}** |
| ⚪ **Skipped Micro-Steps** | **{state['stats']['skipped']}** |
| 🔢 **Total Planned Steps** | **{total}** |

---

## 📜 Recent Micro-Step Trace Log

| Timestamp | Dialect | Project | Phase | Status | Details |
|---|---|---|---|---|---|
"""
    for entry in reversed(state["log_history"][-25:]):
        status_icon = "✅" if entry["status"] == "PASSED" else ("❌" if entry["status"] == "FAILED" else "ℹ️")
        details = entry.get("details", "") or ""
        if len(details) > 60:
            details = details[:57] + "..."
        md_content += f"| `{entry['timestamp']}` | `{entry['dialect']}` | `{entry['project']}` | `{entry['phase']}` | {status_icon} `{entry['status']}` | {details} |\n"

    with open(PROGRESS_MD, "w", encoding="utf-8") as f:
        f.write(md_content)

# Helper: Log Progress Step Atomically
def log_step(state, dialect, project, phase, step_name, status, details=None):
    # Committed progress logs shouldn't embed this machine's absolute path -
    # relativize anything rooted at BASE_DIR before it's ever written out.
    if details:
        details = details.replace(BASE_DIR + os.sep, "").replace(BASE_DIR, ".")
    now_str = datetime.now().isoformat()
    state["last_updated"] = now_str
    state["current_dialect"] = dialect
    state["current_project"] = project
    state["current_step"] = f"{phase}: {step_name}"
    
    if status == "PASSED":
        state["stats"]["passed"] += 1
        state["completed_steps"] += 1
    elif status == "FAILED":
        state["stats"]["failed"] += 1
        state["completed_steps"] += 1
    elif status == "SKIPPED":
        state["stats"]["skipped"] += 1
        state["completed_steps"] += 1

    entry = {
        "timestamp": now_str,
        "dialect": dialect,
        "project": project,
        "phase": phase,
        "step": step_name,
        "status": status,
        "details": details
    }
    state["log_history"].append(entry)
    
    atomic_write_json(PROGRESS_JSON, state)
    render_progress_md(state)
    
    status_symbol = "✅" if status == "PASSED" else ("❌" if status == "FAILED" else "⚪")
    print(f"[{dialect.upper()}] [{project}] {status_symbol} {phase} -> {step_name}", flush=True)

# Helper: Compile SSQL File to Target SQL
def compile_ssql_file(ssql_file, dialect, out_sql_path):
    env = os.environ.copy()
    temp_dir = os.path.join(TESTS_DIR, "cache", dialect, os.path.basename(ssql_file))
    os.makedirs(temp_dir, exist_ok=True)
    env["SSQL_CACHE_DIR"] = temp_dir

    cmd = [COMPILER_BIN, "--dialect", dialect, "--output", out_sql_path, ssql_file]
    try:
        res = subprocess.run(cmd, capture_output=True, text=True, env=env, timeout=30)
        if res.returncode != 0:
            return False, res.stderr + "\n" + res.stdout
        with open(out_sql_path, "r", encoding="utf-8") as f:
            calls = untranslated_function_calls(f.read())
        if calls:
            return False, (
                f"Compiled SQL calls untranslated Sanskrit function(s) {calls}: the compiler "
                "passed an unknown word through to SQL instead of rejecting it"
            )
        return True, out_sql_path
    except Exception as e:
        return False, str(e)


def strip_sql_literals(sql):
    """Blank out comments, string literals and quoted identifiers."""
    sql = re.sub(r"--[^\n]*", "", sql)
    sql = re.sub(r"'(?:[^']|'')*'", "''", sql)
    return re.sub(r'"(?:[^"]|"")*"', '""', sql)


DEVANAGARI_CALL = re.compile(r"([ऀ-ॿ]+)\s*\(")


def untranslated_function_calls(sql):
    """Sanskrit words used as SQL function names. Devanagari is legitimate in
    compiled SQL only as a string literal, an alias or a CTE name, never as a
    function: such a call means an unknown word slipped through (e.g.
    `माध्यम्(t1.price)`), which the database rejects at run time."""
    return sorted({m.group(1) for m in DEVANAGARI_CALL.finditer(strip_sql_literals(sql))})


def _normalise_value(value):
    """One cell as a dialect-independent string: SQLite returns typed values,
    psql --csv returns text; numbers compare by value (6 decimals), booleans
    as 1/0, NULL as the empty string (psql's CSV rendering)."""
    if value is None:
        return ""
    if isinstance(value, bool):
        return "1" if value else "0"
    text = str(value)
    if text in ("t", "f"):
        return "1" if text == "t" else "0"
    try:
        number = float(text)
    except ValueError:
        return text
    if number != number or number in (float("inf"), float("-inf")):
        return text
    rounded = round(number, 6)
    return str(int(rounded)) if rounded == int(rounded) else repr(rounded)


def has_top_level_order_by(sql):
    """Whether the outermost statement fixes row order. Parenthesised
    subqueries and window specs (`OVER (... ORDER BY ...)`) do not."""
    depth = 0
    body = strip_sql_literals(sql)
    for i, ch in enumerate(body):
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        elif depth == 0 and body[i:i + 8].upper() == "ORDER BY":
            return True
    return False


def normalise_rows(dialect, rows, sql):
    """Result rows as a list of string tuples comparable across dialects."""
    if dialect == "postgres":
        out = [tuple(_normalise_value(v if v != "" else None) for v in r) for r in rows]
    else:
        out = [tuple(_normalise_value(v) for v in r) for r in rows]
    return out if has_top_level_order_by(sql) else sorted(out)

# Helper: Reset & Initialize SQLite DB for Project
def reset_sqlite_db(project_name):
    os.makedirs(SQLITE_DB_DIR, exist_ok=True)
    db_path = os.path.join(SQLITE_DB_DIR, f"{project_name}.db")
    if os.path.exists(db_path):
        os.remove(db_path)
    return db_path

# Helper: Execute SQL on SQLite
def run_sql_sqlite(db_path, sql_file_or_text, is_file=True):
    try:
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        if is_file:
            with open(sql_file_or_text, "r", encoding="utf-8") as f:
                sql_script = f.read()
        else:
            sql_script = sql_file_or_text
            
        cursor.executescript(sql_script)
        conn.commit()
        conn.close()
        return True, None
    except Exception as e:
        return False, str(e)

# Helper: Query SQLite and return rows
#
# Two gaps in this harness's SQLite execution model that
# aren't SSQL compiler bugs (the generated SQL is correct in both cases):
#
# 1. SSQL's regex-match syntax (अनुरूपेण) compiles to SQLite's REGEXP
#    operator, which SQLite only supports if the host application registers
#    a matching function - a real deployment would need to do this anyway,
#    so this harness should too.
# 2. Some compiled query files are legitimately a sequence of multiple
#    statements (e.g. a DROP VIEW/CREATE TABLE/DROP TABLE cleanup-test
#    sequence), which cursor.execute() rejects outright. Falling back to
#    executescript() only when single-statement execute() specifically
#    reports that shape of failure keeps the common single-SELECT path
#    (needing fetchall()) completely unchanged.
def query_sqlite(db_path, sql_file):
    conn = None
    try:
        conn = sqlite3.connect(db_path)
        conn.create_function(
            "REGEXP", 2,
            lambda pattern, value: (re.search(pattern, value) is not None) if value is not None else None
        )
        cursor = conn.cursor()
        with open(sql_file, "r", encoding="utf-8") as f:
            sql = f.read()
        try:
            cursor.execute(sql)
            rows = cursor.fetchall()
        except sqlite3.ProgrammingError as e:
            if "you can only execute one statement at a time" in str(e).lower():
                cursor.executescript(sql)
                rows = None  # a script has no single result set to compare
            else:
                raise
        conn.commit()
        return True, rows
    except Exception as e:
        return False, str(e)
    finally:
        # Always release the file: a connection left open by a failed query
        # locked the database for every check that followed.
        if conn is not None:
            conn.close()

# Helper: Reset Postgres Schema for Project
def reset_postgres_db(project_name):
    cmd = ["psql", PG_DB_URL, "-c", "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"]
    try:
        res = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
        if res.returncode != 0:
            return False, res.stderr + res.stdout
        return True, None
    except Exception as e:
        return False, str(e)

# Helper: Execute SQL on Postgres
#
# `-v ON_ERROR_STOP=1` is required: by default `psql -f script.sql` exits 0
# even when a statement inside the script errors (it just prints the error
# to stderr and keeps going) - without this flag, every real SQL error on
# the postgres path was being silently treated as a pass.
def run_sql_postgres(sql_file):
    cmd = ["psql", PG_DB_URL, "-v", "ON_ERROR_STOP=1", "-f", sql_file]
    try:
        res = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        if res.returncode != 0:
            return False, res.stderr + res.stdout
        return True, None
    except Exception as e:
        return False, str(e)

# Helper: Query Postgres and return CSV rows
def query_postgres(sql_file):
    # -q drops command tags ("INSERT 0 1"), leaving only result rows.
    cmd = ["psql", PG_DB_URL, "-q", "-v", "ON_ERROR_STOP=1", "-f", sql_file, "--csv"]
    try:
        res = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        if res.returncode != 0:
            return False, res.stderr + res.stdout
        # Parse psql's CSV properly: stripping lines would delete leading
        # spaces from the first column, and values may contain newlines.
        rows = [r for r in csv.reader(io.StringIO(res.stdout)) if r]
        return True, rows[1:]  # drop the header row
    except Exception as e:
        return False, str(e)

# Result oracles
#
# Executing a query only proves the SQL runs, not that it computes the right
# thing. Two independent checks close that gap:
#
# 1. Cross-dialect agreement: the same SSQL query must return the same rows
#    on SQLite and Postgres (after normalise_rows). A disagreement is a codegen
#    bug in one dialect.
# 2. Golden results: tests/golden/results/<project>/<query>.json records the
#    rows each query returned when it was last reviewed. Any change in a
#    query's output fails until it is reviewed and re-recorded with
#    --bless-results (which records only results the dialects agree on).
#
# NONDETERMINISTIC_QUERIES lists the only queries whose output legitimately
# differs between runs or dialects, with the reason; they are executed but
# their rows are not compared.
GOLDEN_DIR = os.path.join(TESTS_DIR, "golden", "results")

# Independent references: hand-written SQL that implements a query's stated
# objective (reviewed against the objectives in website/build_site.py), kept
# only where it is a good golden for the query's result rows. Each must
# return the same rows as its SSQL query in that dialect; a reference whose
# query no longer exists fails verification.
REFERENCE_DIRS = {
    os.path.join("university_bookstore", "SAMSQL"): {
        "sqlite": os.path.join(EXAMPLES_DIR, "university_bookstore", "SQLite"),
        "postgres": os.path.join(EXAMPLES_DIR, "university_bookstore", "SQL_PG"),
    },
}


def reference_path(pname, qname, dialect):
    ref_dir = REFERENCE_DIRS.get(pname, {}).get(dialect)
    if not ref_dir:
        return None
    path = os.path.join(ref_dir, qname[: -len(".सं")] + ".sql")
    return path if os.path.isfile(path) else None


def rows_match(a_rows, a_sql, b_rows, b_sql):
    """Same result rows; order counts only when both sides fix it."""
    if has_top_level_order_by(a_sql) and has_top_level_order_by(b_sql):
        return a_rows == b_rows
    return sorted(a_rows) == sorted(b_rows)
ALL_RESULTS = {}  # (project, query file) -> {dialect: normalised rows}
BLESS_RESULTS = False
NONDETERMINISTIC_QUERIES = {
    "university_bookstore/SAMSQL/30_explain_query_plan.सं": "EXPLAIN output is each engine's own plan format",
    "university_bookstore/SAMSQL/45_timestamp_functions.सं": "returns the current time",
}


def golden_path(pname, qname):
    return os.path.join(GOLDEN_DIR, pname.replace(os.sep, "__"), qname + ".json")


def verify_result_oracles(state, dialects, compile_only):
    if compile_only:
        return
    for (pname, qname), by_dialect in sorted(ALL_RESULTS.items()):
        reason = NONDETERMINISTIC_QUERIES.get(f"{pname}/{qname}")
        if reason:
            log_step(state, "all", pname, "ORACLE", f"Results {qname}", "SKIPPED", reason)
            continue
        if "sqlite" in by_dialect and "postgres" in by_dialect:
            if by_dialect["sqlite"] != by_dialect["postgres"]:
                log_step(state, "all", pname, "ORACLE", f"Cross-dialect {qname}", "FAILED",
                         f"SQLite returned {by_dialect['sqlite'][:5]} but Postgres returned {by_dialect['postgres'][:5]}")
                continue
            log_step(state, "all", pname, "ORACLE", f"Cross-dialect {qname}", "PASSED", "SQLite and Postgres agree")
        rows = next(iter(by_dialect.values()))
        path = golden_path(pname, qname)
        if BLESS_RESULTS:
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "w", encoding="utf-8") as f:
                json.dump(rows, f, ensure_ascii=False, indent=0)
            log_step(state, "all", pname, "ORACLE", f"Golden {qname}", "PASSED", "Recorded (--bless-results)")
            continue
        if not os.path.isfile(path):
            log_step(state, "all", pname, "ORACLE", f"Golden {qname}", "FAILED",
                     f"No golden result at {os.path.relpath(path, BASE_DIR)}; review the output and run --bless-results")
            continue
        with open(path, "r", encoding="utf-8") as f:
            expected = [tuple(r) for r in json.load(f)]
        if expected != rows:
            log_step(state, "all", pname, "ORACLE", f"Golden {qname}", "FAILED",
                     f"Result changed: expected {expected[:5]}, got {rows[:5]}")
        else:
            log_step(state, "all", pname, "ORACLE", f"Golden {qname}", "PASSED", "Matches reviewed result")


# Discover All Projects in examples/
#
# Walks the ENTIRE examples/ tree (not just direct children of EXAMPLES_DIR),
# so projects nested one level deeper (examples/university_bookstore/SAMSQL,
# examples/gurukula_management/SAMSQL) are found too - a directory is a
# "project" purely by having a schema.सं, regardless of depth. Query files
# are looked for in a "queries" or "*_queries" subdirectory first (the two
# naming conventions actually used across the corpus - coffee_sales/pizza_
# sales/restaurant_orders use "{name}_queries", everything else "queries");
# if neither exists, falls back to any loose *.सं files sitting directly in
# the project directory (the flat, no-subdirectory layout used by both
# SAMSQL projects). seed.सं is optional - a project without one still gets
# discovered and DDL/DQL-compiled, just without a data-driven execute phase
# (see has_seed handling in run_incremental_verification).
ARTIFACT_DIRS = {"ast_dumps", "results", "src", "SQL_PG", "SQLite", "reviews", "db", "cache", "transpiled"}

def discover_projects():
    projects = []
    for root, dirs, files in os.walk(EXAMPLES_DIR):
        dirs.sort()
        dirs[:] = [d for d in dirs if d not in ARTIFACT_DIRS and d != "queries" and not d.endswith("_queries")]

        if "schema.सं" not in files:
            continue

        schema_file = os.path.join(root, "schema.सं")
        seed_file = os.path.join(root, "seed.सं")
        has_seed = os.path.isfile(seed_file)

        query_files = []
        for sub in sorted(os.listdir(root)):
            subpath = os.path.join(root, sub)
            if os.path.isdir(subpath) and (sub == "queries" or sub.endswith("_queries")):
                query_files.extend(sorted(glob.glob(os.path.join(subpath, "*.सं"))))

        if not query_files:
            # Flat layout: loose *.सं files directly in the project dir,
            # excluding the schema/seed files themselves.
            exclude = {"schema.सं", "seed.सं", "test_seed.सं"}
            query_files = sorted(
                f for f in glob.glob(os.path.join(root, "*.सं"))
                if os.path.basename(f) not in exclude
            )

        name = os.path.relpath(root, EXAMPLES_DIR)
        projects.append({
            "name": name,
            "dir": root,
            "schema": schema_file,
            "seed": seed_file if has_seed else None,
            "has_seed": has_seed,
            "queries": query_files
        })

    projects.sort(key=lambda p: p["name"])
    return projects

# Find every .सं file under examples/ that isn't accounted for by any
# discovered project (its own schema/seed/queries) - so files with no
# resolvable schema.सं at all (e.g. legacy single-file examples whose old
# setu.toml mapping was retired) are surfaced explicitly
# instead of silently vanishing from the report.
def find_orphan_files(projects):
    accounted = set()
    for p in projects:
        accounted.add(p["schema"])
        if p["seed"]:
            accounted.add(p["seed"])
        accounted.update(p["queries"])

    orphans = []
    for root, dirs, files in os.walk(EXAMPLES_DIR):
        dirs.sort()
        dirs[:] = [d for d in dirs if d not in ARTIFACT_DIRS]
        for f in sorted(files):
            if f.endswith(".सं"):
                full = os.path.join(root, f)
                if full not in accounted:
                    orphans.append(full)
    return sorted(orphans)

# Main Incremental Verification Runner
def run_incremental_verification(dialects, compile_only=False):
    projects = discover_projects()
    orphans = find_orphan_files(projects)

    # Calculate Total Steps
    # Per dialect:
    # 1 DB Clearance per project (1)
    # DDL Compile (1) + Execute (1)
    # Seed Compile (1) + Execute (1) - only if the project actually has a seed.सं
    # Queries: per query (Compile 1 + Execute/Verify 1)
    total_steps = 0
    for d in dialects:
        for p in projects:
            total_steps += 1 # DB_CLEARANCE
            total_steps += 2 # DDL compile + execute
            total_steps += 2 if p["has_seed"] else 1 # Seed compile + execute, or 1 skip marker
            total_steps += (len(p["queries"]) * 2) # Query compile + verify

    state = {
        "status": "RUNNING",
        "last_updated": datetime.now().isoformat(),
        "current_dialect": dialects[0],
        "current_project": projects[0]["name"] if projects else "",
        "current_step": "INITIALIZING",
        "completed_steps": 0,
        "total_steps": total_steps,
        "stats": {"passed": 0, "failed": 0, "skipped": 0},
        "log_history": [],
        "orphan_files": orphans
    }
    atomic_write_json(PROGRESS_JSON, state)
    render_progress_md(state)

    print(f"\n=======================================================", flush=True)
    print(f"🚀 Starting Granular SSQL Verification Suite ({total_steps} total steps, {len(projects)} projects)", flush=True)
    if orphans:
        print(f"⚠️  {len(orphans)} .सं file(s) found with no resolvable schema.सं - cannot be verified:", flush=True)
        for o in orphans:
            print(f"    - {os.path.relpath(o, EXAMPLES_DIR)}", flush=True)
    print(f"=======================================================\n", flush=True)

    for dialect in dialects:
        for proj in projects:
            pname = proj["name"]
            pname_safe = pname.replace(os.sep, "__")

            # Phase 1: DB Clearance
            if dialect == "sqlite":
                db_path = reset_sqlite_db(pname_safe)
                log_step(state, dialect, pname, "CLEARANCE", "Reset SQLite DB", "PASSED", f"Created fresh DB: {db_path}")
            else:
                ok_clr, err_clr = reset_postgres_db(pname_safe)
                if ok_clr:
                    log_step(state, dialect, pname, "CLEARANCE", "Reset Postgres Schema", "PASSED", "Cascade dropped and recreated public schema")
                else:
                    log_step(state, dialect, pname, "CLEARANCE", "Reset Postgres Schema", "FAILED", err_clr)
                    continue

            # Phase 2: DDL Compilation & Execution
            out_ddl_sql = os.path.join(TESTS_DIR, "transpiled", dialect, f"{pname_safe}_schema.sql")
            os.makedirs(os.path.dirname(out_ddl_sql), exist_ok=True)

            ok_ddl_comp, err_ddl_comp = compile_ssql_file(proj["schema"], dialect, out_ddl_sql)
            if ok_ddl_comp:
                log_step(state, dialect, pname, "DDL", "Compile schema.सं", "PASSED", f"Transpiled to {out_ddl_sql}")
            else:
                log_step(state, dialect, pname, "DDL", "Compile schema.सं", "FAILED", err_ddl_comp)
                continue

            if compile_only:
                log_step(state, dialect, pname, "DDL", "Execute schema.sql", "SKIPPED", "Compile-only mode")
            else:
                if dialect == "sqlite":
                    ok_ddl_exec, err_ddl_exec = run_sql_sqlite(db_path, out_ddl_sql, is_file=True)
                else:
                    ok_ddl_exec, err_ddl_exec = run_sql_postgres(out_ddl_sql)

                if ok_ddl_exec:
                    log_step(state, dialect, pname, "DDL", "Execute schema.sql", "PASSED", "Tables created successfully")
                else:
                    log_step(state, dialect, pname, "DDL", "Execute schema.sql", "FAILED", err_ddl_exec)
                    continue

            # Phase 3: Seed DML Compilation & Execution. A project with no
            # seed.सं (coffee_sales/pizza_sales/restaurant_orders, and
            # university_bookstore/SAMSQL, which only has a hand-written raw
            # .sql seed, never expressed in native SSQL) still gets its
            # queries compiled and executed below - just against the empty
            # schema created above, which still exercises real SQL execution
            # (catches invalid function/table/column names) even without
            # data to compare result sets against.
            if not proj["has_seed"]:
                log_step(state, dialect, pname, "SEED", "Compile seed.सं", "SKIPPED", "No seed.सं present for this project")
            else:
                out_seed_sql = os.path.join(TESTS_DIR, "transpiled", dialect, f"{pname_safe}_seed.sql")
                ok_seed_comp, err_seed_comp = compile_ssql_file(proj["seed"], dialect, out_seed_sql)
                if ok_seed_comp:
                    log_step(state, dialect, pname, "SEED", "Compile seed.सं", "PASSED", f"Transpiled to {out_seed_sql}")
                else:
                    log_step(state, dialect, pname, "SEED", "Compile seed.सं", "FAILED", err_seed_comp)
                    continue

            if compile_only or not proj["has_seed"]:
                reason = "Compile-only mode" if compile_only else "No seed.सं present for this project"
                log_step(state, dialect, pname, "SEED", "Execute seed.sql", "SKIPPED", reason)
            else:
                if dialect == "sqlite":
                    ok_seed_exec, err_seed_exec = run_sql_sqlite(db_path, out_seed_sql, is_file=True)
                else:
                    ok_seed_exec, err_seed_exec = run_sql_postgres(out_seed_sql)
                    
                if ok_seed_exec:
                    log_step(state, dialect, pname, "SEED", "Execute seed.sql", "PASSED", "Mock data inserted successfully")
                else:
                    log_step(state, dialect, pname, "SEED", "Execute seed.sql", "FAILED", err_seed_exec)
                    continue

            # Phase 4: Queries Compilation & Execution Validation
            #
            # Each query runs against a pristine copy of the seeded database
            # (SQLite: a fresh copy of the file; Postgres: inside BEGIN ...
            # ROLLBACK), so an INSERT/UPDATE/DELETE example never changes what
            # a later query sees and every result is independent of run order.
            if dialect == "sqlite" and not compile_only:
                template_db = db_path + ".template"
                shutil.copyfile(db_path, template_db)
            query_results = {}
            for qfile in proj["queries"]:
                qname = os.path.basename(qfile)
                out_q_sql = os.path.join(TESTS_DIR, "transpiled", dialect, f"{pname_safe}_{qname}.sql")
                
                ok_q_comp, err_q_comp = compile_ssql_file(qfile, dialect, out_q_sql)
                if ok_q_comp:
                    log_step(state, dialect, pname, "DQL", f"Compile {qname}", "PASSED", f"Transpiled to {out_q_sql}")
                else:
                    log_step(state, dialect, pname, "DQL", f"Compile {qname}", "FAILED", err_q_comp)
                    continue

                # A compiled query containing a "?"
                # placeholder (SSQL's बाह्य/external-parameter syntax) is a
                # parameterized template meant to be run by an application
                # supplying real bind values, not a standalone-executable
                # script. TEST_BIND_VALUES (top of file) supplies a
                # hardcoded test value per known parameterized query so it
                # can actually be exercised instead of only compile-checked;
                # any parameterized query NOT listed there still skips
                # execution rather than fabricating a value (which could
                # mask real bugs or "pass" on a lucky guess). Compilation is
                # still verified above regardless.
                with open(out_q_sql, "r", encoding="utf-8") as f:
                    q_sql_text = f.read()
                # SQLite's placeholder is `?`, PostgreSQL's `$1..$n`.
                is_parameterized = "?" in q_sql_text or re.search(r"\$\d+", q_sql_text) is not None
                rel_qfile = os.path.relpath(qfile, BASE_DIR)
                bind_values = TEST_BIND_VALUES.get(rel_qfile) if is_parameterized else None

                if compile_only:
                    log_step(state, dialect, pname, "DQL", f"Verify {qname}", "SKIPPED", "Compile-only mode")
                elif is_parameterized and bind_values is None:
                    log_step(state, dialect, pname, "DQL", f"Verify {qname}", "SKIPPED", "Parameterized query - requires application-supplied bind value(s)")
                else:
                    if bind_values is not None:
                        bound_sql = q_sql_text
                        for v in bind_values:
                            bound_sql = bound_sql.replace("?", format_sql_literal(v), 1)
                        # `$10` before `$1`, so a shorter number never eats a longer one.
                        for i in range(len(bind_values), 0, -1):
                            bound_sql = bound_sql.replace(f"${i}", format_sql_literal(bind_values[i - 1]))
                        exec_sql_path = out_q_sql + ".bound.sql"
                        with open(exec_sql_path, "w", encoding="utf-8") as bf:
                            bf.write(bound_sql)
                    else:
                        exec_sql_path = out_q_sql

                    if dialect == "sqlite":
                        shutil.copyfile(template_db, db_path)
                        ok_q_exec, res_rows = query_sqlite(db_path, exec_sql_path)
                    else:
                        isolated_path = exec_sql_path + ".isolated.sql"
                        with open(exec_sql_path, "r", encoding="utf-8") as src, open(isolated_path, "w", encoding="utf-8") as dst:
                            # The `;` ends the query even when its text has no terminator.
                            dst.write("BEGIN;\n" + src.read() + "\n;\nROLLBACK;\n")
                        ok_q_exec, res_rows = query_postgres(isolated_path)
                        
                    if ok_q_exec:
                        query_results[qname] = res_rows
                        bound_note = f" (bound: {bind_values})" if bind_values is not None else ""
                        if res_rows is None:
                            log_step(state, dialect, pname, "DQL", f"Verify {qname}", "PASSED", f"Executed multi-statement script{bound_note}")
                        else:
                            ALL_RESULTS.setdefault((pname, qname), {})[dialect] = normalise_rows(dialect, res_rows, q_sql_text)
                            log_step(state, dialect, pname, "DQL", f"Verify {qname}", "PASSED", f"Returned {len(res_rows)} rows{bound_note}")
                            ref = reference_path(pname, qname, dialect)
                            if ref:
                                with open(ref, "r", encoding="utf-8") as rf:
                                    ref_sql = rf.read()
                                # A parameterised reference gets the query's bind values
                                # (SQLite `?`, Postgres `$1..$n`).
                                for i, v in enumerate(bind_values or [], start=1):
                                    ref_sql = ref_sql.replace("?", format_sql_literal(v), 1) if dialect == "sqlite" \
                                        else ref_sql.replace(f"${i}", format_sql_literal(v))
                                bound_ref = out_q_sql + ".reference.bound.sql"
                                with open(bound_ref, "w", encoding="utf-8") as bf:
                                    bf.write(ref_sql)
                                if dialect == "sqlite":
                                    shutil.copyfile(template_db, db_path)
                                    ok_ref, ref_rows = query_sqlite(db_path, bound_ref)
                                else:
                                    iso_ref = out_q_sql + ".reference.sql"
                                    with open(iso_ref, "w", encoding="utf-8") as dst:
                                        dst.write("BEGIN;\n" + ref_sql + "\n;\nROLLBACK;\n")
                                    ok_ref, ref_rows = query_postgres(iso_ref)
                                rel_ref = os.path.relpath(ref, BASE_DIR)
                                if not ok_ref or ref_rows is None:
                                    log_step(state, dialect, pname, "REFERENCE", f"Reference {qname}", "FAILED", f"{rel_ref} failed to execute: {ref_rows}")
                                elif rows_match(normalise_rows(dialect, res_rows, q_sql_text), q_sql_text, normalise_rows(dialect, ref_rows, ref_sql), ref_sql):
                                    log_step(state, dialect, pname, "REFERENCE", f"Reference {qname}", "PASSED", f"Same rows as {rel_ref}")
                                else:
                                    log_step(state, dialect, pname, "REFERENCE", f"Reference {qname}", "FAILED",
                                             f"Different rows from {rel_ref}: SSQL {normalise_rows(dialect, res_rows, q_sql_text)[:5]} vs reference {normalise_rows(dialect, ref_rows, ref_sql)[:5]}")
                    else:
                        log_step(state, dialect, pname, "DQL", f"Verify {qname}", "FAILED", res_rows)

            # Compare Dual Query Result Sets (Elegant vs Parens)
            if not compile_only and "q1_elegant.सं" in query_results and "q1_parens.सं" in query_results:
                r1 = query_results["q1_elegant.सं"]
                r2 = query_results["q1_parens.सं"]
                if str(r1) == str(r2):
                    log_step(state, dialect, pname, "PARITY", "Compare Elegant vs Parens", "PASSED", "Result sets match 100%")
                else:
                    log_step(state, dialect, pname, "PARITY", "Compare Elegant vs Parens", "FAILED", f"Mismatch: {r1} vs {r2}")

    verify_result_oracles(state, dialects, compile_only)
    for pname, by_dialect in REFERENCE_DIRS.items():
        for dialect, ref_dir in by_dialect.items():
            for ref in sorted(os.listdir(ref_dir)):
                query = os.path.join(EXAMPLES_DIR, pname, ref[: -len(".sql")] + ".सं")
                if ref.endswith(".sql") and not os.path.isfile(query):
                    log_step(state, dialect, pname, "REFERENCE", f"Reference {ref}", "FAILED",
                             f"{os.path.relpath(os.path.join(ref_dir, ref), BASE_DIR)} has no query {os.path.relpath(query, BASE_DIR)}")

    state["status"] = "FINISHED"
    state["current_step"] = "COMPLETE"
    atomic_write_json(PROGRESS_JSON, state)
    render_progress_md(state)

    print(f"\n=======================================================", flush=True)
    print(f"🎉 Verification Complete! Summary: {state['stats']['passed']} Passed, {state['stats']['failed']} Failed, {state['stats']['skipped']} Skipped", flush=True)
    if orphans:
        print(f"⚠️  {len(orphans)} .सं file(s) had no resolvable schema.सं and were NOT verified (see list above / verification_progress.json's orphan_files)", flush=True)
    print(f"=======================================================\n", flush=True)
    return state["stats"]["failed"] == 0 and not orphans

def build_bookstore_eval_db(tmpdir, label):
    """Compile the bookstore schema.सं and seed.सं into a fresh SQLite
    database for executing teaching material. Any failure is fatal: grading
    against a half-built database would make every result meaningless."""
    bookstore_dir = os.path.join(EXAMPLES_DIR, "university_bookstore", "SAMSQL")
    schema_src = os.path.join(tmpdir, "schema.सं")
    shutil.copyfile(os.path.join(bookstore_dir, "schema.सं"), schema_src)
    db_path = os.path.join(tmpdir, f"{label}.db")
    schema_sql = os.path.join(tmpdir, f"{label}_schema.sql")
    ok, err = compile_ssql_file(schema_src, "sqlite", schema_sql)
    if not ok:
        return None, f"bookstore schema.सं failed to compile: {err}"
    ok, err = run_sql_sqlite(db_path, schema_sql, is_file=True)
    if not ok:
        return None, f"bookstore schema failed to execute: {err}"
    seed_src = os.path.join(bookstore_dir, "seed.सं")
    if os.path.isfile(seed_src):
        seed_sql = os.path.join(tmpdir, f"{label}_seed.sql")
        ok, err = compile_ssql_file(seed_src, "sqlite", seed_sql)
        if not ok:
            return None, f"bookstore seed.सं failed to compile: {err}"
        ok, err = run_sql_sqlite(db_path, seed_sql, is_file=True)
        if not ok:
            return None, f"bookstore seed failed to execute: {err}"
    return db_path, None


def is_ssql_query_block(lang, body):
    """A fenced block that is a complete SSQL statement (not prose, not SQL)."""
    if lang.lower() not in ("", "ssql", "samskrutam", "sanskrit"):
        return False
    if not re.search(r"[\u0900-\u097F]", body):
        return False
    if "SELECT " in body or "CREATE TABLE" in body or "cargo run" in body:
        return False
    # Any complete statement: a query, a definition, or a change (INSERT,
    # UPDATE, DELETE) - DML examples must compile and agree like any other.
    return any(verb in body for verb in ("आह्रिय", "रच्यताम्", "निवेश्य", "परिवर्त्य", "निष्कास्य"))


def tutorial_sections(text):
    """Split a tutorial into heading sections and return, per section, its
    SSQL query blocks and its ```sql blocks, each in document order."""
    sections = [([], [])]
    lang = None
    body = []
    indent = 0
    for line in text.splitlines():
        # A fence may be indented under a list item; its block is read
        # without that indentation.
        fence = line.lstrip().startswith("```")
        if lang is None and line.startswith("#"):
            sections.append(([], []))
        elif lang is None and fence:
            indent = len(line) - len(line.lstrip())
            lang = line.lstrip()[3:].strip()
            body = []
        elif lang is not None and fence:
            block = "\n".join(body) + "\n"
            if lang.lower() == "sql":
                sections[-1][1].append(block)
            elif is_ssql_query_block(lang, block):
                sections[-1][0].append(block)
            lang = None
        elif lang is not None:
            body.append(line[min(indent, len(line) - len(line.lstrip())):])
    return [s for s in sections if s[0]]


def verify_tutorial_snippets():
    """Every complete SSQL query block in docs/tutorials/ must compile in strict
    mode and execute against the seeded bookstore database; a block that the
    tutorial pairs with a preceding ```sql block must return the same rows as
    that SQL, so the tutorials cannot teach an SSQL form that means something
    other than what they claim."""
    import tempfile

    tutorials_dir = os.path.join(BASE_DIR, "docs", "tutorials")
    if not os.path.isdir(tutorials_dir):
        return True

    md_files = sorted(glob.glob(os.path.join(tutorials_dir, "**", "*.md"), recursive=True))
    checked = 0
    compared = 0
    failed = 0
    with tempfile.TemporaryDirectory() as tmpdir:
        template_db, err = build_bookstore_eval_db(tmpdir, "tutorial_template")
        if template_db is None:
            print(f"❌ [TUTORIAL] {err}", flush=True)
            return False
        work_db = os.path.join(tmpdir, "tutorial_work.db")

        for md_path in md_files:
            with open(md_path, "r", encoding="utf-8") as f:
                text = f.read()
            rel = os.path.relpath(md_path, BASE_DIR)
            for section_no, (ssql_blocks, sql_blocks) in enumerate(tutorial_sections(text), start=1):
                # Within one heading section the i-th SSQL example is the i-th
                # SQL example; if the counts differ the pairing is unknown, so
                # the blocks are executed but not compared.
                paired = len(ssql_blocks) == len(sql_blocks)
                for i, b in enumerate(ssql_blocks):
                    checked += 1
                    where = f"{rel} (section {section_no}, SSQL example {i + 1})"
                    qpath = os.path.join(tmpdir, f"tut_{checked}.सं")
                    out_sql = os.path.join(tmpdir, f"tut_{checked}.sql")
                    with open(qpath, "w", encoding="utf-8") as qf:
                        qf.write(b.strip() + "\n")
                    ok, err = compile_ssql_file(qpath, "sqlite", out_sql)
                    if not ok:
                        failed += 1
                        print(f"❌ [TUTORIAL] {where} failed strict compilation:\n{err}", flush=True)
                        continue
                    # A table definition illustrates the bookstore's own
                    # schema; it must compile, but re-creating a table the
                    # database already has would test nothing.
                    if "इति पटलम्" in b and "रच्यताम्" in b:
                        continue
                    shutil.copyfile(template_db, work_db)
                    ok, rows = query_sqlite(work_db, out_sql)
                    if not ok:
                        failed += 1
                        print(f"❌ [TUTORIAL] {where} compiled SQL failed to execute: {rows}", flush=True)
                        continue
                    if not paired or rows is None:
                        continue
                    ref_path = os.path.join(tmpdir, f"tut_{checked}_ref.sql")
                    with open(ref_path, "w", encoding="utf-8") as rf:
                        rf.write(sql_blocks[i])
                    shutil.copyfile(template_db, work_db)
                    ok_ref, ref_rows = query_sqlite(work_db, ref_path)
                    compared += 1
                    if not ok_ref:
                        failed += 1
                        print(f"❌ [TUTORIAL] {where}: the tutorial's own SQL fails to execute: {ref_rows}", flush=True)
                        continue
                    with open(out_sql, "r", encoding="utf-8") as f:
                        compiled_sql = f.read()
                    got = normalise_rows("sqlite", rows, compiled_sql)
                    want = normalise_rows("sqlite", ref_rows, sql_blocks[i])
                    if got != want:
                        failed += 1
                        print(f"❌ [TUTORIAL] {where} returns different rows than the SQL the tutorial says it is equivalent to:\n   SSQL: {got[:5]}\n   SQL:  {want[:5]}", flush=True)
    print(f"📚 Tutorial Snippets Verified: {checked - failed}/{checked} Passed ({failed} Failed; {compared} compared against their paired SQL)\n", flush=True)
    return failed == 0

def doc_block_schema_dir(md_rel):
    """The project whose schema a documentation block runs against: the
    nearest enclosing example project, else the University Bookstore."""
    parts = md_rel.split(os.sep)
    if parts[0] == "examples":
        d = os.path.dirname(os.path.join(BASE_DIR, md_rel))
        while d.startswith(EXAMPLES_DIR):
            if os.path.isfile(os.path.join(d, "schema.सं")):
                return d
            d = os.path.dirname(d)
    return os.path.join(EXAMPLES_DIR, "university_bookstore", "SAMSQL")


def verify_doc_blocks():
    """Every complete SSQL statement in the documentation outside the
    tutorials (GUIDE, PROJECT and README files, the reference docs, the
    Python package docs) must compile in strict mode against its project's
    schema, so a document cannot keep showing a form the language dropped.
    The papers under review are excluded."""
    import tempfile

    listed = subprocess.run(["git", "ls-files", "*.md"], cwd=BASE_DIR, capture_output=True, text=True).stdout.split()
    skip = ("docs/tutorials/", "docs/papers/", "process/", "tests/")
    checked = 0
    failed = 0
    with tempfile.TemporaryDirectory() as tmpdir:
        for rel in sorted(listed):
            if rel.startswith(skip):
                continue
            with open(os.path.join(BASE_DIR, rel), "r", encoding="utf-8") as f:
                text = f.read()
            blocks = [b for section in tutorial_sections(text) for b in section[0]]
            if not blocks:
                continue
            schema_dir = doc_block_schema_dir(rel)
            work = os.path.join(tmpdir, f"doc_{checked}")
            os.makedirs(work)
            shutil.copyfile(os.path.join(schema_dir, "schema.सं"), os.path.join(work, "schema.सं"))
            for i, b in enumerate(blocks, start=1):
                checked += 1
                qpath = os.path.join(work, f"block_{i}.सं")
                with open(qpath, "w", encoding="utf-8") as qf:
                    qf.write(b.strip() + "\n")
                ok, err = compile_ssql_file(qpath, "sqlite", qpath + ".sql")
                if not ok:
                    failed += 1
                    print(f"❌ [DOCS] {rel} (SSQL example {i}) failed strict compilation:\n{err}", flush=True)
    print(f"📄 Documentation Snippets Verified: {checked - failed}/{checked} Passed ({failed} Failed)\n", flush=True)
    return failed == 0

# SQL the importer (`--from-sql`) must carry over without changing its
# meaning: each is imported to SSQL, compiled back, and both are run on the
# seeded bookstore database; the rows must match. The thresholds sit on
# prices the seed has (22, 18.5), so a strict/non-strict confusion shows.
IMPORTER_ROUNDTRIP_SQL = [
    "SELECT title FROM books WHERE price >= 22",
    "SELECT title FROM books WHERE price <= 22",
    "SELECT title FROM books WHERE price > 22",
    "SELECT title FROM books WHERE price < 22",
    "SELECT title FROM books WHERE price = 22",
    "SELECT title FROM books WHERE price <> 22",
    "SELECT title FROM books WHERE title LIKE 'S%'",
    "SELECT title FROM books WHERE title NOT LIKE 'S%'",
    "SELECT title FROM books WHERE language IN ('Sanskrit', 'Hindi')",
    "SELECT title FROM books WHERE language IN ('Sanskrit', 'Hindi', 'English')",
    "SELECT title FROM books WHERE language NOT IN ('Sanskrit', 'Hindi')",
    "SELECT title FROM books WHERE price BETWEEN 18.5 AND 22",
    "SELECT title FROM books WHERE price NOT BETWEEN 18.5 AND 22",
    "SELECT title FROM books WHERE price BETWEEN 10 AND 20 AND language = 'Hindi'",
    "SELECT title FROM books WHERE price > 10 OR price < 5 AND language = 'Hindi'",
    "SELECT title FROM books WHERE (price > 10 OR price < 5) AND language = 'Hindi'",
    "SELECT title FROM books WHERE genre IS NULL",
    "SELECT title FROM books WHERE genre IS NOT NULL",
    "SELECT DISTINCT language FROM books",
    "SELECT language, COUNT(*) FROM books GROUP BY language HAVING COUNT(*) > 2",
    "SELECT title, price FROM books ORDER BY price DESC LIMIT 3",
    "SELECT customers.name, sales.total_amount FROM sales JOIN customers ON sales.customer_id = customers.id",
    "SELECT books.title, authors.name FROM books, authors WHERE books.author_id = authors.id",
    "SELECT title, price, RANK() OVER (PARTITION BY language ORDER BY price DESC) AS r FROM books",
    "SELECT title, CASE WHEN price > 30 THEN 'high' ELSE 'low' END AS band FROM books",
    "WITH cheap AS (SELECT title, price FROM books WHERE price < 20) SELECT title FROM cheap",
    "SELECT name FROM authors UNION SELECT name FROM customers",
    "SELECT title FROM books ORDER BY title LIMIT 5 OFFSET 2",
    # Found by the final review's parser pass: each imported to SSQL that
    # meant something else, or that did not compile.
    "SELECT title, CASE WHEN price < 20 THEN 'cheap' WHEN price < 50 THEN 'mid' ELSE 'dear' END AS band FROM books",
    "SELECT title FROM books WHERE isbn = '978''x'",
    "SELECT title, SUM(price) OVER (ORDER BY id ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) AS moving FROM books",
    "SELECT title FROM books WHERE LENGTH(title) > 10",
    "SELECT price * 2 + 3 FROM books",
    "SELECT (price + 2) * 3 FROM books",
    "SELECT price + 2 + 3 FROM books",
    # Compound operands of subtraction and division, by the result noun's
    # case (expert review, grouped operands).
    "SELECT price - publication_year - 1 FROM books",
    "SELECT (price + 2) - 1 FROM books",
    "SELECT price - (price + 2) FROM books",
    "SELECT (price + publication_year) / 2 FROM books",
    "SELECT price / (publication_year + 1) FROM books",
    "SELECT (price - 5) / 2 FROM books",
    "SELECT price / 2 / 3 FROM books",
    "SELECT price - price / 2 FROM books",
    "SELECT price / 2 - 5 FROM books",
    "SELECT price - (publication_year - 1) FROM books",
    "UPDATE books SET genre = NULL WHERE id = 1",
    # A BETWEEN after AND or OR keeps its own AND.
    "SELECT title FROM books WHERE language = 'English' AND price BETWEEN 10 AND 20",
    "SELECT title FROM books WHERE language = 'Hindi' OR price BETWEEN 10 AND 20",
    "SELECT title FROM books WHERE id IN (1, 2, 3) AND price BETWEEN 10 AND 20",
    "SELECT title FROM books WHERE price BETWEEN 1 AND 20 AND publication_year BETWEEN 2000 AND 2020",
    # An aggregate of arithmetic, by the result noun's genitive.
    "SELECT language, SUM(price - 1) FROM books GROUP BY language",
    "SELECT AVG(price * 2) FROM books",
    "SELECT MAX(price + 1) FROM books",
    "SELECT SUM(price / 2) FROM books",
    # A quotient as divisor.
    "SELECT price / (publication_year / 2) FROM books",
    # A negative constant is `-५`; a negative column follows `ऋणात्मक`.
    "SELECT title FROM books WHERE price > -5",
    "SELECT -price FROM books",
    "SELECT price - -5 FROM books",
    "SELECT price + -5 FROM books",
    "SELECT price / -2 FROM books",
    "UPDATE books SET price = -5 WHERE id = 1",
]


def verify_importer_roundtrip():
    """The importer must not change what a query means (see
    IMPORTER_ROUNDTRIP_SQL)."""
    import tempfile

    checked = 0
    failed = 0
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path, err = build_bookstore_eval_db(tmpdir, "importer_eval")
        if db_path is None:
            print(f"❌ [IMPORTER] {err}", flush=True)
            return False
        schema_path = os.path.join(tmpdir, "schema.सं")
        for i, sql in enumerate(IMPORTER_ROUNDTRIP_SQL, start=1):
            checked += 1
            sql_path = os.path.join(tmpdir, f"imp_{i}.sql")
            ssql_path = os.path.join(tmpdir, f"imp_{i}.सं")
            with open(sql_path, "w", encoding="utf-8") as f:
                f.write(sql + ";\n")
            res = subprocess.run([COMPILER_BIN, "--from-sql", sql_path, "--schema", schema_path, "-o", ssql_path],
                                 capture_output=True, text=True, timeout=30)
            if res.returncode != 0:
                failed += 1
                print(f"❌ [IMPORTER] `{sql}` failed to import:\n{res.stderr}{res.stdout}", flush=True)
                continue
            out_sql = ssql_path + ".sql"
            ok, err = compile_ssql_file(ssql_path, "sqlite", out_sql)
            if not ok:
                failed += 1
                with open(ssql_path, encoding="utf-8") as f:
                    print(f"❌ [IMPORTER] `{sql}` imported to SSQL that does not compile:\n{f.read()}\n{err}", flush=True)
                continue
            ok_a, rows_a = query_sqlite(db_path, sql_path)
            ok_b, rows_b = query_sqlite(db_path, out_sql)
            if not (ok_a and ok_b) or normalise_rows("sqlite", rows_a, sql) != normalise_rows("sqlite", rows_b, sql):
                failed += 1
                print(f"❌ [IMPORTER] `{sql}` changed meaning on import: SQL {rows_a} vs SSQL {rows_b}", flush=True)
    print(f"🔁 Importer Round Trip: {checked - failed}/{checked} Passed ({failed} Failed)\n", flush=True)
    return failed == 0

def verify_certification_quizzes():
    """Verify and auto-grade all certification quiz exercises in docs/tutorials/quizzes/certification_quiz.json."""
    import tempfile

    quiz_file = os.path.join(BASE_DIR, "docs", "tutorials", "quizzes", "certification_quiz.json")
    bookstore_dir = os.path.join(EXAMPLES_DIR, "university_bookstore", "SAMSQL")
    bookstore_schema = os.path.join(bookstore_dir, "schema.सं")
    bookstore_seed = os.path.join(bookstore_dir, "seed.सं")
    if not os.path.isfile(quiz_file) or not os.path.isfile(bookstore_schema):
        return True

    with open(quiz_file, "r", encoding="utf-8") as f:
        quizzes = json.load(f)

    passed = 0
    failed = 0
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path, err = build_bookstore_eval_db(tmpdir, "quiz_eval")
        if db_path is None:
            print(f"❌ [QUIZ] {err}", flush=True)
            return False

        for item in quizzes:
            qid = item["id"]
            sol_ssql = item["solution_ssql"]
            ref_sql = item["reference_sql"]
            q_ssql_path = os.path.join(tmpdir, f"{qid}.सं")
            q_sql_path = os.path.join(tmpdir, f"{qid}.sql")
            ref_sql_path = os.path.join(tmpdir, f"{qid}_ref.sql")
            with open(q_ssql_path, "w", encoding="utf-8") as qf:
                qf.write(sol_ssql + "\n")
            with open(ref_sql_path, "w", encoding="utf-8") as rf:
                rf.write(ref_sql + "\n")

            ok_c, err_c = compile_ssql_file(q_ssql_path, "sqlite", q_sql_path)
            if not ok_c:
                failed += 1
                print(f"❌ [QUIZ {qid}] SSQL solution failed strict compilation:\n{err_c}", flush=True)
                continue

            ok_q, ssql_rows = query_sqlite(db_path, q_sql_path)
            ok_r, ref_rows = query_sqlite(db_path, ref_sql_path)
            if not ok_q or not ok_r or ssql_rows != ref_rows:
                failed += 1
                print(f"❌ [QUIZ {qid}] Result mismatch: SSQL={ssql_rows} vs Ref={ref_rows}", flush=True)
            else:
                passed += 1

    print(f"🎓 Certification Quizzes Auto-Graded: {passed}/{len(quizzes)} Passed ({failed} Failed)\n", flush=True)
    return failed == 0

def main():
    parser = argparse.ArgumentParser(description="Native SSQL Granular Verification Suite")
    parser.add_argument("--dialect", choices=["sqlite", "postgres", "all"], default="all", help="Target database dialect")
    parser.add_argument("--compile-only", action="store_true", help="Only verify SSQL compilation without execution")
    parser.add_argument("--tutorials-only", action="store_true", help="Only verify tutorial snippets and certification quizzes")
    parser.add_argument("--bless-results", action="store_true", help="Record current query results as the reviewed golden results (only where SQLite and Postgres agree)")
    args = parser.parse_args()
    global BLESS_RESULTS
    BLESS_RESULTS = args.bless_results

    if args.tutorials_only:
        tut_ok = verify_tutorial_snippets()
        quiz_ok = verify_certification_quizzes()
        docs_ok = verify_doc_blocks()
        imp_ok = verify_importer_roundtrip()
        sys.exit(0 if (tut_ok and quiz_ok and docs_ok and imp_ok) else 1)

    dialects = ["sqlite", "postgres"] if args.dialect == "all" else [args.dialect]
    success = run_incremental_verification(dialects, compile_only=args.compile_only)
    tut_ok = verify_tutorial_snippets()
    quiz_ok = verify_certification_quizzes()
    docs_ok = verify_doc_blocks()
    imp_ok = verify_importer_roundtrip()
    if not (success and tut_ok and quiz_ok and docs_ok and imp_ok):
        sys.exit(1)

if __name__ == "__main__":
    main()

