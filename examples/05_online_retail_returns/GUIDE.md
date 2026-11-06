# Project 05: Online Retail Returns - Beginner's Guide

This project explores how to perform conditional logic (the SQL `CASE` statement) inside SSQL aggregate functions to calculate advanced business metrics (like product return rates).

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Identify returned transactions (codes starting with 'C') and calculate the quantity returned versus the quantity sold.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
Define a single table `व्यवहाराः` (transactions) storing transaction codes, quantities, and prices.
* Note that `quantity` (quantity) and `transaction_code` (transaction_code) are registered along with their grammatical properties.

### 2. Solving Objective 1: Return Rates via `वरणम्` (Case expression)
We filter for transaction codes starting with 'C' using `LIKE 'C%'` and sum the quantities of those rows.
* **Sanskritization**:
  * We query from `व्यवहाराः` (transactions), inflected as Ablative `व्यवहारेभ्यः`.
  * In SSQL, the `CASE` statement uses the `वरणम्` keyword, enclosed with `यत् ... इति` when passed into an aggregate:
    ```sanskrit
    यत् वरणम्
        यस्मिन् व्यवहारस्य सङ्केतः "C%" इत्यनेन सदृशः अस्ति तस्मिन् ऋणात्मिका व्यवहारस्य मात्रा
        अन्यस्मिन् ० इति
    ```
    * Returns are stored as negative quantities, so `ऋणात्मिका` turns them back into a count.
    * `यस्मिन्` starts a conditional branch (like `WHEN`).
    * `तस्मिन्` defines the result of that branch (like `THEN`).
    * `अन्यस्मिन्` defines the default fallback (like `ELSE`).
  * We sum this conditional expression: `यत् वरणम् ... इति इत्यस्य योगः प्रत्यावृत्तमात्रा इति आख्या`. The `यत् ... इति` quoting boundaries around the `वरणम्...अन्यस्मिन् ०` expression ensure that `इत्यस्य योगः` binds to the entire CASE expression (`SUM(CASE WHEN ... THEN quantity ELSE 0 END)`) rather than just the `०` in the ELSE branch.
  * The quantity sold is the same sum over the other transactions, whose code is `सदृशः नास्ति`: `… विक्रीतमात्रा इति आख्या`.
  * Since we select exactly 2 columns, we use the dual fetch verb `आह्रियेताम्`.
* **Query** (`queries/q1_return_rate.सं`):
  ```sanskrit
  व्यवहारेभ्यः
      यत् वरणम्
          यस्मिन् व्यवहारस्य सङ्केतः "C%" इत्यनेन सदृशः अस्ति तस्मिन् ऋणात्मिका व्यवहारस्य मात्रा
          अन्यस्मिन् ० इति
      इत्यस्य योगः प्रत्यावृत्तमात्रा इति आख्या च
      यत् वरणम्
          यस्मिन् व्यवहारस्य सङ्केतः "C%" इत्यनेन सदृशः नास्ति तस्मिन् व्यवहारस्य मात्रा
          अन्यस्मिन् ० इति
      इत्यस्य योगः विक्रीतमात्रा इति आख्या च
  आह्रियेताम् ।
  ```

---

## 🧪 Local Verification

You can verify that these queries compile and execute correctly by running the `verify_all.py` script from the repository root:
```bash
uv run python3 verify_all.py
```
This compiles the queries, executes them against both SQLite and PostgreSQL, and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
