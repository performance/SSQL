# Project: Restaurant Orders

**Domain:** Food & Beverage · **Difficulty:** 🟡 Intermediate

## Problem Description

A restaurant wants basic menu and order analytics: how many items are on
the menu, which items get ordered most and least often, which orders
carried the highest total spend, how menu items are distributed across
cuisine categories, and how to classify each menu item into a simple price
tier for merchandising purposes.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `menu_items` | व्यञ्जनानि | `menu_item_id` (PK), `item_name`, `category`, `price` |
| `order_details` | आदेशविवरणानि | `order_details_id` (PK), `order_id`, `order_date`, `order_time`, `item_id` (FK → menu_items) |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground/restaurant-orders) — "Restaurant Orders" (guided project: "Restaurant Order Analysis," by Alice Zhao), order data for a fictitious international-cuisine restaurant (32 menu items, ~12.2k ordered items across ~5.4k distinct orders in the full dataset). Only a subset of columns is carried over here, and the rows in `seed.सं` are a small hand-built sample (12 menu items), not the Maven data.

## SQL/SSQL Concepts Demonstrated

- Basic aggregate `SELECT` with `COUNT` and no grouping (Objective 1)
- `JOIN` + `GROUP BY` + `COUNT` + `ORDER BY DESC`, for popularity ranking (Objective 2)
- `JOIN` + `GROUP BY` + `SUM` + `ORDER BY DESC` + `LIMIT`, for top-N spend ranking (Objective 3)
- `GROUP BY` + `COUNT`, for a category distribution (Objective 4)
- `CASE` (वरणम्) expression with multiple thresholds, for 3-way price classification (Objective 5)

## Objectives

1. **Objective 1 — Total menu items**: Count the total number of items on the menu.
2. **Objective 2 — Item popularity**: For each menu item, return its name and order count, most-ordered first.
3. **Objective 3 — High-value orders**: The 5 orders with the highest total spend, descending.
4. **Objective 4 — Category distribution**: Count of menu items in each category.
5. **Objective 5 — Spend classification**: Classify each menu item as "Cheap," "Medium," or "Expensive" based on price thresholds.

## Deliverables

- Native SSQL queries under [`restaurant_queries/`](restaurant_queries/) (`total_items.सं`, `item_popularity.सं`, `high_value_orders.सं`, `category_distribution.सं`, `spend_classification.सं`). No `.sql` files are committed; the SQL for either dialect is generated from these queries by the compiler.
- Schema as native SSQL (`schema.सं`) and sample rows in `seed.सं`.

## Status

- **Objective 1** (`total_items.सं`): compiles to `SELECT COUNT(t1.menu_item_id) FROM menu_items t1;` for both SQLite and PostgreSQL; returns 12 against the seed data.
- **Objective 2** (`item_popularity.सं`): compiles to the same SQL for both dialects — `order_details` joined to `menu_items`, grouped by item name and ordered by order count descending. Against the seed data the most-ordered items have 4 orders each.
- **Objective 3** (`high_value_orders.सं`): compiles for both dialects (PostgreSQL additionally wraps the `SUM` in `CAST(... AS DOUBLE PRECISION)`) to a join grouped by order, ordered by total price descending, `LIMIT 5`. Against the seed data the top order is order 10, at 100.96.
- **Objective 4** (`category_distribution.सं`): compiles to the same SQL for both dialects; against the seed data it returns Appetizer 3, Beverage 2, Dessert 3, Entree 4.
- **Objective 5** (`spend_classification.सं`): compiles to the same SQL for both dialects — a `CASE` labelling prices below 10 as `Cheap`, below 20 as `Medium`, and everything else as `Expensive`.
