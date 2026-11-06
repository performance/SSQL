# Project 02: Global Electronics - Beginner's Guide

This project demonstrates how to perform aggregations with groupings and filters based on group metrics (the `HAVING` clause equivalent in SSQL).

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Calculate total revenue by product category, filtering categories with fewer than 500 total orders.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
Define three tables: `ग्राहकाः` (customers), `उत्पादाः` (products), and `आदेशाः` (orders). 
* Notice how gender information is declared for columns using e.g. `सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम्`. This is essential because SSQL uses these grammatical declarations to validate that our queries maintain gender concordance.

### 2. Solving Objective 1: Revenue by Category with Filter
We want to get `category` and the sum of `quantity * unit_price_usd` for category groupings that have more than 500 orders.
* **Sanskritization**:
  * We query from `आदेशाः` (orders), inflected as Ablative `आदेशेभ्यः`.
  * We join the `products` table using `उत्पादैः सह आदेशस्य उत्पादसङ्ख्या उत्पादस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन`.
  * We group by the product category: `उत्पादस्य वर्गः इति समूहेन`. Note that the group-by clause is placed postfix after the joins.
  * We filter the groups having count > 500: `आदेशस्य सङ्ख्यायाः गणना ५०० इत्यस्याः अधिका अस्ति इति समूहनियमेन`. In SSQL, `समूहनियमेन` is the postfix equivalent of SQL's `HAVING` clause.
  * The selected fields are the product category (`उत्पादस्य वर्गः`) and the calculated revenue `आदेशस्य मात्रा च उत्पादस्य मूल्यम् च इत्यनयोः गुणनम् इत्यस्य योगः राजस्वम् इति आख्या`. The dual `इत्यनयोः` takes the two factors and the singular `इत्यस्य` the one product, so no `यत् … इति` group is needed.
  * Since we select exactly 2 columns, we use the dual fetch verb `आह्रियेताम्`.
* **Query** (`queries/q1_revenue_by_category.सं`):
  ```sanskrit
  आदेशेभ्यः
      उत्पादैः सह आदेशस्य उत्पादसङ्ख्या उत्पादस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
      उत्पादस्य वर्गः इति समूहेन
      आदेशस्य सङ्ख्यायाः गणना ५०० इत्यस्याः अधिका अस्ति इति समूहनियमेन
      उत्पादस्य वर्गः च
      आदेशस्य मात्रा च उत्पादस्य मूल्यम् च इत्यनयोः गुणनम् इत्यस्य योगः राजस्वम् इति आख्या च
      आह्रियेताम् ।
  ```

---

## 🧪 Local Verification

You can verify that these queries compile and execute correctly by running the `verify_all.py` script from the repository root:
```bash
uv run python3 verify_all.py
```
This compiles the queries, executes them against both SQLite and PostgreSQL, and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
