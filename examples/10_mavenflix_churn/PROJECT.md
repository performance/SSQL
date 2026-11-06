# Project 10: MavenFlix Churn

**Domain:** SaaS · **Difficulty:** 🔴 Advanced

## Problem Description

MavenFlix is a fictional subscription streaming service. Its retention team
wants to catch subscribers who are sliding toward cancellation by first
downgrading their plan — moving from a Premium tier to a Basic tier — before
they churn outright. Given a month-by-month log of each customer's
subscription tier, flag every customer who was on Premium in one month and
had dropped to Basic by a later month.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `subscriptions` | सदस्यत्वम् | `sub_id` (PK), `customer_id`, `month`, `product` |
| `prev_subs` (view) | पूर्वसदस्यत्वम् | `customer_id`, `month`, `product` — a same-shape copy of `subscriptions`, used to simulate a self-join |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground) — "MavenFlix," a fictional streaming-service dataset used across Maven Analytics' subscription/churn practice materials. This project uses a small hand-picked subset: a single-table, 6-row fixture (3 customers × 2 months) standing in for a real subscriber log, not the full dataset. A view stands in for the self-join: SSQL has no ad-hoc table aliasing in `JOIN`, so a view standing in for "the same table, one step back" is how a self-join is simulated.

## SQL/SSQL Concepts Demonstrated

- `CREATE VIEW` (`दृश्यम्`) as a workaround for self-joins, since SSQL has no ad-hoc table-aliasing syntax in `JOIN`
- `JOIN` between a base table and a view of itself, matched on a shared key (`customer_id`)
- A compound `WHERE`-style condition spanning both joined relations (current tier, previous tier, and a month-over-month "later than" comparison) to detect a downgrade
- Ablative-case (पञ्चमी विभक्ति) `FROM`-source phrasing, and the number agreement (एकवचन/बहुवचन) it's expected to carry

## Objectives

1. **Objective 1 — Downgrade identification**: For every customer, find any month where their subscription tier was "Basic" while an earlier month for the same customer was "Premium," and return the customer id.

## Deliverables

- Native SSQL query (`queries/q1_churn_downgrades.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_churn_downgrades.सं`): compiles to the same SQL for SQLite and PostgreSQL: `SELECT DISTINCT t1.customer_id FROM subscriptions t1 JOIN prev_subs t2 ON t1.customer_id = t2.customer_id WHERE ((t1.product = 'Basic' AND t2.product = 'Premium') AND t1.month > t2.month);` Against the seed data it returns customer 101.
