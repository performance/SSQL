# Project 24: Hotel Booking Cancellations

**Domain:** Hospitality · **Difficulty:** 🟡 Intermediate

## Problem Description

A hotel wants to flag bookings that carry a long "lead time" — the gap
between the day a guest books and the day they're due to arrive. Long lead
times are a well-known proxy for cancellation risk in the hospitality
industry (more time between booking and stay means more opportunity for
plans to change), so the hotel wants a simple report of every booking whose
lead time exceeds 30 days.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `hotel_bookings` | आवासग्रहणानि | `booking_id` (PK), `guest_name`, `booking_day` (int, day-of-year style), `arrival_day` (int, day-of-year style) |

**Source:** No external public dataset. The scenario was invented for this example. It's a
hand-built, minimal 4-column, 3-row fixture with plain integer day-numbers
standing in for real dates, purpose-built to demonstrate date-difference
arithmetic and filtering. For context, the "lead time" concept (days
between booking and arrival, used as a cancellation-risk signal) is also
the central idea of the well-known Kaggle
[Hotel Booking Demand](https://www.kaggle.com/datasets/jessemostipak/hotel-booking-demand)
dataset (Antonio, Almeida & Nunes, *Data in Brief*, 2019, ~119k rows, ~32
columns including a real `is_canceled` flag) — worth pointing a curious
reader toward, but nothing here is derived from it, and there is no
cancellation-status column here despite the project name; "cancellation
risk," not actual cancellation records, is the modeled concept.

## SQL/SSQL Concepts Demonstrated

- Scalar arithmetic in a `SELECT` list (subtraction between two integer columns)
- Filtering (`WHERE`) on a computed expression, not a raw column
- SSQL's ablative-case ("from X, subtract Y") phrasing for subtraction

## Objectives

1. **Objective 1 — High-lead-time bookings**: For every booking with lead
   time (`arrival_day - booking_day`) greater than 30, return the guest
   name and the computed lead time.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles to SQL identical for SQLite and PostgreSQL: `SELECT
  t1.booking_id, t1.guest_name, t1.arrival_day - t1.booking_day AS
  lead_days FROM hotel_bookings t1 WHERE t1.arrival_day - t1.booking_day >
  30;`. Against the seed data it returns two bookings (Ramesh and Mahesh),
  each with a lead time of 40 days.
