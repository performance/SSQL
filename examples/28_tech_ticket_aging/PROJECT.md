# Project 28: Tech Support Ticket Aging

**Domain:** Cloud & Tech Operations · **Difficulty:** 🔴 Advanced

## Problem Description

A support team wants to see how long open tickets have been sitting, in
days, so they can triage the oldest ones first. Given a ticket log where
each row carries the day it was opened and today's day number, compute
each open ticket's age and return it alongside the ticket's issue type —
filtered down to tickets that are still open.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `tickets` | पत्राणि | `ticket_id` (PK), `issue_type`, `open_day` (int, day-number), `current_day` (int, day-number), `status` |

**Source:** No external public dataset. The scenario was invented for this example. It's a hand-built, minimal 3-row fixture
using plain integer "day number" columns (`open_day`, `current_day`)
rather than real dates or timestamps — a strong tell that this is a
pedagogical vehicle for scalar date-subtraction syntax, not a stand-in for
any real ticketing system's data.

## SQL/SSQL Concepts Demonstrated

- Scalar arithmetic in a `SELECT` list (subtraction between two integer
  columns)
- Filtering (`WHERE`) on a status flag alongside a computed expression in
  the projection
- SSQL's ablative-case ("from X, subtract Y") phrasing for subtraction

## Objectives

1. **Objective 1 — Open ticket age**: For every ticket whose status is
   "Open", return the ticket id, issue type, and its age in days
   (`current_day - open_day`).

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles for both SQLite and PostgreSQL, identical generated SQL across both
  dialects:
  ```sql
  SELECT
      t1.ticket_id,
      t1.issue_type,
      t1.current_day - t1.open_day AS open_days
  FROM
      tickets t1
  WHERE
      t1.status = 'Open';
  ```
  Against the seed data it returns two open tickets: Network Drop (15
  days) and Disk Full (24 days).
