# Project 26: Customer Cohorts

**Domain:** SaaS / Customer Analytics · **Difficulty:** 🔴 Advanced

## Problem Description

A SaaS product team wants to build customer cohorts for retention
analysis — grouping users by the period of their very first purchase, so
later behavior can be compared cohort by cohort. The foundational step for
that is answering, for every order row, "what was this user's very first
order date?" without collapsing the order history down to one row per
user. Given a raw order log, tag every row with its user's first order
date so downstream cohort bucketing can build on it.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `user_orders` | ग्राहकादेशः | `order_id` (PK), `user_id`, `order_date` (text, ISO-format date string) |

**Source:** No external public dataset. The scenario was invented for this example. It's a hand-built, minimal 3-row fixture
(2 users, one with two orders) purpose-built to demonstrate one SQL
feature: a partitioned window `MIN()`. For context, `MIN(order_date) OVER (PARTITION BY
user_id)` is itself one of the most common, dataset-agnostic patterns in
cohort-analysis tutorials generally — nothing here is tied to any
particular real dataset.

## SQL/SSQL Concepts Demonstrated

- Window function `MIN(...) OVER (PARTITION BY ...)` — a per-group
  aggregate computed without collapsing rows via `GROUP BY`
- The canonical first step of cohort/retention analysis: tagging every row
  with its group's earliest date

## Objectives

1. **Objective 1 — First order date per user**: For every order, return
   the user id and order date alongside that user's earliest order date
   across all their orders, computed via a partitioned window `MIN()`.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles for both SQLite and PostgreSQL, identical generated SQL across both
  dialects:
  ```sql
  SELECT
      t1.user_id,
      t1.order_date,
      MIN(t1.order_date) OVER (PARTITION BY t1.user_id) AS first_order_date
  FROM
      user_orders t1;
  ```
  Against the seed data, both of user 101's orders are tagged `2026-01-15`
  and user 102's single order `2026-02-10`.
