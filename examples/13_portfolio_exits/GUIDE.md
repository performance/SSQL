# Project 13: Portfolio Exits - Beginner's Guide

This project demonstrates set difference operations (EXCEPT) in SSQL.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Find all unicorn companies that have NOT exited (present in `unicorns` but not present in `exits`).

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
*   **Tables**: We define two tables: `एकशृङ्गाः` ("unicorns") and `निर्गमाः` ("exits").
*   **Columns**:
    *   `एकशृङ्गाः`: `सङ्ख्या` (PK), `संस्था` (company)
    *   `निर्गमाः`: `सङ्ख्या` (PK), `संस्था` (company), `मूल्यम्` (exit_value)

### 2. Solving Objective 1: Set Operations
*   **CTEs + Set Difference**: We define two CTEs (`एकशृङ्गनामानि` and `निर्गमनामानि`) and combine them using `एकशृङ्गनामानि च निर्गमनामानि च इत्यनयोः वर्जनम् आह्रियताम्`.
*   **Operator**: We use the operator `वर्जनम्` (EXCEPT).
*   **Verb Agreement**: Since only one column (`संस्था`) is selected from each subquery and projected overall, we use the singular verb form `आह्रियताम्`.
*   **Query** (`queries/q1_exits.सं`):
    ```sanskrit
    एकशृङ्गनामानि इति आख्या एकशृङ्गनामन् इति नपुंसकलिङ्गम् बहुवचनम् एकशृङ्गेभ्यः संस्था आह्रियताम् ।
    निर्गमनामानि इति आख्या निर्गमनामन् इति नपुंसकलिङ्गम् बहुवचनम् निर्गमेभ्यः संस्था आह्रियताम् ।

    एकशृङ्गनामानि च निर्गमनामानि च इत्यनयोः वर्जनम् आह्रियताम् ।
    ```

---

## 🔍 Verification

Run the verify script from the repository root to compile and execute the queries:
```bash
uv run python3 verify_all.py
```
It executes the queries against both SQLite and PostgreSQL and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
