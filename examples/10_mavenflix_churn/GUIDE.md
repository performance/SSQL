# Project 10: MavenFlix Churn - Beginner's Guide

This project demonstrates views and simulating self-joins in SSQL.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Identify customers who downgraded their subscription tier from "Premium" to "Basic" in a subsequent month.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema & View Definition (`schema.सं`)
*   **Table**: We define `सदस्यत्वम्` ("subscriptions").
*   **View**: SSQL has no table aliases, so a self-join gives the table's second role its own name — with a CTE, or, as here, with an SSQL view `पूर्वसदस्यत्वम्` ("prev_subs") which copies the subscriptions table:
    `पूर्वसदस्यत्वम् इति दृश्यम् "prev_subs" इति आख्या पूर्वसदस्यत्व इति नपुंसकलिङ्गम् एकवचनम् रच्यताम् यत् सदस्यत्वेभ्यः ग्राहकः च माससङ्ख्या च उत्पादः च आह्रियन्ताम् इति दृश्यम् ।`

### 2. Solving Objective 1: Downgrade Joins
To identify downgraded customers:
*   **Self Join**: We query `सदस्यत्वेभ्यः` and join the view `पूर्वसदस्यत्वेन` using the customer ID field: `पूर्वसदस्यत्वेन सह सदस्यत्वस्य ग्राहकः पूर्वसदस्यत्वस्य ग्राहकः इत्यनेन समः अस्ति इति मेलनेन`.
*   **Grammar / Spelling**: Since `सदस्यत्वम्` is neuter, its columns are qualified with the genitive singulars `सदस्यत्वस्य` and `पूर्वसदस्यत्वस्य`.
*   **Conditions**: We check the following conditions joined by `च` (and), closed by `इति नियमेन` (WHERE):
    *   Current tier is "Basic": `सदस्यत्वस्य उत्पादः "Basic" इत्यनेन समः अस्ति`.
    *   Previous tier was "Premium": `पूर्वसदस्यत्वस्य उत्पादः "Premium" इत्यनेन समः अस्ति`.
    *   Current month is later (using feminine `अधिका` to match `माससङ्ख्या`): `सदस्यत्वस्य माससङ्ख्या पूर्वसदस्यत्वस्य माससङ्ख्या इत्यस्याः अधिका अस्ति`.
*   **Verb Agreement**: We select exactly one column (`customer_id`), so we use the singular verb `आह्रियताम्`.
*   **Query** (`queries/q1_churn_downgrades.सं`):
    ```sanskrit
    सदस्यत्वेभ्यः
    पूर्वसदस्यत्वेन सह सदस्यत्वस्य ग्राहकः पूर्वसदस्यत्वस्य ग्राहकः इत्यनेन समः अस्ति इति मेलनेन
    सदस्यत्वस्य उत्पादः "Basic" इत्यनेन समः अस्ति च पूर्वसदस्यत्वस्य उत्पादः "Premium" इत्यनेन समः अस्ति च सदस्यत्वस्य माससङ्ख्या पूर्वसदस्यत्वस्य माससङ्ख्या इत्यस्याः अधिका अस्ति च इति नियमेन
    पुनरुक्तिम् विना सदस्यत्वस्य ग्राहकः
    आह्रियताम् ।
    ```

---

## 🔍 Verification

Run the verify script from the repository root to compile and execute the queries:
```bash
uv run python3 verify_all.py
```
It executes the queries against both SQLite and PostgreSQL and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
