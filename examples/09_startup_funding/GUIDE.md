# Project 09: Startup Funding - Beginner's Guide

This project demonstrates CASE (`वरणम्`) expressions and data cleaning in SSQL.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Clean investor names by replacing "Undisclosed" with "Anonymous", and outputting the investment values.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
We define `निवेशकाः` ("investors"):
*   `नाम` (investor_name) is declared as `नाम इति वाक्यम् ...` to align with the SSQL nominative column naming convention.

### 2. Solving Objective 1: Cleaning Names with CASE
We use the `वरणम्` (CASE) expression:
*   **Case logic**: `वरणम्` introduces the conditional block.
*   **Branches**: 
    *   `यस्मिन् नाम "Undisclosed" इत्यनेन समम् अस्ति तस्मिन् "Anonymous"` replaces undisclosed investor strings.
    *   `अन्यस्मिन् नाम` returns the default name otherwise.
*   **Alias**: We alias the case block as `"clean_name" इति आख्या` to produce an English column alias.
*   **Verb Agreement**: We select exactly two columns (`clean_name` and `funding_amount`), so we use the dual verb `आह्रियेताम्`.
*   **Query** (`queries/q1_funding_clean.सं`):
    ```sanskrit
    निवेशकेभ्यः
        वरणम्
            यस्मिन् नाम "Undisclosed" इत्यनेन समम् अस्ति तस्मिन् "Anonymous"
            अन्यस्मिन् नाम
          "clean_name" इति आख्या च
        मूल्यम् च
        आह्रियेताम् ।
    ```

---

## 🔍 Verification

Run the verify script from the repository root to compile and execute the queries:
```bash
uv run python3 verify_all.py
```
It executes the queries against both SQLite and PostgreSQL and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
