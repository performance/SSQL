#!/usr/bin/env python3
"""Timing benchmark for nested and window-function SSQL queries.
Times the compile step (--dialect sqlite, compile-only, no DB execution)
for a curated query list using the release binary.

Usage: uv run python3 scripts/perf_bench_nested_parens.py [--json out.json]
"""
import subprocess, os, sys, time, json, glob

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.path.join(REPO_ROOT, "target", "release", "ssql_compiler")

# Primary repro file.
PRIMARY = [
    "examples/20_safety_incident_logs/queries/q1_elegant.सं",
]

# Window-function files (regression-risk set - RANK/NTILE/PARTITION BY/
# running-total shapes). Excludes 2 DDL false positives (यत्र used as a
# CREATE-TABLE column-list marker, not a window spec).
WINDOW_FILES = [
    "examples/15_social_engagement/queries/q1_social.सं",
    "examples/16_influencer_moving_avg/queries/q1_elegant.सं",
    "examples/17_funnel_drop_off/queries/q1_elegant.सं",
    "examples/18_patient_readmission/queries/q1_elegant.सं",
    "examples/21_flight_delay_risk/queries/q1_elegant.सं",
    "examples/26_customer_cohorts/queries/q1_elegant.सं",
    "examples/30_server_resource_utilization/queries/q1_elegant.सं",
    "examples/coffee_sales/coffee_queries/running_total.सं",
    "examples/gurukula_management/SAMSQL/q41.सं",
    "examples/gurukula_management/SAMSQL/q42.सं",
    "examples/gurukula_management/SAMSQL/q43.सं",
    "examples/gurukula_management/SAMSQL/q44.सं",
    "examples/gurukula_management/SAMSQL/q45.सं",
    "examples/gurukula_management/SAMSQL/q46.सं",
    "examples/gurukula_management/SAMSQL/q47.सं",
    "examples/gurukula_management/SAMSQL/q48.सं",
    "examples/gurukula_management/SAMSQL/q49.सं",
    "examples/gurukula_management/SAMSQL/q50.सं",
    "examples/university_bookstore/SAMSQL/35_store_top_performer.सं",
    "examples/university_bookstore/SAMSQL/41_cte.सं",
    "examples/university_bookstore/SAMSQL/books_with_high_rating_low_reviews.सं",
    "examples/university_bookstore/SAMSQL/test_window.सं",
    "examples/university_bookstore/SAMSQL/test_window_funcs.सं",
    "examples/university_bookstore/SAMSQL/test_window_function.सं",
]

# CASE-heavy secondary stress files.
CASE_FILES = [
    "examples/university_bookstore/SAMSQL/20_churned_customers.सं",
    "examples/25_genomic_variant_frequencies/queries/q1_elegant.सं",
]

ALL_FILES = PRIMARY + WINDOW_FILES + CASE_FILES


def time_compile(path, timeout=35):
    out = "/tmp/perf_bench_31_out.sql"
    cmd = [BIN, "--dialect", "sqlite", "--output", out, path]
    start = time.perf_counter()
    try:
        res = subprocess.run(cmd, capture_output=True, text=True, cwd=REPO_ROOT, timeout=timeout)
        elapsed = time.perf_counter() - start
        return elapsed, res.returncode == 0, None
    except subprocess.TimeoutExpired:
        elapsed = time.perf_counter() - start
        return elapsed, False, "TIMEOUT"


def main():
    results = {}
    for rel in ALL_FILES:
        full = os.path.join(REPO_ROOT, rel)
        if not os.path.exists(full):
            print(f"MISSING: {rel}")
            continue
        elapsed, ok, err = time_compile(full)
        status = "OK" if ok else (err or "COMPILE_FAIL")
        results[rel] = {"seconds": round(elapsed, 3), "status": status}
        flag = "  <<<< SLOW" if elapsed > 3.0 else ""
        print(f"{elapsed:8.3f}s  {status:12s}  {rel}{flag}")

    if "--json" in sys.argv:
        idx = sys.argv.index("--json")
        outpath = sys.argv[idx + 1]
        with open(outpath, "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        print(f"\nWrote {outpath}")

    total = sum(r["seconds"] for r in results.values())
    print(f"\nTotal: {total:.2f}s across {len(results)} files")


if __name__ == "__main__":
    main()
