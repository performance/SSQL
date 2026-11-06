# Project 09: Startup Funding

**Domain:** Finance · **Difficulty:** 🟡 Intermediate

## Problem Description

A venture analyst is building a clean investor report, but the raw data
uses the literal placeholder string `"Undisclosed"` for investors who
chose not to be named. The analyst wants a display-friendly version of the
data where every `"Undisclosed"` investor name reads as `"Anonymous"`
instead, alongside each investment's funding amount.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `investors` | निवेशकाः | `investor_id` (PK), `investor_name`, `funding_amount` |

**Source:** Kaggle — a Crunchbase-derived startup funding dataset. The
most plausible match by name and theme is
[`arindam235/startup-investments-crunchbase`](https://www.kaggle.com/datasets/arindam235/startup-investments-crunchbase)
("StartUp Investments (Crunchbase)," ~54,294 rows × 39 columns of
company-level funding-round data derived from Crunchbase, including
investor-name fields among its many columns) ; that dataset is
company-centric with dozens of columns, so it is the closest real dataset
in this theme rather than an exact source. This project reduces that shape to a 4-row fixture: 4 investors,
2 of which are literally named `"Undisclosed"` in the source data — a real
placeholder convention in Crunchbase-style investor data, which is exactly
what this project's cleaning step targets.

## SQL/SSQL Concepts Demonstrated

- `CASE`/`वरणम्` expression with an equality-test branch
  (`यस्मिन् ... तस्मिन् ...`) and a default branch (`अन्यस्मिन् ...`)
- String-literal equality comparison
- Column aliasing to an English name (`"clean_name" इति आख्या`)

Note: the mechanism here is `CASE`/`वरणम्`, not `COALESCE`. `COALESCE` only substitutes on `NULL`; replacing a specific
non-null string literal like `"Undisclosed"` requires an equality check,
which only `CASE` (or an equivalent conditional) expresses. The compiled
output confirms this: `CASE WHEN t1.investor_name = 'Undisclosed' THEN
'Anonymous' ELSE t1.investor_name END`.

## Objectives

1. **Objective 1 — Clean investor names**: Replace `"Undisclosed"`
   investor names with `"Anonymous"` and return the cleaned name alongside
   each row's funding amount.

## Deliverables

- Native SSQL query (`queries/q1_funding_clean.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_funding_clean.सं`): compiles to the same SQL for
  SQLite and PostgreSQL and returns 4 rows against the seed data: Sequoia
  Capital / 5000000, Anonymous / 1220000, Y Combinator / 150000, Anonymous
  / 3000000.
