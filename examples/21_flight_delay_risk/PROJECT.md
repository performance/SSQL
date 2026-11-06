# Project 21: Flight Delay Risk

**Domain:** Aviation · **Difficulty:** 🔴 Advanced

## Problem Description

An airline analyst wants to compare how badly individual flights are
delayed relative to other flights operated by the *same* carrier — not
against the whole fleet, since carriers differ in scale and route mix.
Given a log of flights with their delay in minutes, rank each flight's
delay against only its own carrier's other flights, worst delay first.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `flights` | विमानानि | `flight_id` (PK), `carrier`, `flight_num`, `delay_minutes` |

**Source:** No external public dataset. The scenario was invented for this example. It's a
hand-built, minimal 4-row fixture using two real Indian carrier names
("Indigo," "AirIndia") as flavor text, purpose-built to demonstrate one
SQL feature: partitioned window ranking. For context, real flight-delay
datasets in this general domain — e.g. the Maven Analytics Data
Playground's airline delay practice set, or Kaggle's
[2015 Flight Delays and Cancellations](https://www.kaggle.com/datasets/usdot/flight-delays)
(sourced from the US DOT/BTS) — carry airport codes, real dates, and
millions of rows; nothing here is derived from them.

## SQL/SSQL Concepts Demonstrated

- Window functions: `DENSE_RANK() OVER (PARTITION BY ... ORDER BY ... DESC)`
  (सघनश्रेणी), as the original design specified. (`श्रेणी` is plain `RANK()`.)

## Objectives

1. **Objective 1 — Delay rank per carrier**: For every flight, return its
   carrier, flight number, delay in minutes, and its `DENSE_RANK()` among that
   carrier's flights ordered by delay descending.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles to SQL identical for SQLite and PostgreSQL (`DENSE_RANK() OVER
  (PARTITION BY t1.carrier ORDER BY t1.delay_minutes DESC NULLS LAST) AS
  delay_rank`). The seed has no tied delays, so DENSE_RANK and RANK agree on it.
  Against the seed data: AirIndia 202 (60 min) → rank 1, 201 (10 min) →
  rank 2; Indigo 102 (45 min) → rank 1, 101 (15 min) → rank 2.
