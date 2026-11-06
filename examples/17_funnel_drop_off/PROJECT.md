# Project 17: Funnel Drop-off

**Domain:** E-Commerce · **Difficulty:** 🔴 Advanced

## Problem Description

An e-commerce site instruments every click a visitor makes as they move
through the site (home page, catalog, cart, checkout) within a single
browsing session. A product analyst investigating funnel drop-off wants to
know how much time elapses between consecutive clicks within the same
session, since a long gap between two steps in the funnel can be a signal
of friction or hesitation.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `clicks` | सम्पर्काः | `click_id` (PK), `session_id`, `click_time`, `page_name` |

**Source:** Inspired by e-commerce funnel/clickstream datasets on Kaggle;
the data here is a small hand-built fixture. Kaggle datasets in this general space include e.g.
[e-Shop Clickstream Dataset](https://www.kaggle.com/datasets/arashnic/e-shop-clickstream-dataset)
and
[E-commerce Clickstream and Transaction Dataset](https://www.kaggle.com/datasets/waqi786/e-commerce-clickstream-and-transaction-dataset),
but this is not an extract of any of them — it is a hand-built, minimal 4-column, 5-row
fixture (2 sessions: home→catalog→cart, and home→checkout) purpose-built
to demonstrate `LAG` combined with native subtraction.

## SQL/SSQL Concepts Demonstrated

- Window function `LAG(click_time) OVER (PARTITION BY session_id ORDER BY
  click_time ASC)` — SSQL's `पश्चात्` (LAG) combined with `विभाजनम्`
  (partition) and `अनुसारम्` (order by)
- Native ablative-case subtraction (`विशोध्य`/`अवशिष्टम्`)
  applied directly to a window-function result, rather than plain
  column-minus-column subtraction

## Objectives

1. **Objective 1 — Time between clicks**: For every click, return its
   session, page, timestamp, and the number of seconds elapsed since the
   previous click in the same session (NULL for each session's first
   click).

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough (`GUIDE.md`), which also documents the exact
  expected generated SQL.

## Status

- **Objective 1** (`q1_elegant.सं`): compiles to SQL identical for SQLite and PostgreSQL. Against the 5-row
  seed: session `S101` → NULL, 45, 75 seconds; session `S102` → NULL, 80
  seconds.
