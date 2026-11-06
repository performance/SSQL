# Project 08: Ingredient Costs - Beginner's Guide

This project demonstrates inner joins and arithmetic field operations in SSQL.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Join pizza types and ingredients to calculate the total profit (`price - ingredient_cost`) for each pizza.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
We define two tables:
*   `पिष्टकप्रकाराः` ("pizza_types") with `नाम` (name) as a `वाक्यम्` (TEXT) and `मूल्यम्` (price) as a `वास्तविकः` (numeric/decimal).
*   `घटकाः` ("ingredients") with `घटकमूल्यम्` (ingredient cost) as `वास्तविकः`.

### 2. Solving Objective 1: Profit Calculation
To join the two tables and calculate profit:
*   **Join**: We select from `पिष्टकप्रकाराः` (Ablative plural `पिष्टकप्रकारेभ्यः`) and join `घटकाः` using `घटकैः सह पिष्टकप्रकारस्य प्रकारसङ्ख्या घटकस्य प्रकारसङ्ख्या इत्यनया समा अस्ति इति मेलनेन`. Note the correct instrumental plural `घटकैः सह`.
*   **Arithmetic**: We subtract ingredients cost from pizza price using the native subtraction pattern (Shashthi-qualified Panchami minuend + Shashthi-qualified Dvitiya subtrahend + `विशोध्य`/`अवशिष्टम्`): `पिष्टकप्रकारस्य मूल्यात् घटकस्य घटकमूल्यम् विशोध्य अवशिष्टम् "profit" इति आख्या`.
*   **Verb Agreement**: We select exactly two columns (`profit` and `name`), so we use the dual verb `आह्रियेताम्`.
*   **Query** (`queries/q1_profit.सं`):
    ```sanskrit
    पिष्टकप्रकारेभ्यः घटकैः सह पिष्टकप्रकारस्य प्रकारसङ्ख्या घटकस्य प्रकारसङ्ख्या इत्यनया समा अस्ति इति मेलनेन पिष्टकप्रकारस्य मूल्यात् घटकस्य घटकमूल्यम् विशोध्य अवशिष्टम् "profit" इति आख्या च पिष्टकप्रकारस्य नाम च आह्रियेताम् ।
    ```

---

## 🔍 Verification

Run the verify script from the repository root to compile and execute the queries:
```bash
uv run python3 verify_all.py
```
It executes the queries against both SQLite and PostgreSQL and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
