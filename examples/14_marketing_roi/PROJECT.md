# Project 14: Marketing ROI

**Domain:** Marketing · **Difficulty:** 🟢 Basic

## Problem Description

A marketing team ran several promotional campaigns, and the resulting promo
codes were logged inconsistently — mixed case, with some entries carrying
stray leading/trailing whitespace from manual entry or spreadsheet
copy-paste. Before analysts can join these codes against any other
campaign-tracking table, they want each code's case normalized, alongside
its ID and spend, so it can be matched and reported on reliably.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `promotions` | प्रचाराः | `promo_id` (PK), `promo_code`, `cost` |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground) — the "Maven Marketing Challenge," a free practice challenge dataset (marketing campaign/promo-code performance data). This project uses a small hand-picked subset: a single 3-column `promotions` table with 3 seed rows (`promo_id`, `promo_code`, `cost`), not the full challenge's broader schema or row count.

## SQL/SSQL Concepts Demonstrated

- Scalar string function invocation (`UPPER`, via SSQL's `महावर्णरूपम्`) applied to a column and aliased in the `SELECT` list
- Genitive-case ("of X") access to a table's column from a projection position (`प्रचारस्य सङ्केतपदस्य` — "of the promotion, the promo-code('s uppercase form)")
- Multi-column projection with plural verb agreement (three output columns → `आह्रियन्ताम्`)

## Objectives

1. **Objective 1 — Promo code case normalization**: For every promotion,
   return its ID, cost, and promo code converted to uppercase.

## Deliverables

- Native SSQL query (`queries/q1_marketing.सं`).
- A step-by-step walkthrough (`GUIDE.md`).

## Status

- **Objective 1** (`q1_marketing.सं`): compiles to the same SQL for SQLite
  and PostgreSQL: `SELECT upper(t1.promo_code) AS cleaned_code, t1.promo_id,
  t1.cost FROM promotions t1;`. Against the seed data, e.g.
  `"   promo123   "` → `"   PROMO123   "`, with leading/trailing spaces
  preserved.
