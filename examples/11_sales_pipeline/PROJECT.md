# Project 11: B2B Sales Pipeline

**Domain:** CRM · **Difficulty:** 🔴 Advanced

## Problem Description

A B2B hardware/software sales organization tracks every deal ("opportunity")
in its CRM pipeline from first customer engagement to close. Sales
leadership wants to measure "sales velocity" — how many days a deal
actually takes to close once it's won — broken out by product line, so
reps and managers can see where deals move fastest and where they drag.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `opportunities` | अवसराः | `opportunity_id` (PK), `product`, `deal_stage`, `engage_day` (int), `close_day` (int) |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground) — "B2B Sales Pipeline" (also distributed there as "CRM Sales Opportunities," and the basis of the Maven Sales Challenge). The real dataset is a multi-table CRM extract (separate accounts, products, sales-team, and opportunity tables, with real dates); this project uses a small hand-picked subset — a single 5-row `opportunities` fixture carrying just product, deal stage, and engage/close day-numbers in place of real calendar dates.

## SQL/SSQL Concepts Demonstrated

- Common Table Expression (CTE) definition, filtering rows (`deal_stage = "Won"`) before the outer query sees them
- Native ablative-case subtraction (`अन्तदिनात् प्रारम्भदिनम् विशोध्य अवशिष्टम्`) to compute a day-count inside the CTE
- `GROUP BY` + `AVG()` over the CTE's output, aliased to an English output column (`avg_days`)

## Objectives

1. **Objective 1 — Sales velocity**: For each product, compute the average number of days between engagement and close among deals that were won.

## Deliverables

- Native SSQL query (`queries/q1_velocity.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_velocity.सं`): compiles for both SQLite and PostgreSQL: a `WITH ... AS (...)` CTE computing `close_day - engage_day` filtered to `deal_stage = 'Won'`, then `AVG(...) AS avg_days ... GROUP BY product` over the CTE (PostgreSQL additionally wraps the `AVG()` in `CAST(... AS DOUBLE PRECISION)`). Against the seed data it returns Hardware 20.0 and Software 17.5.
