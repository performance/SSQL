# Project 15: Social Engagement - Beginner's Guide

This project demonstrates window functions and category ranking in SSQL.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Rank social posts within each category by the number of views descending using window functions.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
*   **Table**: We define table `लेखाः` ("posts").
*   **Columns**:
    *   `सङ्ख्या` (PK, post_id)
    *   `वर्गः` (category)
    *   `शीर्षकम्` (title)
    *   `दृष्टिसङ्ख्या` (the view count, marked as `अनिवार्या` to avoid null-sorting dialect issues)

### 2. Solving Objective 1: Window Partitioning & Ranking
*   **Window Function**: SSQL window rankings are defined using the `श्रेणी यत्र ... श्रेणिक्रमः इति आख्या` (RANK OVER ...) construct.
*   **Partitioning**: Partitioned by category using `वर्गस्य आधारेण विभाजनम्`.
*   **Ordering**: Ordered descending by views using `दृष्टिसङ्ख्यायाः अनुसारम् अवरोहक्रमेण`.
*   **Verb Agreement**: Since four columns are selected (`श्रेणिक्रमः`, `वर्गः`, `शीर्षकम्`, and `दृष्टिसङ्ख्या`), we use the plural verb form `आह्रियन्ताम्`.
*   **Query** (`queries/q1_social.सं`):
    ```sanskrit
    लेखेभ्यः
    श्रेणी यत्र वर्गस्य आधारेण विभाजनम् दृष्टिसङ्ख्यायाः अनुसारम् अवरोहक्रमेण श्रेणिक्रमः इति आख्या च
    लेखस्य वर्गः च
    लेखस्य शीर्षकम् च
    लेखस्य दृष्टिसङ्ख्या च
    आह्रियन्ताम् ।
    ```

---

## 🔍 Verification

Run the verify script from the repository root to compile and execute the queries:
```bash
uv run python3 verify_all.py
```
It executes the queries against both SQLite and PostgreSQL and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
