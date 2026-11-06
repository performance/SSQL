# Project: Pizza Sales

**Domain:** Food & Beverage · **Difficulty:** 🟡 Intermediate

## Problem Description

A pizza restaurant wants basic order analytics out of its point-of-sale
data: how many orders it has taken in total, how much total revenue those
orders add up to, how sales break down by pizza category, how order volume
is distributed across hours of the day, and which pizza types sell the most
by quantity.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `orders` | आदेशाः | `order_id` (PK), `date`, `time` |
| `order_details` | आदेशविवरणानि | `order_details_id` (PK), `order_id` (FK → orders), `pizza_id` (FK → pizzas), `quantity` |
| `pizzas` | पिष्टकानि | `pizza_id` (PK), `pizza_type_id` (FK → pizza_types), `size`, `price` |
| `pizza_types` | पिष्टकप्रकाराः | `pizza_type_id` (PK), `name`, `category`, `ingredients` |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground/pizza-place-sales) — "Pizza Place Sales," annual sales data for a fictitious pizza place (4 tables — `orders`, `order_details`, `pizzas`, `pizza_types` — roughly 21k orders / 48k order-detail rows / 33 pizza types in the full dataset). This project's `schema.सं` follows the dataset's four tables column for column. The rows in `seed.सं` are a small hand-built sample (20 orders), not the Maven data. This project is separate from the numbered project `06_pizza_throughput`, which shares the theme but uses a smaller, single-table schema.

## SQL/SSQL Concepts Demonstrated

- Basic aggregate `SELECT` with `COUNT` and no grouping (Objective 1)
- 2-table `JOIN` with `SUM` over a computed expression (`quantity × price`) (Objective 2)
- 3-table `JOIN` + `GROUP BY` + `SUM`, for a category rollup (Objective 3)
- Date/time function extraction (hour-of-day bucketing) + `GROUP BY` + `COUNT` (Objective 4)
- 3-table `JOIN` + `GROUP BY` + `ORDER BY DESC` + `LIMIT`, for top-N ranking (Objective 5)

## Objectives

1. **Objective 1 — Total order count**: Count the total number of orders.
2. **Objective 2 — Total revenue**: Compute total revenue (`quantity × price`, summed) across all order line items joined to pizzas.
3. **Objective 3 — Category sales**: Total quantity sold per pizza category, joining order details through pizzas to pizza types.
4. **Objective 4 — Hourly order volume**: Count of orders bucketed by hour of day.
5. **Objective 5 — Top-selling pizzas**: The 5 pizza types with the highest total quantity sold, descending.

## Deliverables

- Native SSQL queries under [`pizza_queries/`](pizza_queries/) (`total_orders.सं`, `total_revenue.सं`, `category_sales.सं`, `hourly_orders.सं`, `top_selling.सं`). No `.sql` files are committed; the SQL for either dialect is generated from these queries by the compiler.
- Schema as native SSQL (`schema.सं`) and sample rows in `seed.सं`.

## Status

- **Objective 1** (`total_orders.सं`): compiles to `SELECT COUNT(t1.order_id) FROM orders t1;` for both SQLite and PostgreSQL; returns 20 against the seed data.
- **Objective 2** (`total_revenue.सं`): compiles for both dialects to a `SUM(quantity * price)` over `order_details` joined to `pizzas` (PostgreSQL additionally wraps the `SUM` in `CAST(... AS DOUBLE PRECISION)`); returns 1167.18 against the seed data.
- **Objective 3** (`category_sales.सं`): compiles for both dialects to a three-table join grouped by category; against the seed data it returns Chicken 16, Classic 30, Supreme 14, Veggie 22.
- **Objective 4** (`hourly_orders.सं`): compiles for both dialects, extracting the hour as an integer with `CAST(strftime('%H', t1.time) AS INTEGER)` on SQLite and `CAST(EXTRACT(HOUR FROM (…)::time) AS INTEGER)` on PostgreSQL; against the seed data it returns nine hourly buckets between 11:00 and 21:00.
- **Objective 5** (`top_selling.सं`): compiles for both dialects to a three-table join with `GROUP BY`, `ORDER BY SUM(quantity) DESC` and `LIMIT 5`; against the seed data Pepperoni (12) ranks first.
