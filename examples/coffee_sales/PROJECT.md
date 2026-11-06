# Project: Coffee Sales

**Domain:** Food & Beverage · **Difficulty:** 🔴 Advanced

## Problem Description

A coffee shop chain wants a handful of standard sales-analytics answers out
of its point-of-sale data: how many transactions happened, how much revenue
they added up to, how each individual transaction should be sized ("Single"
vs. "Double" vs. "Bulk" cup order), a running total of cups sold over time,
and a revenue-by-category breakdown across its product catalog.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `transactions` | व्यवहाराः | `transaction_id` (PK), `transaction_date`, `transaction_time`, `transaction_qty`, `store_id` (FK → stores), `product_id` (FK → products) |
| `products` | उत्पादाः | `product_id` (PK), `unit_price`, `product_category`, `product_type` |
| `stores` | आपणाः | `store_id` (PK), `store_location` |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground/coffee-shop-sales) — "Coffee Shop Sales," transaction records for Maven Roasters, a fictitious coffee chain across three NYC locations (~149k rows in the full dataset). Only a small subset of columns is carried over here, and the rows in `seed.सं` are a small hand-built sample (30 transactions), not the Maven data.

## SQL/SSQL Concepts Demonstrated

- Basic aggregate `SELECT` with `COUNT` and no grouping (Objective 1)
- Multi-table `JOIN` with `SUM` over a computed expression (`quantity × unit_price`) (Objective 2)
- `CASE` (वरणम्) expression for row-level classification (Objective 3)
- Window function: `SUM() OVER (ORDER BY ...)` for a running total (Objective 4)
- `JOIN` + `GROUP BY` + aggregate for a category rollup (Objective 5)

## Objectives

1. **Objective 1 — Total transaction count**: Count the total number of transactions.
2. **Objective 2 — Total revenue**: Compute total revenue (`transaction_qty × unit_price`, summed) across all transactions joined to products.
3. **Objective 3 — Transaction size classification**: Classify each transaction as "Single," "Double," or "Bulk" based on its quantity.
4. **Objective 4 — Running total of quantity**: For each transaction, return a running total of quantity ordered by date.
5. **Objective 5 — Category sales**: Total quantity sold per product category, across transactions joined to products.

## Deliverables

- Native SSQL queries under [`coffee_queries/`](coffee_queries/) (`total_sales.सं`, `total_revenue.सं`, `transaction_size.सं`, `running_total.सं`, `category_sales.सं`). No `.sql` files are committed; the SQL for either dialect is generated from these queries by the compiler.
- Schema as native SSQL (`schema.सं`) and sample rows in `seed.सं`.

## Status

- **Objective 1** (`total_sales.सं`): compiles to `SELECT COUNT(t1.transaction_id) FROM transactions t1;` for both SQLite and PostgreSQL; returns 30 against the seed data.
- **Objective 2** (`total_revenue.सं`): compiles for both dialects to a `SUM(transaction_qty * unit_price)` over `transactions` joined to `products` (PostgreSQL additionally wraps the `SUM` in `CAST(... AS DOUBLE PRECISION)`); returns 225.25 against the seed data.
- **Objective 3** (`transaction_size.सं`): compiles to the same SQL for both dialects — a `CASE` labelling quantity 1 as `Single`, 2 as `Double`, and anything else as `Bulk`.
- **Objective 4** (`running_total.सं`): compiles to the same SQL for both dialects — `SUM(transaction_qty) OVER (ORDER BY transaction_date ASC, transaction_id ASC)`. The transaction number breaks ties within a date, so each transaction has its own running total.
- **Objective 5** (`category_sales.सं`): compiles for both dialects (PostgreSQL additionally wraps the `SUM` in `CAST(... AS DOUBLE PRECISION)`); against the seed data it returns Bakery 10, Coffee 26, Drinking Chocolate 7, Tea 17.
