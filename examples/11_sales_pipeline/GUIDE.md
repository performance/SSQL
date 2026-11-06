# Project 11: B2B Sales Pipeline - Beginner's Guide

This project demonstrates using Common Table Expressions (CTEs) and basic math operations in SSQL.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Evaluate the sales velocity (average days to close won deals) for each product.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
*   **Table**: We define the table `अवसराः` ("opportunities").
*   **Columns**:
    *   `सङ्ख्या` (PK, opportunity_id)
    *   `उत्पादः` (product)
    *   `अवस्था` (deal_stage)
    *   `प्रारम्भदिनम्` (engage_day)
    *   `अन्तदिनम्` (close_day)

### 2. Solving Objective 1: Sales Velocity Query
*   **CTE Definition**: First we define a CTE named `विजेतव्यकालः` to compute the closure days for all won opportunities using native Panchami subtraction (`अन्तदिनात् प्रारम्भदिनम् विशोध्य अवशिष्टम्`):
    `विजेतव्यकालः इति आख्या विजेतव्यकाल इति पुंलिङ्गम् एकवचनम् अवसरेभ्यः अवस्था "Won" इत्यनया समा अस्ति इति नियमेन अन्तदिनात् प्रारम्भदिनम् विशोध्य अवशिष्टम् दिनान्तरम् इति आख्या च उत्पादः च आह्रियेताम् ।`
*   **Main Selection**: Then we select from the CTE, grouping by product to find the average:
    `विजेतव्यकालात् उत्पादः इति समूहेन उत्पादः च दिनान्तरम् इत्यस्य माध्यम् "avg_days" इति आख्या च आह्रियेताम् ।`
*   **Verb Agreement**: Since two fields are selected, we use the dual verb form `आह्रियेताम्`.
*   **Query** (`queries/q1_velocity.सं`):
    ```sanskrit
    विजेतव्यकालः इति आख्या विजेतव्यकाल इति पुंलिङ्गम् एकवचनम् अवसरेभ्यः अवस्था "Won" इत्यनया समा अस्ति इति नियमेन अन्तदिनात् प्रारम्भदिनम् विशोध्य अवशिष्टम् दिनान्तरम् इति आख्या च उत्पादः च आह्रियेताम् ।
    विजेतव्यकालात् उत्पादः इति समूहेन उत्पादः च दिनान्तरम् इत्यस्य माध्यम् "avg_days" इति आख्या च आह्रियेताम् ।
    ```

---

## 🔍 Verification

Run the verify script from the repository root to compile and execute the queries:
```bash
uv run python3 verify_all.py
```
It executes the queries against both SQLite and PostgreSQL and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
