# Project 06: Pizza Throughput

**Domain:** Food · **Difficulty:** 🟢 Basic

## Problem Description

A pizzeria manager wants to understand order throughput across the day —
which hours the kitchen is busiest — so staffing can be matched to demand.
Given a log of orders (each with a date and a timestamp), the manager wants
a simple count of orders placed in each hour of the day.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `orders` | आदेशाः | `order_id` (PK), `date`, `time` |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground)
— the "Maven Pizza Challenge," one of Maven Analytics' dated "Challenge"
datasets (posed on its own Challenge Page with a specific business question
attached, rather than a plain passive practice download). The real
challenge dataset spans a full year of pizzeria activity across four
related tables (`orders`, `order_details`, `pizzas`, `pizza_types`) with
tens of thousands of order rows. This project keeps only the `orders`
table's `date`/`time` columns and shrinks it to a hand-picked 5-row fixture
(all on `2023-01-01`, at 09:15, 09:45, 10:05, 12:30, 12:55) — enough to
populate three distinct hour buckets and demonstrate the grouping pattern,
nothing more.

## SQL/SSQL Concepts Demonstrated

- Date/time part extraction (`होरा` → `HOUR`) on a `TEXT`-typed time column
- `GROUP BY` on a computed (non-column) expression
- Aggregation with `COUNT`

## Objectives

1. **Objective 1 — Hourly order counts**: Group orders by the hour of the
   order time and count how many orders fall in each hour.

## Deliverables

- Native SSQL query (`queries/q1_throughput.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_throughput.सं`): compiles for both SQLite and
  PostgreSQL. Against the seed data it returns 3 rows (`9`→2, `10`→1,
  `12`→2); the hour is an integer, not a zero-padded string. The two
  dialects extract the hour differently: SQLite emits
  `CAST(strftime('%H', t1.time) AS INTEGER)`, while PostgreSQL emits
  `CAST(EXTRACT(HOUR FROM (…)::time) AS INTEGER)` — the compiler casts the
  `TEXT`-typed source column to `::time` first.
