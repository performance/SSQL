# Project 04: Customer Acquisition - Beginner's Guide

This project demonstrates how to perform `LEFT OUTER JOIN` queries to identify records in one table that do not have matching associations in another table (e.g. unacquired customers who have placed zero orders).

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Find all customers with 0 orders (using Left Join and NULL checks).

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
Define two tables: `ग्राहकाः` (customers) and `आदेशाः` (orders). 
* Genders are defined explicitly for all columns.
* A foreign key relation is set up in the orders table referencing customers: `ग्राहकेषु बद्धः ग्राहकसङ्ख्या`.

### 2. Solving Objective 1: Unacquired Customers (0 Orders)
We perform a `LEFT JOIN` on orders and check if order ID is `NULL`.
* **Sanskritization**:
  * We query from `ग्राहकाः` (customers), inflected as Ablative `ग्राहकेभ्यः`.
  * We perform a Left Join with `आदेशाः` using `आदेशैः सह ग्राहकस्य सङ्ख्या आदेशस्य ग्राहकसङ्ख्या इत्यनया समा अस्ति इति वाममेलनेन`. Note that `वाममेलनेन` is SSQL's keyword for `LEFT OUTER JOIN`.
  * We filter for rows where the order ID is NULL: `आदेशस्य सङ्ख्या रिक्ता अस्ति इति नियमेन`. In SSQL, `रिक्ता अस्ति` parses to `IS NULL` in SQL.
  * The selected fields are the customer name (`ग्राहकस्य नाम`) and email (`ग्राहकस्य विद्युत्पत्रम्`).
  * Since we select exactly 2 columns, we use the dual fetch verb `आह्रियेताम्`.
* **Query** (`queries/q1_unacquired_customers.सं`):
  ```sanskrit
  ग्राहकेभ्यः
      आदेशैः सह ग्राहकस्य सङ्ख्या आदेशस्य ग्राहकसङ्ख्या इत्यनया समा अस्ति इति वाममेलनेन
      आदेशस्य सङ्ख्या रिक्ता अस्ति इति नियमेन
      ग्राहकस्य नाम च ग्राहकस्य विद्युत्पत्रम् च
      आह्रियेताम् ।
  ```

---

## 🧪 Local Verification

You can verify that these queries compile and execute correctly by running the `verify_all.py` script from the repository root:
```bash
uv run python3 verify_all.py
```
This compiles the queries, executes them against both SQLite and PostgreSQL, and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
