# Project 22: Severe Weather Tracking

**Domain:** Meteorology · **Difficulty:** 🟡 Intermediate

## Problem Description

A weather service logs temperature and rainfall readings from several
monitoring stations. Forecasters want a quick way to flag "severe weather"
events — readings where the temperature and the rainfall both cross
dangerous thresholds at the same time — so they can pull just those log
entries out of the full history for closer review.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `weather_logs` | ऋतुवृत्तान्ताः | `log_id` (PK), `station_name`, `record_date`, `temperature` (real), `rainfall` (real) |

**Source:** No external public dataset. The scenario was invented for this example. It's a
hand-built, minimal 5-column, 4-row fixture (four station names —
"Central," "North," "South," "West" — as generic flavor text, one day's
readings) purpose-built to demonstrate one SQL feature: range filtering
with a compound `AND` condition across two independent numeric columns.
Nothing here is derived from any real meteorological station or dataset.

## SQL/SSQL Concepts Demonstrated

- `WHERE` clause with a compound `AND` condition across two independent
  numeric columns (temperature threshold, rainfall threshold)
- Comparison operators expressed via SSQL's ablative-comparative phrasing
  (`X इत्यस्मात् अधिकम् अस्ति` — "X is greater than Y")

## Objectives

1. **Objective 1 — Severe weather log filter**: For every weather log entry
   where temperature exceeds 40 and rainfall exceeds 50, return the station
   name, record date, temperature, and rainfall.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles to the same SQL for SQLite and PostgreSQL:
  ```sql
  SELECT
      t1.station_name,
      t1.record_date,
      t1.temperature,
      t1.rainfall
  FROM
      weather_logs t1
  WHERE
      (t1.temperature > 40 AND t1.rainfall > 50);
  ```
  Against the seed data it returns two rows (Central Station and West
  Station).
