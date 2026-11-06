# Project 29: Cart Abandonment Analysis

**Domain:** Retail / E-Commerce · **Difficulty:** 🟡 Intermediate

## Problem Description

An e-commerce team wants to find shoppers who showed clear purchase intent —
they added something to a shopping cart — but never actually completed an
order. This "cart abandonment" list is a classic retention target: every
user on it is a candidate for a follow-up nudge (reminder email, discount),
since they got partway through the funnel and then stopped.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `shopping_carts` | क्रयणशकटाः | `cart_id` (PK), `user_id` |
| `completed_orders` | पूर्णादेशाः | `order_id` (PK), `user_id` |

**Source:** No external public dataset. The scenario was invented for this example. It's a hand-built, minimal two-table,
bare-id-pair fixture (3 cart rows across 3 users, 1 completed order),
purpose-built to demonstrate `EXCEPT` set difference. For context, real
cart-abandonment datasets — e.g. Kaggle's `dynamo21/cart-abandonment` or the
REES46 e-commerce events dataset — carry wide session/behavior-feature
tables (device, timestamps, product categories, event sequences); nothing
here is derived from them, and this schema is deliberately reduced to just
the two `user_id` columns that `EXCEPT` needs.

## SQL/SSQL Concepts Demonstrated

- Set-difference queries via `EXCEPT` (SSQL: वर्जनेन/वर्जनम्) between two
  independently-scoped `SELECT`s
- Named subquery aliasing (आख्या) on each side of the set operation, which
  the compiler lowers into a `WITH ... AS (...)` common table expression
  per side

## Objectives

1. **Objective 1 — Cart abandoners**: Return every user who added an item
   to a shopping cart but never went on to complete an order.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles
  for both SQLite and PostgreSQL, identical generated SQL across both
  dialects. The actual generated SQL wraps each
  side of the `EXCEPT` in a named CTE:
  ```sql
  WITH क्रयणशकटनामन् AS (
  SELECT
      t1.user_id
  FROM
      shopping_carts t1
  ),
  पूर्णादेशनामन् AS (
  SELECT
      t2.user_id
  FROM
      completed_orders t2
  )
  SELECT * FROM क्रयणशकटनामन् EXCEPT SELECT * FROM पूर्णादेशनामन्;
  ```
  This directory's `GUIDE.md` shows the same SQL. Executed against the seed data (cart
  users 101/102/103; one completed order, for 101), the query returns users 102 and 103.
