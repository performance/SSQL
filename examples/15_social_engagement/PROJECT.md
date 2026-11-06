# Project 15: Social Engagement

**Domain:** Social Media · **Difficulty:** 🟡 Intermediate

## Problem Description

A small blog network publishes posts across a handful of topic categories
(Tech, Life, etc.) and tracks how many views each post receives. Editors
want to know, within each category, how each post's popularity stacks up
against its category-mates — not against the network as a whole, since
categories differ widely in typical readership — so they can spot each
category's top performer at a glance.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `posts` | लेखाः | `post_id` (PK), `category`, `title`, `views` (NOT NULL) |

**Source:** Inspired by social-media engagement/posts datasets on Kaggle.
Several
similarly-scoped public Kaggle datasets exist under this general theme,
e.g. [Social Media Engagement Dataset](https://www.kaggle.com/datasets/aviral342/social-media-engagement-dataset)
(post-level records with post ID, content category, and view/engagement
counts among ~20 fields) and
[Social Media Post Engagement Dataset](https://www.kaggle.com/datasets/cagrickr/social-media-post-engagement-dataset),
but the data here is not taken from any of them — this repo's schema (`post_id`, `category`, `title`,
`views`) is a minimal, hand-built 4-row fixture consistent with the general
shape of these datasets, not an extract of any one source.

## SQL/SSQL Concepts Demonstrated

- Window function: `RANK() OVER (PARTITION BY category ORDER BY views
  DESC)` — SSQL's `श्रेणी` (rank) construct with `विभाजनम्` (partition) and
  `अवरोहक्रमेण` (descending order)
- `NOT NULL` column marking (`अनिवार्याः`) on `views`, used specifically to
  avoid cross-dialect ambiguity in how SQLite vs. PostgreSQL sort NULLs
  within an `ORDER BY`
- Aliased computed window column inside a four-column projection (plural
  verb agreement, `आह्रियन्ताम्`)

## Objectives

1. **Objective 1 — Rank posts within category**: For every post, return
   its category, title, view count, and its rank among posts in the same
   category ordered by views descending.

## Deliverables

- Native SSQL query (`queries/q1_social.सं`).
- A step-by-step walkthrough (`GUIDE.md`).

## Status

- **Objective 1** (`q1_social.सं`): compiles to the same SQL for SQLite and
  PostgreSQL. Against the seed data it ranks posts within each category:
  Tech — "Rust for Dummies" (300 views) → rank 1, "Sanskrit SQL Intro"
  (150) → rank 2; Life — "Morning Meditation" (80) → rank 1, "Yoga
  Benefits" (50) → rank 2.
