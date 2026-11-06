# Project 18: Patient Readmission Analysis

**Domain:** Healthcare · **Difficulty:** 🔴 Advanced

## Problem Description

A hospital wants to spot patients who are cycling back through admissions
too quickly after being discharged — a well-known early-warning signal for
readmission risk. Given a log of admissions and discharges per patient,
compute, for every admission, how many days have passed since that same
patient's previous discharge.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `admissions` | प्रवेशाः | `admission_id` (PK), `patient_id`, `admission_date` (int, day-number), `discharge_date` (int, day-number) |

**Source:** Maven Analytics' [Maven Hospital Challenge](https://mavenanalytics.io/challenges/maven-hospital-challenge)
— a free practice dataset in which the learner plays analytics consultant
for a fictional "Massachusetts General Hospital," working from ~1,000
synthetic patient records (2011–2022) of demographics, insurance, and
encounters. One of the challenge's own stated objectives is computing a
30-day readmission rate from admission/discharge date pairs — exactly the
event-pairing concept this project models. This project keeps only that
admission/discharge-pair shape and discards everything else (demographics,
insurance, cost, the synthetic-hospital framing); it's a hand-built, minimal
4-column, 4-row fixture across 2 patients, with plain integer day-numbers
standing in for real dates, not the full dataset.

## SQL/SSQL Concepts Demonstrated

- Window function `LAG() OVER (PARTITION BY ... ORDER BY ...)` (पश्चात्
  with विभाजनम्/अनुसारम् आरोहक्रमेण)
- Native ablative-case subtraction (`विशोध्य`/`अवशिष्टम्`) applied to a
  window-function result, not just a plain column

## Objectives

1. **Objective 1 — Days since last discharge**: For every admission,
   return the patient id, admission date, discharge date, and the number
   of days since that same patient's previous discharge (admission date
   minus `LAG(discharge_date)`, partitioned by patient, ordered by
   admission date ascending).

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles to SQL identical for SQLite and PostgreSQL
  (`t1.admission_date - LAG(t1.discharge_date) OVER (PARTITION BY
  t1.patient_id ORDER BY t1.admission_date ASC) AS days_since_discharge`).
  Against the seed data: patient 501 → NULL, 10; patient 502 → NULL, 48.
