# Project: Ecommerce Monthly Insights

**Domain:** Finance · **Difficulty:** 🟢 Basic

## Problem Description

A finance team tracks monthly actuals-vs-budget figures across regions in a
flat P&L-style ledger: type (Actual/Budget), month, year, region, product
category, volume, gross sales, deductions, and a chain of cost line items.
For a single reporting cycle — actuals for September 2023 — they want a
per-region rollup: total volume, total gross sales, and a derived net-revenue
figure (gross sales minus deductions), ordered so the highest-revenue
regions surface first.

Despite the directory's name, this is a general finance/FP&A ledger, not an
ecommerce orders/customers schema — there is no product-purchase, order, or
customer entity anywhere in it.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `finance_data` | वित्तपत्राणि | `id` (PK), `type`, `month`, `year`, `region`, `category`, `product`, `volume`, `gross_sales`, `deductions`, `cogs`, `other_direct_exp`, `selling_exp`, `depreciation`, `ga_exp`, `rnd_exp`, `overheads` |

**Source:** No external dataset — an example scenario invented for this
repo. The generic Actual/Budget, region-breakdown, gross-to-net revenue
shape resembles common FP&A tutorial datasets (e.g. Microsoft's "Financial
Sample" workbook used in Power BI tutorials), but it is not derived from
that or any other named dataset. The schema is declared in `schema.सं`, and
sample rows are in `seed.सं`.

## SQL/SSQL Concepts Demonstrated

- Named formula declaration (`इति सूत्रम्`) — a derived column (net revenue
  = gross sales − deductions) declared once, ahead of the query, and reused
  in the SELECT list
- Multi-condition `WHERE` filter joined with `च` (AND) across three columns
- `GROUP BY` (`इति समूहेन`) with multiple aggregates (`योगः` / SUM) over
  both raw and formula-derived columns
- `ORDER BY` descending (`अवरोहः ... क्रमेण`) on an aggregate expression

## Objectives

1. **Objective 1 — September 2023 actuals by region**: For actual (not
   budget) records dated September 2023, return each region's total volume,
   total gross sales, and total net revenue, ordered by gross sales
   descending.

## Deliverables

- A single native SSQL query, `monthly_insights.सं`, which sits directly in
  the project root (there is no `queries/` subdirectory).
- No `.sql` file is committed; the SQL for either dialect is generated from
  `monthly_insights.सं` by the compiler.
- `schema.सं` and `seed.सं` — the table definition and sample rows.

## Status

- **Objective 1** (`monthly_insights.सं`): compiles for both SQLite and
  PostgreSQL (PostgreSQL additionally wraps each `SUM` in `CAST(... AS
  DOUBLE PRECISION)`). Against the seed data it returns two regions, ordered
  by gross sales: EMEA (gross sales 11500, volume 35, net revenue 10350),
  then APAC (8000, 20.5, 7200).
