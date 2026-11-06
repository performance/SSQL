# Project 16: Influencer Moving Average

**Domain:** Marketing · **Difficulty:** 🔴 Advanced

## Problem Description

An influencer's daily post performance (likes) is noisy day to day, which
makes it hard to spot the underlying engagement trend from the raw numbers
alone. A marketing analyst wants a smoothed view: for every post, a short
moving average that blends that post's likes with the two posts before it,
so short-term spikes and dips don't obscure the trend.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `influencers` | प्रभावकाः | `influencer_id` (PK), `post_date`, `likes` |

**Source:** Inspired by influencer-marketing datasets on Kaggle; the data
here is a small hand-built fixture.
Kaggle datasets in this general space include e.g.
[Instagram Analytics Dataset](https://www.kaggle.com/datasets/kundanbedmutha/instagram-analytics-dataset)
and
[Influencer Marketing ROI Dataset](https://www.kaggle.com/datasets/tfisthis/influencer-marketing-roi-dataset),
but this is not an extract of any of them — it is a hand-built, minimal 3-column, 5-row
fixture (`influencer_id`, `post_date`, `likes`) purpose-built to demonstrate
a window-frame moving average.

## SQL/SSQL Concepts Demonstrated

- Window frame: `AVG(likes) OVER (ORDER BY post_date ASC ROWS BETWEEN 2
  PRECEDING AND CURRENT ROW)` — SSQL's `पङ्क्तयः N पूर्ववर्तितः आरभ्य
  वर्तमानपङ्क्तिपर्यन्तम्` ("starting N rows preceding, up to the current
  row") frame-boundary phrasing

## Objectives

1. **Objective 1 — 3-post moving average**: For each influencer post
   (ordered by date), return the post date, like count, and the moving
   average of likes over the current row and its 2 preceding rows.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough (`GUIDE.md`), which also documents the exact
  expected generated SQL.

## Status

- **Objective 1** (`q1_elegant.सं`): compiles for SQLite and PostgreSQL; the only difference is that
  PostgreSQL adds `COLLATE "C"` to `post_date` in the window's `ORDER BY`. Against the 5-row
  seed (2026-07-01 through 2026-07-05) the moving averages are 150.0,
  175.0, 200.0, 250.0, and 243.33.
