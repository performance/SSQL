# Project 27: Movie Ratings & Reviews

**Domain:** Media & Entertainment · **Difficulty:** 🟡 Intermediate

## Problem Description

A media platform wants a quick way to surface well-reviewed movies: join
every movie to its reviews and keep only the reviews that clear a minimum
rating bar, returning the movie's title and genre alongside the qualifying
rating. This is the canonical two-table inner join plus a threshold filter
on the "many" side of a one-to-many relationship.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `movies` | चलच्चित्राणि | `movie_id` (PK), `title`, `genre` |
| `reviews` | समीक्षाः | `review_id` (PK), `movie_id` (FK → movies), `rating` |

**Source:** No external public dataset. The scenario was invented for this example. It's a hand-built, minimal fixture (3
movies, 3 reviews, one review per movie) purpose-built to demonstrate a
multi-table join with a rating-threshold filter. For a curious reader's
context, the well-known [MovieLens](https://grouplens.org/datasets/movielens/)
dataset covers similar territory (movies + ratings), but its structure differs — MovieLens has a `userId`
dimension and rating timestamps that this fixture lacks, and its `genres`
field is a pipe-separated list rather than this project's single scalar
`genre` column — so nothing here is derived from it.

## SQL/SSQL Concepts Demonstrated

- Multi-table `JOIN` (movies ⋈ reviews on `movie_id`)
- `WHERE` filter on a threshold applied to a joined-in column (`rating`)

## Objectives

1. **Objective 1 — Well-reviewed movies**: For every review with a rating
   strictly greater than 4, return the movie's title, genre, and that
   rating.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles for both SQLite and PostgreSQL, identical
  generated SQL across both dialects, matching this directory's `GUIDE.md`:
  ```sql
  SELECT
      t1.title,
      t1.genre,
      t2.rating
  FROM
      movies t1
  JOIN reviews t2 ON t1.movie_id = t2.movie_id
  WHERE
      t2.rating > 4;
  ```
  The query text reads "मूल्याङ्कनम् ४ इत्यस्मात् अधिकम् अस्ति" (strictly
  greater-than, no "or equal"), so it compiles to a strict `> 4`. Against
  this project's seed data (ratings 4.8, 4.9, 2.5) it returns two rows:
  Inception (4.8) and The Dark Knight (4.9).
