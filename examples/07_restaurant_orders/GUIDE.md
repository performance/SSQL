# Project 07: Restaurant Orders - Beginner's Guide

This project demonstrates subqueries and comparative filter expressions in SSQL.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Filter order detail records where the individual item price is greater than the global average ticket/item price.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
We define `आदेशविवरणानि` ("order_details") with:
*   `विवरणसङ्ख्या` (details ID), `आदेशसङ्ख्या` (order ID), and `मूल्यम्` (price).
*   Genders and grammatical numbers are explicitly declared for all fields (e.g. `मूल्य इति नपुंसकलिङ्गम् एकवचनम् च`).

### 2. Solving Objective 1: Comparative Subquery
We retrieve all tickets where the item price is greater than the average price:
*   **Subquery**: `यत् आदेशविवरणेभ्यः मूल्यस्य माध्यम् आह्रियताम् इति` evaluates the average price: `SELECT AVG(price) FROM order_details`, enclosed in traditional Sanskrit `यत् ... इति` subordinate-clause delimiters.
*   **Comparison**: `मूल्यम् यत् आदेशविवरणेभ्यः मूल्यस्य माध्यम् आह्रियताम् इति इत्यस्मात् अधिकम् अस्ति इति नियमेन` compares the row price to the subquery average.
*   **Select list**: We select three columns, hence the plural verb `आह्रियन्ताम्` is used.
*   **Query** (`queries/q1_large_tickets.सं`):
    ```sanskrit
    आदेशविवरणेभ्यः
        मूल्यम् यत् आदेशविवरणेभ्यः मूल्यस्य माध्यम् आह्रियताम् इति इत्यस्मात् अधिकम् अस्ति इति नियमेन
        विवरणसङ्ख्या च आदेशसङ्ख्या च मूल्यम् च
        आह्रियन्ताम् ।
    ```

---

## 🔍 Verification

Run the verify script from the repository root to compile and execute the queries:
```bash
uv run python3 verify_all.py
```
It executes the queries against both SQLite and PostgreSQL and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
