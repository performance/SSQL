# Project 12: Unicorn Valuation

**Domain:** Venture Capital · **Difficulty:** 🟡 Intermediate

## Problem Description

Maven Analytics posed the "Maven Unicorn Challenge" as an open-ended
dashboard exercise: given a global list of unicorn (>$1B valuation)
companies, tell a story about the landscape of unicorn companies however
you see fit. This project narrows that open brief to one concrete
analytical question: which industries have produced a cohort of unicorns
large enough to be worth comparing, and how does their combined valuation
rank?

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `unicorns` | एकशृङ्गाः | `company_id` (PK), `company`, `valuation` (real), `industry`, `country` |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground) — the "Maven Unicorn Challenge" dataset ("Unicorn Companies," ~1,074 real companies with valuation, total funding, industry, country, select investors, and founding year). This project uses a small hand-picked subset: a 9-row fixture covering two industries (6 Fintech, 3 Health) across four countries, keeping only company, valuation, industry, and country — enough to make the `HAVING count > 5` filter select exactly one qualifying group (Fintech). The original Maven Unicorn Challenge was a free-form "build a report/dashboard" competition (see Maven Analytics' [Maven Unicorn Challenge announcement](https://mavenanalytics.io/blog/maven-unicorn-challenge)) rather than a fixed question set; this project's `GROUP BY`/`HAVING`/`ORDER BY` query is one concrete question drawn from that open brief, not the challenge's own prescribed deliverable.

## SQL/SSQL Concepts Demonstrated

- `GROUP BY` with a `SUM()` aggregate
- `HAVING` filtering on a `COUNT()` over the grouped rows (समूहनियमेन)
- `ORDER BY` on a computed aggregate expression, descending
- A real-valued/decimal (`वास्तविकः`) valuation column

## Objectives

1. **Objective 1 — Unicorn industries by valuation**: Group unicorn companies by industry, keep only industries with more than 5 companies, and return each qualifying industry's total valuation, ordered highest first.

## Deliverables

- Native SSQL query (`queries/q1_unicorns.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_unicorns.सं`): compiles for both SQLite and PostgreSQL to `SELECT SUM(valuation) AS total_valuation, industry FROM unicorns GROUP BY industry HAVING COUNT(company) > 5 ORDER BY SUM(valuation) DESC` (PostgreSQL wraps both `SUM(valuation)` occurrences in `CAST(... AS DOUBLE PRECISION)`). Against the 9-row fixture it returns exactly one row (Fintech, 6 companies).
