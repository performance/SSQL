# Project 14: Marketing ROI - Beginner's Guide

This project demonstrates using scalar string functions (`upper` / `महावर्णरूपम्`) with Shashthi Vibhakti in SSQL.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Standardize promotional codes by converting them to uppercase alongside their ID and cost.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
*   **Table**: We define table `प्रचाराः` ("promotions").
*   **Columns**:
    *   `सङ्ख्या` (PK, promo_id)
    *   `सङ्केतपदम्` (promo_code)
    *   `व्ययः` (cost)

### 2. Solving Objective 1: Promo Code Standardization
*   **Uppercase (`महावर्णरूपम्`)**: Applied using Shashthi Vibhakti (`प्रचारस्य सङ्केतपदस्य महावर्णरूपम् "cleaned_code" इति आख्या` — literally "the uppercase form of the promotion's promo-code").
*   **Verb Agreement**: Since three columns are selected (`cleaned_code`, `सङ्ख्या`, and `व्ययः`), we use the plural verb form `आह्रियन्ताम्`.
*   **Query** (`queries/q1_marketing.सं`):
    ```sanskrit
    प्रचारेभ्यः
        प्रचारस्य सङ्केतपदस्य महावर्णरूपम् "cleaned_code" इति आख्या च
        प्रचारस्य सङ्ख्या च
        प्रचारस्य व्ययः च
        आह्रियन्ताम् ।
    ```

---

## 🔍 Verification

Run the verify script from the repository root to compile and execute the queries:
```bash
uv run python3 verify_all.py
```
It executes the queries against both SQLite and PostgreSQL and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
