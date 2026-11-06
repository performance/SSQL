# Project 01: Maven Toys Sales

**Domain:** Retail · **Difficulty:** 🟢 Basic

## Problem Description

Maven Toys operates a small chain of toy stores. Each store sells a catalog of
products, and every purchase is recorded as a sale (a customer buying some
quantity of one product at one store on one date). Management wants two basic
analytical answers out of this data: a simple product catalog listing, and a
per-store revenue rollup.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `stores` | क्रीडनकभाण्डाराः | `store_id` (PK), `store_name`, `store_city`, `store_location`, `store_open_date` |
| `products` | उत्पादाः | `product_id` (PK), `product_name`, `product_category`, `product_cost`, `product_price` |
| `sales` | विक्रयाः | `sale_id` (PK), `date`, `store_id` (FK → stores), `product_id` (FK → products), `units` |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground) — "Maven Toys Sales," a free practice dataset (stores/products/sales schema, a few thousand transactions). This project uses a small hand-picked subset of that schema and data, not the full dataset.

## SQL/SSQL Concepts Demonstrated

- Basic `SELECT` with column projection (Objective 1)
- Multi-table `JOIN` (sales ⋈ stores, sales ⋈ products) (Objective 2)
- Aggregation with `GROUP BY` and `SUM` over a computed expression (`units × price`) (Objective 2)

## Objectives

1. **Objective 1 — Product catalog**: Retrieve every product's name, cost, and price.
2. **Objective 2 — Revenue per store**: Calculate total revenue (units sold × product price, summed) for each store.

## Deliverables

For each objective, this project provides:
- A native SSQL query (`queries/q*.सं`), each with a header docstring stating its objective and standard-SQL equivalent.
- A plain-SQL solution guide covering both target dialects ([`SOLUTION_SQL.md`](SOLUTION_SQL.md)).
- A detailed SSQL solution guide explaining the grammatical/vibhakti reasoning behind the query ([`SOLUTION_SSQL.md`](SOLUTION_SSQL.md)).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_products.सं`): compiles to the same SQL for SQLite and PostgreSQL — a three-column projection from `products` — and returns all 4 products in the seed data.
- **Objective 2** (`q2_revenue_per_store.सं`): compiles for both dialects (PostgreSQL additionally wraps the `SUM` in `CAST(... AS DOUBLE PRECISION)`). Against the seed data it returns one row per store, revenue first and then store name: 459.81 Downtown Toys, 119.85 Metro Playroom, 169.95 Suburbia Play.
