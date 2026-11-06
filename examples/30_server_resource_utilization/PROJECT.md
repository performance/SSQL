# Project 30: Server Resource Utilization

**Domain:** DevOps / Cloud Infrastructure · **Difficulty:** 🔴 Advanced

## Problem Description

An operations team polls CPU utilization on their servers at regular
intervals, but a single spot reading is noisy — a momentary spike or dip
doesn't necessarily mean anything. They want each reading smoothed against
its own server's recent history: a moving average over the current reading
and the two immediately before it, computed independently per server, so
sustained trends stand out from one-off blips.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `server_logs` | सेवकवृत्तान्ताः | `log_id` (PK), `server_id`, `log_time` (int, plain increasing sequence — not a real timestamp), `cpu_usage` (real) |

**Source:** No external public dataset. The scenario was invented for this example. It's a hand-built, minimal single-table,
4-row fixture (three readings for server 501, one for server 502) using a
plain integer `log_time` sequence (100, 101, 102...) rather than a real
timestamp — a strong tell that this is a pedagogical vehicle for
window-frame syntax, not a scaled-down real observability dataset.
Purpose-built to demonstrate a bounded window frame (`ROWS BETWEEN 2
PRECEDING AND CURRENT ROW`) computing a moving average. For context, real
server/observability monitoring data (Prometheus/Datadog exports, cloud
provider CloudWatch-style metric dumps) carries true timestamps, many more
metrics (memory, disk I/O, network), and orders of magnitude more rows;
nothing here is derived from any of them.

## SQL/SSQL Concepts Demonstrated

- Window functions with an explicit frame:
  `AVG(...) OVER (PARTITION BY ... ORDER BY ... ASC ROWS BETWEEN 2
  PRECEDING AND CURRENT ROW)` (SSQL: माध्यम् ... श्रेणी ... पङ्क्तयः २
  पूर्ववर्तितः आरभ्य वर्तमानपङ्क्तिपर्यन्तम्)
- Partitioned, ordered moving-average computation — a rolling window scoped
  to each server's own timeline

## Objectives

1. **Objective 1 — Moving CPU average**: For every server log entry,
   return its server id, log time, CPU usage, and a 3-row (current row plus
   the 2 preceding) moving average of CPU usage within that server's own
   timeline.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles
  for both SQLite and PostgreSQL, identical generated SQL across both
  dialects, matching this directory's `GUIDE.md` exactly:
  ```sql
  SELECT
      t1.server_id,
      t1.log_time,
      t1.cpu_usage,
      AVG(t1.cpu_usage) OVER (PARTITION BY t1.server_id ORDER BY t1.log_time ASC NULLS LAST, t1.log_id ASC NULLS LAST ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) AS moving_cpu_avg
  FROM
      server_logs t1;
  ```
  Executed against the seed data (server 501:
  45/60/75 at times 100/101/102; server 502: 30 at time 100), the moving averages are 45.0, 52.5, 60.0 for server 501 and 30.0 for
  server 502.
