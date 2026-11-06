# Project 06: Pizza Throughput - Beginner's Guide

This project demonstrates grouping, counting, and date/time part extraction in SSQL.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Group orders by the hour of the order time and count the total number of orders in each hour.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
We create the table `आदेशाः` ("orders").
*   **Feminine Concordance**: Since `सङ्ख्या` (order_id) is feminine, the constraints use feminine forms: `क्रमिका` (autoincrement) and `प्राथमिकी` (primary key).
*   **Time Data Type**: We define time as `समयः इति वाक्यम् "time"`, which maps to SQLite/PostgreSQL string/text format.

### 2. Solving Objective 1: Hourly Order Counts
To extract the hour, we use the `होरा` (hour) keyword:
*   **Sanskrit time extraction**: `आदेशसमयस्य होरा` (Hour of order_time) is evaluated as an integer hour: `CAST(strftime('%H', t1.time) AS INTEGER)` in SQLite and `CAST(EXTRACT(HOUR FROM (...)::time) AS INTEGER)` in PostgreSQL.
*   **Grouping**: We group by the extracted hour using `आदेशसमयस्य होरा इति समूहेन`.
*   **Select list**: We select the hour (`आदेशसमयस्य होरा "hour" इति आख्या`) and the order count using `सङ्ख्यायाः गणना "order_count" इति आख्या`.
*   **Verb agreement**: Since we select two columns (`hour` and `order_count`), we use the dual verb `आह्रियेताम्`.
*   **Query** (`queries/q1_throughput.सं`):
    ```sanskrit
    आदेशेभ्यः आदेशसमयस्य होरा इति समूहेन आदेशसमयस्य होरा "hour" इति आख्या च सङ्ख्यायाः गणना "order_count" इति आख्या च आह्रियेताम् ।
    ```

---

## 🔍 Verification

Run the verify script from the repository root to compile and execute the queries:
```bash
uv run python3 verify_all.py
```
It executes the queries against both SQLite and PostgreSQL and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
