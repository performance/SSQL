# Project 13: Portfolio Exits

**Domain:** Finance · **Difficulty:** 🔴 Advanced

## Problem Description

A venture fund tracks its portfolio of unicorn (>$1B) investments in one
table, and records each exit (an acquisition or IPO) in a separate table as
it happens. The fund's portfolio team wants a standing report of which
portfolio companies are still active investments — i.e., every unicorn that
has no matching row yet in the exits table.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `unicorns` | एकशृङ्गाः | `company_id` (PK), `company` |
| `exits` | निर्गमाः | `exit_id` (PK), `company`, `exit_value` (real) |

**Source:** Kaggle, Crunchbase-derived — same general family as project 09's (`09_startup_funding`) investor/funding data. Datasets in this general family include ["StartUp Investments (Crunchbase)"](https://www.kaggle.com/datasets/arindam235/startup-investments-crunchbase) (~50k companies, funding rounds, market/industry, 1902–2014) and ["Startup Success/Fail Dataset from Crunchbase"](https://www.kaggle.com/datasets/yanmaksi/big-startup-secsees-fail-dataset-from-crunchbase) (labels success as M&A/IPO), but neither matches this project's minimal `unicorns`/`exits` pair column for column, so no specific dataset is claimed as the source. This project is a small hand-built fixture: 3 unicorns (Stripe, SpaceX, ByteDance), 1 exit record (Stripe, `exit_value` 95000.0 — the fixture doesn't state a unit).

## SQL/SSQL Concepts Demonstrated

- Two single-column CTEs, each projecting `company` from a different table
- `EXCEPT` (वर्जनम्) set-difference between the two CTEs, as a set-based alternative to a `NOT IN`/anti-join
- Comparing rows across tables purely by matching column values, with no explicit `JOIN` condition

## Objectives

1. **Objective 1 — Still-active portfolio companies**: Return every company present in `unicorns` that does not also appear in `exits`.

## Deliverables

- Native SSQL query (`queries/q1_exits.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_exits.सं`): compiles to the same SQL for SQLite and PostgreSQL: `WITH एकशृङ्गनामन् AS (SELECT t1.company FROM unicorns t1), निर्गमनामन् AS (SELECT t2.company FROM exits t2) SELECT * FROM एकशृङ्गनामन् EXCEPT SELECT * FROM निर्गमनामन्;` Against the seed data it returns SpaceX and ByteDance (Stripe is excluded, being the one company present in `exits`).
