# Project: Simple Basic SELECT

**Domain:** Education · **Difficulty:** 🟢 Basic

## Problem Description

A minimal smoke-test scenario: given a table of student records (name, age,
school, grade, score), return the name and age of every student older than
18. No joins, no aggregation, no grouping — just a single filtered
projection, meant to exercise the most basic SELECT...WHERE shape end to
end.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `students` | छात्राः | `id` (PK), `name`, `age`, `school`, `grade`, `score` |

**Source:** No external dataset — a minimal smoke-test scenario invented
for this repo. The schema is declared in `schema.सं`, and three sample rows
are in `seed.सं`.

## SQL/SSQL Concepts Demonstrated

- Basic `SELECT` with column projection
- `WHERE` filter with a numeric comparison (`अधिकः अस्ति`, "is greater
  than")
- The simplest FROM/WHERE/SELECT query shape in the whole corpus — a
  baseline smoke test rather than a feature demonstration

## Objectives

1. **Objective 1 — Students over 18**: Return the name and age of every
   student whose age is greater than 18.

## Deliverables

- A single native SSQL query, `basic_select.सं`, which sits directly in the
  project root (there is no `queries/` subdirectory).
- No `.sql` file is committed; the SQL for either dialect is generated from
  `basic_select.सं` by the compiler.
- `schema.सं` and `seed.सं` — the table definition and sample rows.

## Status

- **Objective 1** (`basic_select.सं`): compiles to the same SQL for SQLite
  and PostgreSQL — `SELECT t1.name, t1.age FROM students t1 WHERE t1.age >
  18;` — and returns Arjun (20) and Rohan (22) against the seed data.
