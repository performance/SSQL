# Project 12: Unicorn Valuation - Beginner's Guide

This project demonstrates aggregations, GROUP BY, and HAVING constraints in SSQL.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Group unicorn companies by industry, filtering for industries with more than 5 companies, ordered by total valuation descending.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
*   **Table**: We define table `एकशृङ्गाः` ("unicorns").
*   **Columns**:
    *   `सङ्ख्या` (PK, company_id)
    *   `संस्था` (company)
    *   `मूल्यम्` (valuation, Decimal/NUMERIC for precision)
    *   `उद्योगः` (industry)
    *   `देशः` (country)

### 2. Solving Objective 1: Aggregated Valuation
*   **Grouping**: Grouped by industry using `उद्योगः इति समूहेन`.
*   **Filtering (HAVING)**: Filtered for count > 5 using `संस्थायाः गणना ५ इत्यस्याः अधिका अस्ति इति समूहनियमेन`.
*   **Sorting**: Ordered descending by total valuation using `मूल्यस्य योगः अवरोहः इति क्रमेण`.
*   **Verb Agreement**: Since two fields are selected, we use the dual verb form `आह्रियेताम्`.
*   **Query** (`queries/q1_unicorns.सं`):
    ```sanskrit
    एकशृङ्गेभ्यः
        उद्योगः इति समूहेन
        संस्थायाः गणना ५ इत्यस्याः अधिका अस्ति इति समूहनियमेन
        मूल्यस्य योगः अवरोहः इति क्रमेण
        मूल्यस्य योगः "total_valuation" इति आख्या च
        उद्योगः च
        आह्रियेताम् ।
    ```

---

## 🔍 Verification

Run the verify script from the repository root to compile and execute the queries:
```bash
uv run python3 verify_all.py
```
It executes the queries against both SQLite and PostgreSQL and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
