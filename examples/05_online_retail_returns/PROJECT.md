# Project 05: Online Retail Returns

**Domain:** Retail · **Difficulty:** 🟡 Intermediate

## Problem Description

An online retailer records every line item as a transaction, but cancelled
orders are logged as ordinary transactions whose transaction code happens to
start with the letter "C" (and whose quantity is negative). An analyst wants
to flag those cancellation/return rows via that code convention and compute
the two totals needed for a return rate: quantity returned versus overall
quantity transacted.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `transactions` | व्यवहाराः | `transaction_id` (PK), `transaction_code`, `product_id`, `quantity`, `price`, `customer_id` |

**Source:** [UCI Machine Learning Repository — "Online Retail"](https://archive.ics.uci.edu/dataset/352/online+retail), a well-known real transactional dataset (541,909 line-item invoice records from a UK-based online retailer, 1 Dec 2010 – 9 Dec 2011) whose defining convention — an `InvoiceNo` prefixed with the letter "C" marks a cancellation, typically paired with a negative `Quantity` — is exactly what this project's `transaction_code` / `LIKE 'C%'` check models. This project flattens the real dataset's columns (`InvoiceNo`, `StockCode`, `Quantity`, `UnitPrice`, `CustomerID`, plus `Description`/`InvoiceDate`/`Country`, which are dropped here) into a single `transactions` table and uses a small hand-picked 5-row fixture — 3 ordinary transactions and 2 "C"-prefixed cancellations — not the full ~542K-row dataset.

## SQL/SSQL Concepts Demonstrated

- `CASE`/`WHEN`/`THEN`/`ELSE` conditional expression (SSQL's वरणम् / यस्मिन् / तस्मिन् / अन्यस्मिन्)
- Conditional aggregation: `SUM(CASE WHEN ... THEN ... ELSE 0 END)`
- Pattern matching with `LIKE` (SSQL's `सदृशः अस्ति`, agreeing with the masculine column `सङ्केतः`)
- Required grouping with `यत् … इति` around a `CASE` expression before summing it — without it, `इत्यस्य योगः` binds to the nearest preceding value (the ELSE-branch `०` alone) instead of the whole `वरणम्...अन्यस्मिन्` expression, producing `CASE WHEN ... THEN quantity ELSE SUM(0) END` instead of the intended `SUM(CASE WHEN ... THEN quantity ELSE 0 END)` (see `GUIDE.md`).

## Objectives

1. **Objective 1 — Return-rate components**: For every transaction, treat a `transaction_code` starting with "C" as a return; compute the quantity returned (returns are stored as negative quantities, so `ऋणात्मिका` turns them back into a count) alongside the quantity sold (the other transactions). (The query returns these two totals side by side — the numerator and denominator of a return rate — rather than a single pre-divided ratio.)

## Deliverables

- Native SSQL query (`queries/q1_return_rate.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_return_rate.सं`): compiles for both SQLite and PostgreSQL (structurally identical SQL; PostgreSQL wraps both `SUM`s in `CAST(... AS DOUBLE PRECISION)`). Against the seed data it returns `3` returned (the two "C"-coded rows, stored as -2 and -1) against `16` sold (10 + 5 + 1), a return rate of 18.75%.
