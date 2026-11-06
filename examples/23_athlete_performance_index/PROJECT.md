# Project 23: Athlete Performance Index

**Domain:** Sports Analytics · **Difficulty:** 🔴 Advanced

## Problem Description

A sports federation logs individual event scores for athletes who compete
across multiple events within their sport. An analyst wants a per-athlete,
per-sport performance summary — the average score and the total score
across all recorded events — ranked so the highest overall total appears
first.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `athlete_scores` | क्रीडाङ्काः | `score_id` (PK), `athlete_name`, `sport`, `score` (real) |

**Source:** No external public dataset. The scenario was invented for this example. It's a
hand-built, minimal 4-column, 4-row fixture using two real Indian athlete
names ("Virat," "Neeraj") and their real sports (cricket, javelin) as
flavor text, purpose-built to demonstrate one SQL feature: multi-column
`GROUP BY` combined with multiple aggregates and an `ORDER BY` on a
computed aggregate. Nothing here is derived from any real competition
results or scoring dataset.

## SQL/SSQL Concepts Demonstrated

- Multi-column `GROUP BY` (grouping by both `athlete_name` and `sport`)
- Multiple aggregates in one `SELECT` (`AVG`, `SUM`)
- `ORDER BY` on a computed aggregate expression, descending

## Objectives

1. **Objective 1 — Performance index per athlete/sport**: For every
   athlete/sport combination, return the athlete name, sport, average
   score, and total score, ordered by total score descending.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles
  for both SQLite and PostgreSQL. Generated SQL **differs between
  dialects**:
  - **SQLite:**
    ```sql
    SELECT
        t1.athlete_name,
        t1.sport,
        AVG(t1.score) AS avg_score,
        SUM(t1.score) AS total_score
    FROM
        athlete_scores t1
    GROUP BY t1.athlete_name, t1.sport
    ORDER BY SUM(t1.score) DESC NULLS LAST;
    ```
  - **PostgreSQL:** identical shape, but the compiler wraps every
    aggregate — including the repeated one in `ORDER BY` — in an explicit
    `CAST(... AS DOUBLE PRECISION)`:
    ```sql
    SELECT
        t1.athlete_name,
        t1.sport,
        CAST(AVG(t1.score) AS DOUBLE PRECISION) AS avg_score,
        CAST(SUM(t1.score) AS DOUBLE PRECISION) AS total_score
    FROM
        athlete_scores t1
    GROUP BY t1.athlete_name, t1.sport
    ORDER BY CAST(SUM(t1.score) AS DOUBLE PRECISION) DESC NULLS LAST;
    ```
  - In both dialects, `ORDER BY` re-expresses the full `SUM(t1.score)`
    aggregate rather than referencing the `total_score` alias — functionally
    equivalent.
  - Against the seed data: Virat / Cricket (average 97.5, total 195), then
    Neeraj / Javelin (average 90.35, total 180.7).
