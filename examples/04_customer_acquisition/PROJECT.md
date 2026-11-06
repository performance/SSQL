# Project 04: Customer Acquisition

**Domain:** CRM · **Difficulty:** 🟡 Intermediate

## Problem Description

A business wants to know which customers in its CRM have never actually
converted — that is, which ones have zero orders on record. This requires
looking at every customer regardless of whether they match anything in the
orders table, then keeping only the ones with no match at all.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `customers` | ग्राहकाः | `customer_id` (PK), `first_name`, `last_name`, `email` |
| `orders` | आदेशाः | `order_id` (PK), `order_date`, `customer_id` (FK → customers), `total_amount` |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground). No single dataset on the playground is titled "Customer Acquisition" — this project instead models the generic customers+orders CRM shape shared by several of the playground's CRM/sales datasets, purpose-built to teach the "find unmatched rows via `LEFT JOIN` + `IS NULL`" pattern rather than to reproduce one specific named dataset. It's a small hand-built fixture: 3 customers and 2 orders, with one customer (Aditya Rao) deliberately left with zero orders so Objective 1 has a real, non-empty answer to find.

## SQL/SSQL Concepts Demonstrated

- `LEFT OUTER JOIN` (SSQL's वाममेलनेन)
- Anti-join pattern: filtering for unmatched rows via `IS NULL` on the joined side's key (SSQL's रिक्ता अस्ति)

## Objectives

1. **Objective 1 — Unacquired customers**: Find every customer with zero orders, by left-joining customers to orders and keeping only rows where the order side is `NULL`.

## Deliverables

- Native SSQL query (`queries/q1_unacquired_customers.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_unacquired_customers.सं`): compiles to the same SQL for SQLite and PostgreSQL. Against the seed data it returns exactly one row — Aditya (`aditya@example.org`), the only one of the 3 customers with no matching order.
