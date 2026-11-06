# Project 07: Restaurant Orders

**Domain:** Food · **Difficulty:** 🟡 Intermediate

## Problem Description

A restaurant's finance team wants to flag unusually large tickets for
review — order-line items priced well above what a typical order costs.
Given a log of order-detail line items (one row per priced item on an
order), the team wants every line item whose price exceeds the average
price across all line items.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `order_details` | आदेशविवरणानि | `order_details_id` (PK), `order_id`, `price` |

**Source:** Modeled on the "order details" table shape
(`order_details_id` / `order_id` / `price`) common to several Kaggle
restaurant-orders datasets (e.g. `mohamedharris/restaurant-order-details`
and `yogeshpandey1/restaurant-order-detail-dataset`); the data is not taken
from any one of them. It's a hand-built 5-row fixture (three orders, one with two line
items), enough to produce a real above/below-average split.

## SQL/SSQL Concepts Demonstrated

- Scalar subquery in a `WHERE` clause (`SELECT AVG(price) FROM ...`)
- Comparison against a subquery result (अधिकम्/"greater than")
- The `इति नियमेन` marker for a `WHERE`-clause predicate

## Objectives

1. **Objective 1 — Above-average tickets**: Filter order-detail records
   whose price exceeds the global average price across all order details
   (a subquery-based comparison).

## Deliverables

- Native SSQL query (`queries/q1_large_tickets.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_large_tickets.सं`): compiles for both SQLite and
  PostgreSQL (PostgreSQL casts the subquery's `AVG` to `DOUBLE PRECISION`).
  The average price across the 5 seed rows is ≈20.998; the query returns
  the 2 rows above it — (`order_details_id=3, order_id=2, price=45.00`) and
  (`order_details_id=5, order_id=3, price=22.50`).
