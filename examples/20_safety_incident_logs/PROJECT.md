# Project 20: Safety Incident Logs

**Domain:** Industrial Safety · **Difficulty:** 🟡 Intermediate

## Problem Description

A factory logs every safety incident as a free-text description that
embeds a standardized error code (e.g. `"ERR102"`) alongside a severity
rating. Safety engineers want that error code pulled out into its own
column so incidents can be filtered and grouped by code without
re-parsing the description text every time.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `incidents` | दुर्घटनाः | `incident_id` (PK), `description`, `severity` |

**Source:** Kaggle's [Industrial Safety and Health Analytics Database](https://www.kaggle.com/datasets/ihmstefanini/industrial-safety-and-health-analytics-database)
(ihmstefanini) — a real accident log from 12 industrial plants across 3
countries, with a free-text description field and an Accident
Level/Potential Accident Level severity grading that this project's
`description`/`severity` columns echo in shape. Note that the real
dataset's descriptions are risk narratives (e.g. "while removing the
drill..."), not text containing embedded `"ERRxxx"`-style codes — the
error-code convention is this repo's own invented flavor detail, layered
on top of the borrowed description+severity shape specifically to teach
string-position/substring extraction, and is not present in the actual
dataset. This project is a hand-built, minimal 3-column, 3-row fixture;
nothing here is pulled from the real dataset's rows.

## SQL/SSQL Concepts Demonstrated

- Dialect-aware built-in string-position function `स्थानम्` — compiles to
  SQLite's `instr(...)` vs PostgreSQL's `strpos(...)`, no custom function
  declaration needed (PostgreSQL has no `instr()`, and its `position`
  keyword only accepts the special `POSITION(substring IN string)` syntax
  rather than an ordinary function call, so `strpos()` — same call syntax
  and argument order as `instr()` — is emitted instead)
- Dialect-aware built-in substring function `खण्डः` (`SUBSTR`, identical
  spelling in both dialects)
- Nested function calls inside a computed, aliased `SELECT`-list column

## Objectives

1. **Objective 1 — Error code extraction**: For every incident, return
   its id, description, severity, and a 6-character `error_code` column:
   the substring of the description starting where `"ERR"` first appears.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles
  to the same SQL within each dialect —
  `substr(description, nullif(instr(description, 'ERR'), 0), 6)` on SQLite (NULL where `ERR` is absent), with
  `strpos(...)` in place of `instr(...)` on PostgreSQL. Against the seed
  data it extracts `ERR102`, `ERR404`, and `ERR901`.
