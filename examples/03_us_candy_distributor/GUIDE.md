# Project 03: US Candy Distributor - Beginner's Guide

This project covers multi-table inner joins in SSQL, connecting orders to their respective products and manufacturing factories.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Join the `orders`, `factories`, and `products` tables to output complete order transaction details.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
Define three tables: `उत्पादाः` (products), `उद्योगशालाः` (factories), and `आदेशाः` (orders).
* SSQL requires that all columns have grammatical genders and numbers declared (e.g. `राजस्व इति नपुंसकलिङ्गम् एकवचनम्`).
* Primary keys (`प्राथमिकी सङ्ख्या`) and foreign keys like `उत्पादेषु बद्धः` are declared directly inline.

### 2. Solving Objective 1: Joining Three Tables
We retrieve order ID, product name, factory name, quantity, and revenue.
* **Sanskritization**:
  * We query from the main transaction table `आदेशाः` (orders), inflected as Ablative `आदेशेभ्यः`.
  * We join the `products` table using `उत्पादैः सह आदेशस्य उत्पादसङ्ख्या उत्पादस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन`.
  * We join the `factories` table using `उद्योगशालाभिः सह आदेशस्य उद्योगशालासङ्ख्या उद्योगशालायाः सङ्ख्या इत्यनया समा अस्ति इति मेलनेन`.
  * We select: `आदेशस्य सङ्ख्या` (order ID), `उत्पादस्य नाम` (product name), `उद्योगशालायाः नाम` (factory name), `आदेशस्य मात्रा` (units), and `आदेशस्य राजस्वम्` (revenue).
  * Since we select 5 columns, we use the plural fetch verb `आह्रियन्ताम्`.
* **Query** (`queries/q1_order_details.सं`):
  ```sanskrit
  आदेशेभ्यः
      उत्पादैः सह आदेशस्य उत्पादसङ्ख्या उत्पादस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
      उद्योगशालाभिः सह आदेशस्य उद्योगशालासङ्ख्या उद्योगशालायाः सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
      आदेशस्य सङ्ख्या च उत्पादस्य नाम च उद्योगशालायाः नाम च आदेशस्य मात्रा च आदेशस्य राजस्वम् च
      आह्रियन्ताम् ।
  ```

---

## 🧪 Local Verification

You can verify that these queries compile and execute correctly by running the `verify_all.py` script from the repository root:
```bash
uv run python3 verify_all.py
```
This compiles the queries, executes them against both SQLite and PostgreSQL, and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
