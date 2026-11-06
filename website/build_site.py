#!/usr/bin/env python3
"""Build-on-change generator for the SSQL Website & Playground.

Generates:
1. `website/data/corpus.json`: Every example project in `examples/` with its
   `schema.सं`, pre-compiled SQLite `setup_sql` (DDL + seed data), and all `.सं` queries.
2. `website/data/content.json`: The inline SSQL Constitution (`docs/reference/constitution.md`),
   all 10 tutorial chapters across Track A and Track B with interactive end-of-chapter
   exercises, and the downloadable release artifact manifest.
3. `website/downloads/*`: Up-to-date downloadable binaries and packages (`ssql-vscode-0.2.1.vsix`,
   `ssql-0.2.1-*.whl`, `ssql-wasm-0.2.1.tar.gz`, `ssql_compiler-0.2.1-macos-arm64.tar.gz`,
   and `ssql-lsp-0.2.1-macos-arm64.tar.gz`), rebuilt automatically whenever sources change.
"""

from __future__ import annotations

import hashlib
import io
import json
import os
import re
import shutil
import sqlite3
import subprocess
import tarfile
import zipfile
from pathlib import Path
from typing import Any

import ssql

WEBSITE_DIR = Path(__file__).resolve().parent
REPO_ROOT = WEBSITE_DIR.parent
EXAMPLES_DIR = REPO_ROOT / "examples"
TUTORIALS_DIR = REPO_ROOT / "docs" / "tutorials"
CONSTITUTION_PATH = REPO_ROOT / "docs" / "reference" / "constitution.md"
DATA_DIR = WEBSITE_DIR / "data"
DOWNLOADS_DIR = WEBSITE_DIR / "downloads"

VERSION = "0.2.1"


def _latest_mtime(paths: list[Path]) -> float:
    latest = 0.0
    for base in paths:
        if not base.exists():
            continue
        if base.is_file():
            latest = max(latest, base.stat().st_mtime)
        else:
            for root, dirs, files in os.walk(base):
                dirs[:] = [
                    d
                    for d in dirs
                    if d not in {"node_modules", "target", ".git", "__pycache__", ".venv"}
                ]
                for f in files:
                    p = Path(root) / f
                    try:
                        latest = max(latest, p.stat().st_mtime)
                    except OSError:
                        pass
    return latest


def _human_size(num_bytes: int) -> str:
    if num_bytes >= 1024 * 1024:
        return f"{num_bytes / (1024 * 1024):.2f} MB"
    if num_bytes >= 1024:
        return f"{num_bytes / 1024:.1f} KB"
    return f"{num_bytes} B"


def _sha256_full(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _sha256_short(path: Path) -> str:
    return _sha256_full(path)[:16]


PROJECT_TITLE_OVERRIDES: dict[str, str] = {
    "university_bookstore/SAMSQL": "University Bookstore",
    "gurukula_management/SAMSQL": "Gurukula School Management",
    "01_maven_toys_sales": "Toy Store Retail Sales",
    "03_us_candy_distributor": "US Candy Distributor",
    "07_restaurant_orders": "Restaurant Menu & Orders",
    "10_mavenflix_churn": "Streaming Subscription Churn",
    "14_marketing_roi": "Marketing ROI Analytics",
    "16_influencer_moving_avg": "Influencer Moving Average",
    "19_hr_org_chart": "HR Organization Chart",
    "ecommerce": "E-Commerce Storefront",
    "restaurant_orders": "Restaurant Order Combos",
    "simple": "Simple Inventory Starter",
}

UPPERCASE_WORDS = {"us", "hr", "roi", "id", "isbn", "cte", "udf", "sql", "ssql", "rfm", "abc", "crm", "sla", "kpi"}


def _humanize_words(words: list[str]) -> str:
    out: list[str] = []
    for w in words:
        if not w:
            continue
        low = w.lower()
        if low in UPPERCASE_WORDS:
            out.append(low.upper())
        elif low == "avg":
            out.append("Average")
        elif low == "calc":
            out.append("Calculation")
        elif low == "rep":
            out.append("Rep")
        elif low == "reps":
            out.append("Reps")
        else:
            out.append(w.capitalize())
    return " ".join(out)


def _human_project_title(project_id: str) -> str:
    if project_id in PROJECT_TITLE_OVERRIDES:
        return PROJECT_TITLE_OVERRIDES[project_id]
    base = project_id.replace("/SAMSQL", "").replace("/", " ")
    base = re.sub(r"^(?:project_)?\d+_", "", base)
    return _humanize_words(base.split("_"))


def _strip_parens_text(text: str) -> str:
    """Replace parenthesized phrases `(foo)` with `, foo` so titles and objectives never show parentheses."""
    out = re.sub(r"\s*\(([^)]+)\)", r", \1", text)
    out = re.sub(r",\s*,", ",", out)
    return out.strip()


GURUKULA_QUERY_TITLES: dict[str, str] = {
    "q01": "List the names and designations of all teachers",
    "q02": "Find all teachers with a salary greater than 70000",
    "q03": "Find students admitted after '2020-06-01'",
    "q04": "Find students born before '2005-01-01'",
    "q05": "List students sorted by fees descending",
    "q06": "Find the top 3 highest-paid teachers",
    "q07": "Find the 4th and 5th highest-paid teachers, LIMIT and OFFSET",
    "q08": "Find teachers who are Kulapati OR have salary > 80000",
    "q09": "Find students whose fees are strictly between 4500 and 5500, both bounds excluded, AND filter",
    "q10": "Find teachers whose names start with 'V', LIKE filter",
    "q11": "Class schedule details, classes with subject and teacher names",
    "q12": "Exam results with student name, subject name, and marks",
    "q13": "List room allotments with student names and room numbers",
    "q14": "Find all classes taught by teacher 'Vasishtha'",
    "q15": "Find exam scores of students in the subject 'Vyakarana'",
    "q16": "List all students and their room numbers using LEFT JOIN, some may not have rooms",
    "q17": "List all teachers and the classes they teach using LEFT JOIN",
    "q18": "List all subjects and their class times using LEFT JOIN",
    "q19": "Show the name, score and hostel room of each student who scored above 90",
    "q20": "Show classes taught by teachers earning more than 75000",
    "q21": "Find the total number of teachers",
    "q22": "Calculate the total salary paid to teachers",
    "q23": "Calculate the average fees of all students",
    "q24": "List room numbers and count of students in each room",
    "q25": "Find the average difficulty level of all subjects",
    "q26": "Find the maximum and minimum exam marks",
    "q27": "Find total amount of donations received",
    "q28": "Group hostels by room and find the average hostel fee where it is > 1600, HAVING",
    "q29": "Total amount donated by each donor",
    "q30": "Find count of classes per teacher",
    "q31": "Categorize teachers' salaries into High, Medium, Low",
    "q32": "Categorize student fees into Premium or Regular",
    "q33": "Categorize subject difficulty levels",
    "q34": "Categorize student marks into A, B, C",
    "q35": "Categorize hostel fees into Expensive or Cheap",
    "q36": "Categorize donations into Mega or Small",
    "q37": "Classify student stay details based on room number",
    "q38": "Map teacher designations to levels",
    "q39": "Group and map students' admission years to seniority categories",
    "q40": "Categorize subjects into Core vs Elective based on difficulty",
    "q41": "Rank students by fees descending using RANK",
    "q42": "Rank teachers by salary using RANK",
    "q43": "Calculate lead salary of teachers",
    "q44": "Calculate lag salary of teachers",
    "q45": "Calculate rolling sum of student fees",
    "q46": "Rank donations by amount",
    "q47": "Calculate running total of donations",
    "q48": "Rank students' exam marks within each subject, Partition By",
    "q49": "Calculate lead marks for students",
    "q50": "Calculate lag marks for students",
    "q51": "Self-join to find pairs of students paying the same fees, each pair once",
    "q52": "UNION of names of teachers and students",
    "q53": "EXCEPT to find students who have not taken any exams",
    "q54": "INTERSECT of students who scored > 90 and live in hostels",
    "q55": "Subquery to find students who pay more than the average student fee",
    "q56": "Subquery to find subjects with difficulty greater than average difficulty",
    "q57": "Self-join to find pairs of subjects with the same difficulty level, each pair once",
    "q58": "UNION of donor names and teacher names",
    "q59": "Self-join to find pairs of teachers earning the same salary, each pair once",
    "q60": "Subquery to find teachers earning more than the average teacher salary",
}

BOOKSTORE_QUERY_OBJECTIVES: dict[str, str] = {
    "01_books_published_after_2010": "Retrieve the title and publication year of all books published after 2010.",
    "02_authors_starting_with_k": "Find all authors whose name starts with K using string prefix matching.",
    "03_book_details_by_isbn": "Look up book details by an ISBN bind parameter.",
    "04_stores_in_mumbai": "List all bookstore locations situated in Mumbai.",
    "05_books_by_author_id": "Retrieve all books written by author ID 1.",
    "06_books_under_20_dollars": "Retrieve all books priced under 20 dollars.",
    "07_customers_by_email_domain": "Find all customers whose email address is at the example.org domain.",
    "08_sales_on_specific_date": "Retrieve all sales recorded on a specific date.",
    "09_purchase_history_arjun": "Retrieve the purchase history for customer Arjun.",
    "10_total_sales_per_store": "Count total sales transactions grouped by store.",
    "11_books_with_low_inventory": "List books whose inventory quantity falls below the restock threshold.",
    "12_revenue_per_store": "Aggregate total sales revenue grouped by store name.",
    "13_avg_price_by_language": "Compute the average book price grouped by publication language.",
    "14_top_customers_by_spending": "Rank the top customers by their cumulative spending across all sales.",
    "15_monthly_sales_report": "Report the number of sales and the revenue for each month of each year.",
    "16_books_with_no_sales": "Identify books in the catalog that have zero recorded sale items.",
    "17_running_total_sales": "Compute a chronological running total of sales amounts using a window function.",
    "18_most_expensive_book_per_language": "Find the most expensive book in each language, by ranking within the language.",
    "19_price_vs_publisher_avg": "Compare each book price against the average book price of its publisher.",
    "20_churned_customers": "Identify customers with no order since 1 November 2023, including those who never ordered.",
    "21_search_books_by_keyword": "Search book titles matching a keyword pattern.",
    "22_update_book_price": "Update the price of a book identified by its ISBN.",
    "23_above_average_rated_books": "Find books whose review ratings exceed the catalog average rating.",
    "24_insert_new_book": "Insert a new Sanskrit book record into the books table.",
    "25_high_value_inventory_stores": "Find stores whose total inventory value (price × copies) exceeds 20000.",
    "26_create_view_top_selling": "Create a view summarizing top selling books and total copies sold.",
    "27_create_view_customer_activity": "Create a view of every customer's order count and total spend, including customers with no orders.",
    "28_delete_inactive_customers": "Delete customers who have never placed an order.",
    "29_apply_discount_to_category": "Take 10% off the price of Sanskrit titles for a promotion.",
    "30_explain_query_plan": "Generate an EXPLAIN QUERY PLAN for a filtered query on sales.",
    "31_rep_revenue": "Calculate total sales revenue generated by each sales representative.",
    "32_commission_calc": "Compute sales representative commission from total sales revenue.",
    "33_managers_team": "Join sales representatives with their managers to list reporting pairs.",
    "34_zero_sales_reps": "Find sales representatives with zero recorded sales using a left join.",
    "35_store_top_performer": "Rank sales representatives within each store by revenue using a window function.",
    "36_above_average_price": "Retrieve books priced above the catalog average price.",
    "37_in_clause": "Filter books matching a set of languages using list membership.",
    "37b_in_clause_dual": "Filter books matching two languages using dual list membership.",
    "37c_not_in_clause": "Filter books whose language is not in a list (NOT IN): `… इत्येषु नास्ति`.",
    "38_case_expression": "Classify book prices into affordability tiers using a conditional expression.",
    "39_distinct": "Select distinct publication languages from the books catalog.",
    "40_devanagari_string": "Filter books using Devanagari string literals.",
    "41_b": "Query from a Common Table Expression over bookstore records.",
    "41_cte": "Define a Common Table Expression of copies sold per book and filter it.",
    "42_union": "Combine customer and author names using a UNION set operation.",
    "42a_union": "Combine customer, author and publisher names, keeping duplicates, using UNION ALL.",
    "43_between": "Filter books whose price falls within a numeric range.",
    "44_date_year_extract": "Extract the year and month components from sale dates.",
    "45_timestamp_functions": "Extract the hour and minute of a date column, which are 0, beside the current moment, date and time.",
    "46_intersect": "Find names that are both a customer's and an author's using INTERSECT.",
    "47_except": "Find customer names that are not author names using EXCEPT.",
    "48_distinct_aggregate": "Compute distinct count and distinct sum aggregates over books.",
    "49_create_view": "Create a database view over filtered bookstore records.",
    "50_drop_table": "Drop an obsolete table from the database schema.",
    "books_with_high_rating_low_reviews": "Find books with high average ratings and few total reviews.",
    "new_math_rules": "Compute arithmetic expressions over book prices and inventory quantities.",
    "test_chet": "Filter books priced above a threshold.",
    "test_comprehensive": "Combine CTEs, joins, filtering, grouping and a union in one comprehensive query.",
    "test_date_regex": "Show the weekday of each review and whether its comment matches a regular expression.",
    "test_external": "Filter books against an external bind parameter threshold.",
    "test_field_access": "Project qualified table columns across joined bookstore tables.",
    "test_having": "Filter grouped bookstore aggregates using a HAVING clause.",
    "test_natural_funcs": "Apply scalar string and numeric functions to bookstore columns.",
    "test_offset": "Paginate ordered book results using LIMIT and OFFSET.",
    "test_scalar_funcs": "Evaluate nested scalar functions over book titles and prices.",
    "test_window": "Compute a running total of sales amounts in a CTE using a window sum.",
    "test_window_funcs": "Evaluate analytical window functions across book partitions.",
    "test_window_function": "Compute partitioned window aggregates over bookstore records.",
}


def _clean_markdown_inline(text: str) -> str:
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    text = text.replace("`", "")
    text = re.sub(r"\s*—\s*", ": ", text)
    text = re.sub(r"\s+", " ", text).strip()
    return _strip_parens_text(text)


def _parse_project_metadata(root_path: Path, rel_id: str) -> dict[str, Any]:
    """Parse `PROJECT.md` (if present) for domain, difficulty, summary, and per-file objectives."""
    md_path = root_path / "PROJECT.md"
    if not md_path.exists():
        md_path = root_path.parent / "PROJECT.md"

    domain = "Relational Analytics"
    difficulty = "🟢 Basic"
    summary = ""
    file_objectives: dict[str, tuple[str, str]] = {}

    if md_path.exists():
        raw = md_path.read_text(encoding="utf-8")
        dom_m = re.search(
            r"\*\*Domain:\*\*\s*([^·\n]+?)(?:\s*·\s*\*\*(?:Difficulty|Kind):\*\*\s*([^\n]+))?$",
            raw,
             flags=re.MULTILINE,
        )
        if dom_m:
            domain = _clean_markdown_inline(dom_m.group(1))
            if dom_m.group(2):
                difficulty = _clean_markdown_inline(dom_m.group(2))

        prob_m = re.search(
            r"##\s+(?:Problem Description|What this is)\s*\n+([\s\S]*?)(?=\n##\s+|\Z)",
            raw,
        )
        if prob_m:
            first_para = prob_m.group(1).strip().split("\n\n")[0]
            summary = _clean_markdown_inline(first_para)

        # Parse numbered Objectives: `1. **Objective 1 — Short Title**: Full description`
        objectives_by_num: dict[str, tuple[str, str]] = {}
        obj_sec = re.search(r"##\s+Objectives\s*\n+([\s\S]*?)(?=\n##\s+|\Z)", raw)
        if obj_sec:
            obj_text = obj_sec.group(1)
            for m in re.finditer(
                r"^\s*\d+\.\s*\*\*Objective\s+(\d+)\s*(?:—|--|-|:)\s*([^*]+)\*\*:\s*([\s\S]*?)(?=^\s*\d+\.\s*\*\*Objective|\Z)",
                obj_text,
                flags=re.MULTILINE,
            ):
                num, short_title, desc = m.groups()
                objectives_by_num[num] = (
                    _clean_markdown_inline(short_title),
                    _clean_markdown_inline(desc),
                )

        # Parse `## Status` mapping `Objective N (filename.सं)` -> file stem
        status_sec = re.search(r"##\s+Status\s*\n+([\s\S]*?)(?=\n##\s+|\Z)", raw)
        if status_sec and objectives_by_num:
            for m in re.finditer(
                r"-\s*\*\*Objective\s+(\d+)\*\*\s*\(([^)]+)\)",
                status_sec.group(1),
            ):
                num, files_str = m.groups()
                if num in objectives_by_num:
                    for f_m in re.finditer(r"`?([a-zA-Z0-9_]+)\.सं`?", files_str):
                        stem = f_m.group(1)
                        if stem != "q1_parens":
                            file_objectives[stem] = objectives_by_num[num]

        # If there is 1 objective in `objectives_by_num` and `file_objectives` is still empty, store default `"__single__"`
        if len(objectives_by_num) == 1 and not file_objectives:
            file_objectives["__single__"] = next(iter(objectives_by_num.values()))

    if rel_id == "university_bookstore/SAMSQL":
        domain = "Retail • Bookstore"
        difficulty = "🧪 Full Verification Corpus"
        summary = (
            "University bookstore network managing books, authors, "
            "publishers, inventory, customers, sales, "
            "sales representatives, and book reviews."
        )
    elif rel_id == "gurukula_management/SAMSQL":
        domain = "Education • Gurukula"
        difficulty = "🧪 60 Query Suite"
        summary = (
            "Traditional Gurukula school management database tracking teachers, "
            "students, subjects, classes, exams, "
            "hostels, and donations across 60 progressive relational queries."
        )

    return {
        "domain": domain,
        "difficulty": difficulty,
        "summary": summary,
        "file_objectives": file_objectives,
    }


def _build_default_sample_sql(ddl_sql: str) -> str:
    """Synthesize a clean, runnable SELECT statement from the first user table in `ddl_sql`."""
    for m in re.finditer(
        r"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([a-zA-Z0-9_]+)\s*\(([\s\S]*?)\);",
        ddl_sql,
        flags=re.IGNORECASE,
    ):
        tbl_name = m.group(1)
        if tbl_name.startswith("cte_") or tbl_name == "top_selling_books":
            continue
        cols_block = m.group(2)
        cols: list[tuple[str, str]] = []
        for line in cols_block.splitlines():
            line_s = line.strip().rstrip(",")
            if not line_s or line_s.upper().startswith(("PRIMARY", "FOREIGN", "UNIQUE", "CHECK", "CONSTRAINT")):
                continue
            parts = line_s.split()
            if len(parts) >= 2:
                col_name = parts[0].strip('"`')
                col_type = parts[1].upper()
                # Skip non-ASCII scaffolding column names
                if col_name.isidentifier() and col_name.isascii():
                    cols.append((col_name, col_type))
        if cols:
            proj_cols = [c[0] for c in cols[: min(3, len(cols))]]
            order_col = proj_cols[0]
            return (
                f"SELECT {', '.join(proj_cols)}\n"
                f"FROM {tbl_name}\n"
                f"ORDER BY {order_col} ASC\n"
                f"LIMIT 5;"
            )
    return "SELECT title, price FROM books ORDER BY price DESC LIMIT 5;"


def _format_query_prefix(stem: str) -> str:
    """Format query stem into a clean numeric/q prefix without raw snake_case stems."""
    m = re.match(r"^([qQ]?\d+[a-z]?)(?:_.*)?$", stem)
    if m:
        return m.group(1)
    return ""


def _human_query_title(
    stem: str,
    rel_id: str = "",
    file_objectives: dict[str, tuple[str, str]] | None = None,
) -> tuple[str, str]:
    """Return `(short_title, full_objective)` for a query stem without parentheses."""
    if file_objectives:
        if stem in file_objectives:
            return file_objectives[stem]
        if "__single__" in file_objectives:
            return file_objectives["__single__"]

    if rel_id == "gurukula_management/SAMSQL" and stem in GURUKULA_QUERY_TITLES:
        desc = GURUKULA_QUERY_TITLES[stem]
        return desc, f"{desc}."

    if rel_id == "university_bookstore/SAMSQL" and stem in BOOKSTORE_QUERY_OBJECTIVES:
        obj = BOOKSTORE_QUERY_OBJECTIVES[stem]
        special_titles = {
            "41_b": "Common Table Expression Variant",
            "41_cte": "Common Table Expression",
            "37c_not_in_clause": "NOT IN Clause",
            "test_chet": "Conditional Branching",
            "new_math_rules": "Arithmetic Expressions",
        }
        if stem in special_titles:
            return special_titles[stem], obj
        cleaned = re.sub(r"^[qQ]?\d+[a-z]?_", "", stem)
        cleaned = re.sub(r"^test_", "", cleaned)
        short = _humanize_words(cleaned.split("_")) or stem
        return short, obj

    cleaned = re.sub(r"^[qQ]?\d+[a-z]?_", "", stem)
    cleaned = re.sub(r"_elegant$", "", cleaned)
    short = _humanize_words(cleaned.split("_")) or "Relational Query"
    return short, f"Execute {short} query against the {_human_project_title(rel_id)} schema."


def discover_example_projects() -> list[dict[str, Any]]:
    """Discover every SSQL example project and compile their SQLite setup SQL."""
    projects: list[dict[str, Any]] = []

    for root, dirs, files in os.walk(EXAMPLES_DIR):
        dirs.sort()
        if "schema.सं" not in files:
            continue
        root_path = Path(root)
        rel_id = str(root_path.relative_to(EXAMPLES_DIR))
        schema_path = root_path / "schema.सं"
        schema_ssql = schema_path.read_text(encoding="utf-8")

        proj_meta = _parse_project_metadata(root_path, rel_id)

        # Collect all query files across `queries/`, `*_queries/`, or flat `*.सं`
        query_paths: list[Path] = []
        for sub in sorted(os.listdir(root_path)):
            subpath = root_path / sub
            if subpath.is_dir() and (sub == "queries" or sub.endswith("_queries")):
                query_paths.extend(
                    sorted(p for p in subpath.glob("*.सं") if not p.stem.endswith("_parens"))
                )

        if not query_paths:
            exclude = {"schema.सं", "seed.सं", "test_seed.सं"}
            query_paths = sorted(
                p
                for p in root_path.glob("*.सं")
                if p.name not in exclude and not p.stem.endswith("_parens")
            )

        if not query_paths:
            if (root_path / "seed.सं").exists():
                query_paths = [root_path / "seed.सं"]
            else:
                query_paths = [schema_path]

        # The playground's database is exactly what verify_all checks: the
        # SQL compiled from schema.सं and seed.सं, nothing added by hand. A
        # project whose schema or seed fails to compile fails the build.
        setup_parts: list[str] = []
        compiler = ssql.Compiler(schema_ssql, "sqlite")
        ddl_sql = compiler.compile(schema_ssql)
        setup_parts.append(ddl_sql)
        for seed_name in ("seed.सं", "test_seed.सं"):
            seed_path = root_path / seed_name
            if seed_path.exists():
                setup_parts.append(compiler.compile(seed_path.read_text(encoding="utf-8")))
                break

        setup_sql = "\n\n".join(p.strip() for p in setup_parts if p.strip())
        default_sample_sql = _build_default_sample_sql(ddl_sql)

        queries: list[dict[str, Any]] = []
        for qf in query_paths:
            stem = qf.stem
            q_ssql = qf.read_text(encoding="utf-8")

            short_title, objective = _human_query_title(
                stem, rel_id, proj_meta["file_objectives"]
            )
            prefix = _format_query_prefix(stem)
            display_title = f"{prefix}: {short_title}" if prefix else short_title
            queries.append(
                {
                    "id": stem,
                    "filename": qf.name,
                    "title": display_title,
                    "objective": objective,
                    "ssql": q_ssql,
                    "sample_sql": default_sample_sql,
                }
            )

        seed_ssql = ""
        if (root_path / "seed.सं").exists():
            seed_ssql = (root_path / "seed.सं").read_text(encoding="utf-8")

        projects.append(
            {
                "id": rel_id,
                "title": _human_project_title(rel_id),
                "domain": proj_meta["domain"],
                "difficulty": proj_meta["difficulty"],
                "summary": proj_meta["summary"],
                "sample_sql": default_sample_sql,
                "schema_ssql": schema_ssql,
                "seed_ssql": seed_ssql,
                "setup_sql": setup_sql,
                "query_count": len(queries),
                "queries": queries,
            }
        )

    # Sort so university_bookstore/SAMSQL is first, then alphabetical
    projects.sort(
        key=lambda p: (0 if p["id"] == "university_bookstore/SAMSQL" else 1, p["id"])
    )

    return projects


def parse_constitution() -> dict[str, Any]:
    """Parse `docs/reference/constitution.md` into structured articles and full markdown."""
    raw = CONSTITUTION_PATH.read_text(encoding="utf-8").strip()
    lines = raw.splitlines()
    title = "The Samskrtam Programming Language: The v1.0 Design Principles (Constitution)"
    articles: list[dict[str, Any]] = []
    closing_lines: list[str] = []

    current_article: dict[str, Any] | None = None
    in_closing = False

    for line in lines:
        stripped = line.strip()
        if stripped.startswith("### **"):
            title = stripped.lstrip("#").strip().strip("*").strip()
            continue
        if stripped.startswith("#### **"):
            header = stripped.lstrip("#").strip().strip("*").strip()
            m = re.match(r"^([०-९]+)\.\s*`([^`]+)`\s*\(([^)]+)\)(?::\s*(.*))?$", header)
            if current_article is not None:
                current_article["body"] = "\n".join(current_article["body"]).strip()
                articles.append(current_article)
            if m:
                num, sanskrit, english, subtitle = m.groups()
                current_article = {
                    "number": num,
                    "sanskrit": sanskrit,
                    "english": english.strip(),
                    "subtitle": (subtitle or "").strip(),
                    "header": header,
                    "body": [],
                }
            else:
                current_article = {
                    "number": "",
                    "sanskrit": header,
                    "english": "",
                    "subtitle": "",
                    "header": header,
                    "body": [],
                }
            continue

        if stripped.startswith("This is our constitution."):
            in_closing = True

        if in_closing:
            closing_lines.append(stripped)
        elif current_article is not None:
            current_article["body"].append(line)

    if current_article is not None:
        current_article["body"] = "\n".join(current_article["body"]).strip()
        articles.append(current_article)

    return {
        "title": title,
        "markdown": raw,
        "articles": articles,
        "closing": "\n".join(closing_lines).strip(),
    }


TUTORIAL_EXERCISES: dict[str, dict[str, Any]] = {
    "track-a-01": {
        "title": "Exercise 1: Your First SSQL Query (एकवचनम्)",
        "prompt": "From the `पुस्तकानि` (`पुस्तकेभ्यः`) table in the University Bookstore, fetch 1 column: `शीर्षकम्` (`title`). Remember to use the singular verb `आह्रियताम्` and end with `।`.",
        "project_id": "university_bookstore/SAMSQL",
        "starter_ssql": "पुस्तकेभ्यः ",
        "hint": "Write `शीर्षकम् आह्रियताम् ।` after `पुस्तकेभ्यः` — 1 projected column requires the singular Karmaṇi Loṭ verb `आह्रियताम्`.",
        "solution_ssql": "पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।",
        "expected_columns": ["title"],
        "expected_min_rows": 5,
    },
    "track-a-02": {
        "title": "Exercise 2: Multi-Column SELECT & Aliasing (प्रथमा + इति)",
        "prompt": "From `पुस्तकानि` (`पुस्तकेभ्यः`), select 2 columns connected with `च`: `शीर्षकम्` and `मूल्यम्` aliased as `पुस्तकमूल्यम्` (`मूल्यम् पुस्तकमूल्यम् इति आख्या`). Because 2 columns are selected, finish with the dual verb `आह्रियेताम् ।`.",
        "project_id": "university_bookstore/SAMSQL",
        "starter_ssql": "पुस्तकेभ्यः\n    शीर्षकम् च\n    ",
        "hint": "Add `मूल्यम् पुस्तकमूल्यम् इति आख्या च आह्रियेताम् ।` to complete the 2-column aliased projection.",
        "solution_ssql": "पुस्तकेभ्यः शीर्षकम् च मूल्यम् पुस्तकमूल्यम् इति आख्या च आह्रियेताम् ।",
        "expected_columns": ["title", "पुस्तकमूल्यम्"],
        "expected_min_rows": 5,
    },
    "track-a-03": {
        "title": "Exercise 3: Filtering Rows with WHERE (इति नियमेन)",
        "prompt": "From `पुस्तकानि` (`पुस्तकेभ्यः`), filter where `मूल्यम्` (neuter singular) is less than (`न्यूनम्`) `२०` (`मूल्यम् २० इत्यस्मात् न्यूनम् अस्ति इति नियमेन`), and fetch 2 columns: `शीर्षकम्` and `मूल्यम्`.",
        "project_id": "university_bookstore/SAMSQL",
        "starter_ssql": "पुस्तकेभ्यः\n    ",
        "hint": "Write the filter `मूल्यम् २० इत्यस्मात् न्यूनम् अस्ति इति नियमेन`, followed by `शीर्षकम् च मूल्यम् च आह्रियेताम् ।`.",
        "solution_ssql": "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् न्यूनम् अस्ति इति नियमेन शीर्षकम् च मूल्यम् च आह्रियेताम् ।",
        "expected_columns": ["title", "price"],
        "expected_min_rows": 1,
    },
    "track-a-04": {
        "title": "Exercise 4: Multi-Table INNER JOIN (सह ... इति मेलनेन)",
        "prompt": "From `विक्रयणानि` (`विक्रयणेभ्यः`), join with `आपणाः` (`आपणैः सह`) where `विक्रयणस्य आपणसङ्ख्या` equals `आपणस्य सङ्ख्या` (`इत्यनया समा अस्ति इति मेलनेन`), and fetch 2 columns: `आपणस्य नाम` and `विक्रयणस्य सकलमूल्यम्`.",
        "project_id": "university_bookstore/SAMSQL",
        "starter_ssql": "विक्रयणेभ्यः\n    आपणैः सह ",
        "hint": "Because `सङ्ख्या` is feminine, the join condition is `विक्रयणस्य आपणसङ्ख्या आपणस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन`, followed by `आपणस्य नाम च विक्रयणस्य सकलमूल्यम् च आह्रियेताम् ।`.",
        "solution_ssql": "विक्रयणेभ्यः आपणैः सह विक्रयणस्य आपणसङ्ख्या आपणस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन आपणस्य नाम च विक्रयणस्य सकलमूल्यम् च आह्रियेताम् ।",
        "expected_columns": ["name", "total_amount"],
        "expected_min_rows": 5,
    },
    "track-a-05": {
        "title": "Exercise 5: Grouping & Aggregation (GROUP BY + COUNT)",
        "prompt": "From `पुस्तकानि` (`पुस्तकेभ्यः`), grouped by `भाषा` (`भाषा इति समूहेन`), fetch 2 columns: `भाषा` and the count (`गणना`) of `सङ्ख्या` (`सङ्ख्यायाः गणना`) aliased as `पुस्तकगणना`.",
        "project_id": "university_bookstore/SAMSQL",
        "starter_ssql": "पुस्तकेभ्यः\n    भाषा इति समूहेन\n    ",
        "hint": "Project `भाषा च सङ्ख्यायाः गणना पुस्तकगणना इति आख्या च आह्रियेताम् ।`.",
        "solution_ssql": "पुस्तकेभ्यः भाषा इति समूहेन भाषा च सङ्ख्यायाः गणना पुस्तकगणना इति आख्या च आह्रियेताम् ।",
        "expected_columns": ["language", "पुस्तकगणना"],
        "expected_min_rows": 1,
    },
    "track-a-06": {
        "title": "Exercise 6: Window Function Ranking (OVER PARTITION BY)",
        "prompt": "From `पुस्तकानि` (`पुस्तकेभ्यः`), fetch 4 columns: `शीर्षकम्`, `भाषा`, `मूल्यम्`, and `RANK() OVER (PARTITION BY language ORDER BY price DESC)` (`श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण`) aliased as `मूल्यश्रेणी`.",
        "project_id": "university_bookstore/SAMSQL",
        "starter_ssql": "पुस्तकेभ्यः\n    शीर्षकम् च\n    भाषा च\n    मूल्यम् च\n    ",
        "hint": "Add `श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण मूल्यश्रेणी इति आख्या च आह्रियन्ताम् ।` (plural verb for 4 columns).",
        "solution_ssql": "पुस्तकेभ्यः शीर्षकम् च भाषा च मूल्यम् च श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण मूल्यश्रेणी इति आख्या च आह्रियन्ताम् ।",
        "expected_columns": ["title", "language", "price", "मूल्यश्रेणी"],
        "expected_min_rows": 5,
    },
    "track-a-07": {
        "title": "Exercise 7: Pure User-Defined Function (शुद्धम् कार्यम् + प्रदाय)",
        "prompt": "Complete the query below the `करसहितमूल्यम्` Pure UDF definition to fetch 2 columns from `पुस्तकेभ्यः`: `शीर्षकम्` and `मूल्यम् च ५ च प्रदाय करसहितमूल्यम्` aliased as `सकलमूल्यम्` (`… सकलमूल्यम् इति आख्या`).",
        "project_id": "university_bookstore/SAMSQL",
        "starter_ssql": "करसहितमूल्यम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च\n    मूल्यम् इति वास्तविकः च करः इति वास्तविकः च दीयेते\n    मूल्यम् च करः च इत्यनयोः योगः इति उत्तरम् प्रतिदीयताम्\nइति कार्यम् ।\n\nपुस्तकेभ्यः\n    शीर्षकम् च\n    ",
        "hint": "Add `मूल्यम् च ५ च प्रदाय करसहितमूल्यम् सकलमूल्यम् इति आख्या च आह्रियेताम् ।`.",
        "solution_ssql": "करसहितमूल्यम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च\n    मूल्यम् इति वास्तविकः च करः इति वास्तविकः च दीयेते\n    मूल्यम् च करः च इत्यनयोः योगः इति उत्तरम् प्रतिदीयताम्\nइति कार्यम् ।\n\nपुस्तकेभ्यः शीर्षकम् च मूल्यम् च ५ च प्रदाय करसहितमूल्यम् सकलमूल्यम् इति आख्या च आह्रियेताम् ।",
        "expected_columns": ["title", "सकलमूल्यम्"],
        "expected_min_rows": 5,
    },
    "track-b-01": {
        "title": "Exercise B1: पञ्चमी (FROM) & तृतीया (ORDER BY क्रमेण)",
        "prompt": "From `ग्राहकाः` (Ablative plural: `ग्राहकेभ्यः`), ordered by `नाम` ascending (`नाम आरोहः इति क्रमेण`), fetch 1 column: `नाम` using the singular verb `आह्रियताम् ।`.",
        "project_id": "university_bookstore/SAMSQL",
        "starter_ssql": "ग्राहकेभ्यः\n    ",
        "hint": "Write `नाम आरोहः इति क्रमेण नाम आह्रियताम् ।` after `ग्राहकेभ्यः`.",
        "solution_ssql": "ग्राहकेभ्यः नाम आरोहः इति क्रमेण नाम आह्रियताम् ।",
        "expected_columns": ["name"],
        "expected_min_rows": 3,
    },
    "track-b-02": {
        "title": "Exercise B2: वचनान्वयः • Plural Verb Concordance (बहुवचनम्)",
        "prompt": "From `आपणाः` (`आपणेभ्यः`), fetch 3 columns in `प्रथमा विभक्तिः`: `सङ्ख्या`, `नाम`, and `नगरम्` (each followed by `च`), and finish with the plural Karmaṇi Loṭ verb `आह्रियन्ताम् ।`.",
        "project_id": "university_bookstore/SAMSQL",
        "starter_ssql": "आपणेभ्यः\n    ",
        "hint": "3 projected columns require the plural (`बहुवचनम्`) verb: `सङ्ख्या च नाम च नगरम् च आह्रियन्ताम् ।`.",
        "solution_ssql": "आपणेभ्यः सङ्ख्या च नाम च नगरम् च आह्रियन्ताम् ।",
        "expected_columns": ["id", "name", "city"],
        "expected_min_rows": 3,
    },
    "track-b-03": {
        "title": "Exercise B3: लिङ्गसमन्वयः • Three-Gender Adjective Concordance",
        "prompt": "In `पुस्तकानि` (`पुस्तकेभ्यः`), `भाषा` (`language`) is Feminine Singular (`स्त्रीलिङ्गम् एकवचनम्`). Filter where `भाषा` equals `\"Sanskrit\"` using the feminine pronoun `इत्यनया` and feminine adjective `समा` (`भाषा \"Sanskrit\" इत्यनया समा अस्ति इति नियमेन`), and fetch 2 columns: `शीर्षकम्` and `भाषा`.",
        "project_id": "university_bookstore/SAMSQL",
        "starter_ssql": "पुस्तकेभ्यः\n    ",
        "hint": "Write `भाषा \"Sanskrit\" इत्यनया समा अस्ति इति नियमेन शीर्षकम् च भाषा च आह्रियेताम् ।`.",
        "solution_ssql": 'पुस्तकेभ्यः भाषा "Sanskrit" इत्यनया समा अस्ति इति नियमेन शीर्षकम् च भाषा च आह्रियेताम् ।',
        "expected_columns": ["title", "language"],
        "expected_min_rows": 2,
    },
}


def build_tutorials() -> list[dict[str, Any]]:
    """Load all 10 tutorial chapters across Track A and Track B with exercises."""
    tracks = [
        (
            "track-a",
            "Track A: SSQL for SQL Developers",
            TUTORIALS_DIR / "ssql-for-sql-developers",
        ),
        (
            "track-b",
            "Track B: Samskrutam Through SSQL",
            TUTORIALS_DIR / "samskrutam-through-ssql",
        ),
    ]

    chapters: list[dict[str, Any]] = []
    for track_id, track_title, folder in tracks:
        md_files = sorted(folder.glob("[0-9][0-9]-*.md"))
        for idx, fpath in enumerate(md_files, start=1):
            chap_id = f"{track_id}-{idx:02d}"
            fname = fpath.name
            md = fpath.read_text(encoding="utf-8")
            first_line = md.splitlines()[0].lstrip("#").strip() if md.splitlines() else fname
            chapters.append(
                {
                    "id": chap_id,
                    "track_id": track_id,
                    "track_title": track_title,
                    "filename": fname,
                    "title": first_line,
                    "markdown": md,
                    "exercise": TUTORIAL_EXERCISES[chap_id],
                }
            )
    return chapters


def package_vsix(vsix_path: Path) -> None:
    """Package `editors/vscode` into a standard Open Packaging Conventions `.vsix` archive."""
    vscode_dir = REPO_ROOT / "editors" / "vscode"
    out_ext = vscode_dir / "out" / "extension.js"
    src_ext = vscode_dir / "src" / "extension.ts"
    if not out_ext.exists() or src_ext.stat().st_mtime > out_ext.stat().st_mtime:
        subprocess.run(["npm", "run", "compile"], cwd=vscode_dir, check=True)

    content_types_xml = """<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="json" ContentType="application/json"/>
  <Default Extension="js" ContentType="application/javascript"/>
  <Default Extension="md" ContentType="text/markdown"/>
  <Default Extension="vsixmanifest" ContentType="text/xml"/>
</Types>"""

    vsix_manifest_xml = f"""<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
  <Metadata>
    <Identity Language="en-US" Id="ssql" Version="{VERSION}" Publisher="samskrutam-computing"/>
    <DisplayName>SSQL (संस्कृतम् SQL)</DisplayName>
    <Description>Language support, syntax highlighting, hover grammar help, autocomplete, formatter, linter, and LSP client for SSQL (.सं)</Description>
    <Tags>ssql,sanskrit,sql,devanagari</Tags>
    <Categories>Programming Languages,Linters,Formatters,Snippets</Categories>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code"/>
  </Installation>
  <Dependencies/>
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true"/>
  </Assets>
</PackageManifest>"""

    include_files = [
        "package.json",
        "language-configuration.json",
        "syntaxes/ssql.tmLanguage.json",
        "snippets/ssql.json",
        "out/extension.js",
    ]

    with zipfile.ZipFile(vsix_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("[Content_Types].xml", content_types_xml)
        zf.writestr("extension.vsixmanifest", vsix_manifest_xml)
        for rel in include_files:
            fp = vscode_dir / rel
            if fp.exists():
                zf.write(fp, arcname=f"extension/{rel}")


def package_tar_gz(archive_path: Path, files: list[tuple[Path, str]]) -> None:
    with tarfile.open(archive_path, "w:gz") as tf:
        for src_path, arcname in files:
            if src_path.exists():
                tf.add(src_path, arcname=arcname)


def build_downloads_on_change() -> list[dict[str, Any]]:
    """Build or refresh downloadable release artifacts whenever underlying sources change."""
    DOWNLOADS_DIR.mkdir(parents=True, exist_ok=True)

    # 1. VS Code Extension (.vsix)
    vsix_path = DOWNLOADS_DIR / f"ssql-vscode-{VERSION}.vsix"
    vscode_mtime = _latest_mtime(
        [
            REPO_ROOT / "editors" / "vscode" / "src",
            REPO_ROOT / "editors" / "vscode" / "syntaxes",
            REPO_ROOT / "editors" / "vscode" / "snippets",
            REPO_ROOT / "editors" / "vscode" / "package.json",
            REPO_ROOT / "editors" / "vscode" / "language-configuration.json",
        ]
    )
    if not vsix_path.exists() or vscode_mtime > vsix_path.stat().st_mtime:
        package_vsix(vsix_path)

    # 2. WASM SDK Bundle (.tar.gz) — rebuild `website/pkg` via `wasm-pack` if `ssql-wasm/src` changed
    wasm_src_mtime = _latest_mtime(
        [REPO_ROOT / "src", WEBSITE_DIR / "ssql-wasm" / "src", WEBSITE_DIR / "ssql-wasm" / "Cargo.toml"]
    )
    wasm_bg_file = WEBSITE_DIR / "pkg" / "ssql_wasm_bg.wasm"
    if not wasm_bg_file.exists() or wasm_src_mtime > wasm_bg_file.stat().st_mtime:
        subprocess.run(
            ["wasm-pack", "build", "--release", "--target", "web", "--out-dir", "../pkg"],
            cwd=WEBSITE_DIR / "ssql-wasm",
            check=True,
        )

    wasm_tar_path = DOWNLOADS_DIR / f"ssql-wasm-{VERSION}.tar.gz"
    wasm_mtime = _latest_mtime([WEBSITE_DIR / "pkg", WEBSITE_DIR / "ssql-wasm" / "src"])
    if not wasm_tar_path.exists() or wasm_mtime > wasm_tar_path.stat().st_mtime:
        pkg_files = [
            (p, f"ssql-wasm-{VERSION}/{p.name}")
            for p in sorted((WEBSITE_DIR / "pkg").iterdir())
            if p.is_file()
        ]
        package_tar_gz(wasm_tar_path, pkg_files)

    # 3. Compiler CLI binary archive (`ssql_compiler`)
    compiler_bin = REPO_ROOT / "target" / "release" / "ssql_compiler"
    src_mtime = _latest_mtime([REPO_ROOT / "src", REPO_ROOT / "Cargo.toml"])
    if not compiler_bin.exists() or src_mtime > compiler_bin.stat().st_mtime:
        subprocess.run(
            ["cargo", "build", "--release", "--bin", "ssql_compiler"],
            cwd=REPO_ROOT,
            check=True,
        )
    compiler_tar = DOWNLOADS_DIR / f"ssql_compiler-{VERSION}-macos-arm64.tar.gz"
    if not compiler_tar.exists() or compiler_bin.stat().st_mtime > compiler_tar.stat().st_mtime:
        package_tar_gz(
            compiler_tar,
            [
                (compiler_bin, "ssql_compiler"),
                (REPO_ROOT / "README.md", "README.md"),
            ],
        )

    # 4. Language Server binary archive (`ssql-lsp`)
    lsp_bin = REPO_ROOT / "target" / "release" / "ssql-lsp"
    lsp_mtime = max(
        src_mtime,
        _latest_mtime([REPO_ROOT / "ssql-lsp" / "src", REPO_ROOT / "ssql-lsp" / "Cargo.toml"]),
    )
    if not lsp_bin.exists() or lsp_mtime > lsp_bin.stat().st_mtime:
        subprocess.run(
            ["cargo", "build", "--release", "-p", "ssql-lsp"],
            cwd=REPO_ROOT,
            check=True,
        )
    lsp_tar = DOWNLOADS_DIR / f"ssql-lsp-{VERSION}-macos-arm64.tar.gz"
    if not lsp_tar.exists() or lsp_bin.stat().st_mtime > lsp_tar.stat().st_mtime:
        package_tar_gz(
            lsp_tar,
            [
                (lsp_bin, "ssql-lsp"),
                (REPO_ROOT / "README.md", "README.md"),
            ],
        )

    # 5. Python Wheel (`ssql-0.2.1-*.whl`)
    py_mtime = max(
        src_mtime,
        _latest_mtime([REPO_ROOT / "ssql-py" / "src", REPO_ROOT / "ssql-py" / "pyproject.toml"]),
    )
    existing_wheels = sorted(DOWNLOADS_DIR.glob(f"ssql-{VERSION}-*.whl"))
    if not existing_wheels or py_mtime > existing_wheels[-1].stat().st_mtime:
        for old_w in DOWNLOADS_DIR.glob("ssql-*.whl"):
            old_w.unlink(missing_ok=True)
        subprocess.run(
            [
                "uv",
                "run",
                "--frozen",
                "maturin",
                "build",
                "--release",
                "--manifest-path",
                str(REPO_ROOT / "ssql-py" / "Cargo.toml"),
                "--out",
                str(DOWNLOADS_DIR),
            ],
            cwd=REPO_ROOT,
            check=True,
        )
        existing_wheels = sorted(DOWNLOADS_DIR.glob(f"ssql-{VERSION}-*.whl"))

    wheel_path = existing_wheels[-1]

    artifacts = [
        {
            "id": "vsix",
            "title": "VS Code Extension (.vsix)",
            "subtitle": "Syntax highlighting, Hover grammar help, Autocomplete, Formatter & LSP client",
            "filename": vsix_path.name,
            "url": f"/downloads/{vsix_path.name}",
            "size": _human_size(vsix_path.stat().st_size),
            "sha256": _sha256_short(vsix_path),
            "sha256_full": _sha256_full(vsix_path),
            "install_cmd": f"code --install-extension {vsix_path.name}",
            "source_cmd": f"cd editors/vscode && npm install && npx @vscode/vsce package && code --install-extension {vsix_path.name}",
            "platform_note": "Cross-Platform (macOS, Linux, Windows)",
            "badge": "Cross-Platform VSIX",
        },
        {
            "id": "compiler",
            "title": "SSQL Compiler CLI (ssql_compiler)",
            "subtitle": "Native release binary with SQLite/PostgreSQL codegen, --fmt, --lint & --from-sql",
            "filename": compiler_tar.name,
            "url": f"/downloads/{compiler_tar.name}",
            "size": _human_size(compiler_tar.stat().st_size),
            "sha256": _sha256_short(compiler_tar),
            "sha256_full": _sha256_full(compiler_tar),
            "install_cmd": f"tar -xzf {compiler_tar.name} && sudo install -m 755 ssql_compiler /usr/local/bin/",
            "source_cmd": "cargo install --path .",
            "platform_note": "Prebuilt for macOS ARM64 (Apple Silicon) • Linux/Windows/Intel: cargo install --path .",
            "badge": "macOS ARM64 Binary",
        },
        {
            "id": "lsp",
            "title": "SSQL Language Server (ssql-lsp)",
            "subtitle": "Native Tower LSP binary with automatic schema.सं discovery, Kāraka diagnostics, hover & completions",
            "filename": lsp_tar.name,
            "url": f"/downloads/{lsp_tar.name}",
            "size": _human_size(lsp_tar.stat().st_size),
            "sha256": _sha256_short(lsp_tar),
            "sha256_full": _sha256_full(lsp_tar),
            "install_cmd": f"tar -xzf {lsp_tar.name} && sudo install -m 755 ssql-lsp /usr/local/bin/",
            "source_cmd": "cargo install --path ssql-lsp",
            "platform_note": "Prebuilt for macOS ARM64 (Apple Silicon) • Linux/Windows/Intel: cargo install --path ssql-lsp",
            "badge": "macOS ARM64 Binary",
        },
        {
            "id": "wheel",
            "title": "Python Bindings Wheel (ssql-py)",
            "subtitle": "Native PyO3 wheel providing import ssql; ssql.Compiler() for Python & uv workflows",
            "filename": wheel_path.name,
            "url": f"/downloads/{wheel_path.name}",
            "size": _human_size(wheel_path.stat().st_size),
            "sha256": _sha256_short(wheel_path),
            "sha256_full": _sha256_full(wheel_path),
            "install_cmd": f"uv pip install {wheel_path.name}",
            "source_cmd": "uv pip install ./ssql-py",
            "platform_note": "Prebuilt for macOS ARM64 (Python 3.14) • Any OS/Python 3.10+: uv pip install ./ssql-py",
            "badge": "macOS ARM64 • Py3.14 Wheel",
        },
        {
            "id": "wasm",
            "title": "WebAssembly SDK (ssql-wasm)",
            "subtitle": "Browser & Edge WASM package (compile_ssql_wasm, format_ssql_wasm, lint_ssql_wasm, transpile_sql_to_ssql_wasm)",
            "filename": wasm_tar_path.name,
            "url": f"/downloads/{wasm_tar_path.name}",
            "size": _human_size(wasm_tar_path.stat().st_size),
            "sha256": _sha256_short(wasm_tar_path),
            "sha256_full": _sha256_full(wasm_tar_path),
            "install_cmd": f"tar -xzf {wasm_tar_path.name}",
            "source_cmd": "wasm-pack build --target web website/ssql-wasm",
            "platform_note": "Cross-Platform Browser & Edge WebAssembly ES Module",
            "badge": "Cross-Platform WASM",
        },
    ]
    return artifacts


def ensure_up_to_date(force: bool = False) -> dict[str, Any]:
    """Ensure `corpus.json`, `content.json`, and all downloadable binaries are up to date."""
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    DOWNLOADS_DIR.mkdir(parents=True, exist_ok=True)

    corpus_path = DATA_DIR / "corpus.json"
    content_path = DATA_DIR / "content.json"

    examples_mtime = _latest_mtime([EXAMPLES_DIR, Path(__file__)])
    if force or not corpus_path.exists() or examples_mtime > corpus_path.stat().st_mtime:
        projects = discover_example_projects()
        total_queries = sum(p["query_count"] for p in projects)
        corpus_payload = {
            "version": VERSION,
            "project_count": len(projects),
            "total_queries": total_queries,
            "projects": projects,
        }
        corpus_path.write_text(
            json.dumps(corpus_payload, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )

    artifacts = build_downloads_on_change()

    docs_mtime = _latest_mtime([TUTORIALS_DIR, CONSTITUTION_PATH, DOWNLOADS_DIR, Path(__file__)])
    if force or not content_path.exists() or docs_mtime > content_path.stat().st_mtime:
        constitution = parse_constitution()
        tutorials = build_tutorials()
        content_payload = {
            "version": VERSION,
            "constitution": constitution,
            "tutorials": tutorials,
            "downloads": artifacts,
        }
        content_path.write_text(
            json.dumps(content_payload, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )

    return {
        "corpus": str(corpus_path),
        "content": str(content_path),
        "downloads_count": len(artifacts),
    }


if __name__ == "__main__":
    summary = ensure_up_to_date(force=True)
    corpus_data = json.loads((DATA_DIR / "corpus.json").read_text(encoding="utf-8"))
    content_data = json.loads((DATA_DIR / "content.json").read_text(encoding="utf-8"))
    print(
        f"Built SSQL site bundles: {corpus_data['project_count']} projects, "
        f"{corpus_data['total_queries']} queries, "
        f"{len(content_data['tutorials'])} tutorials, "
        f"{len(content_data['constitution']['articles'])} constitutional articles, "
        f"{len(content_data['downloads'])} downloadable artifacts."
    )
