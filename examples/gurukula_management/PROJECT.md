# Project: Gurukula Management — SQL Feature Verification Suite

**Domain:** Education (traditional gurukula/school) · **Kind:** 🧪 SQL-feature verification corpus (not a single-narrative tutorial)

## What this is

Unlike the numbered tutorial projects (`01_*`–`30_*`), this is **not** one business
scenario driving a handful of analytical questions. It's a broad SQL-feature
practice/regression corpus: 60 independent queries (`q01.सं`–`q60.सं`), each
targeting one specific SQL construct, dressed in a "gurukula" (traditional
school) domain — teachers (आचार्याः), students (छात्राः), subjects (शास्त्राणि),
classes (कक्षाः), exams (परीक्षाः), hostels (वसतिगृहाणि), and donations
(दानानि). The queries fall into six 10-query "parts," which double as a rough SQL-difficulty progression from plain
`SELECT`/`WHERE` up to self-joins, correlated-style subqueries, and set
operations. Given the scale and one-feature-per-file structure, this reads as
part of the repo's own regression-testing material rather than a teaching
narrative for a course.

**Provenance:** the 60 queries were first written out by a generator script.
That script went stale as the language changed and has been removed; the
`.सं` files are the source, and their one-line titles live in
`website/build_site.py`. There is no external dataset behind this project.

## Schema

| Table | Sanskrit name | Notes |
|---|---|---|
| Teachers | आचार्याः (`teachers`) | name, designation, phone, salary |
| Students | छात्राः (`students`) | name, DOB, admission date, fees |
| Subjects | शास्त्राणि (`subjects`) | name, difficulty level |
| Classes | कक्षाः (`classes`) | teacher_id FK, subject_id FK, class_time |
| Exams | परीक्षाः (`exams`) | student_id FK, subject_id FK, exam_date, marks |
| Donations | दानानि (`donations`) | donor name, amount, donation date |
| Hostels | वसतिगृहाणि (`hostels`) | student_id FK, room_number, hostel_fee |

Every table in `schema.सं` is a domain entity. The CTEs used by the
self-join, subquery and set-operation queries (q51–q60) are declared in the
queries that build them, never as schema tables.

## Query categories (60 queries)

- **Part 1 — Basic filters & projections** (`q01`–`q10`, 10 queries): plain
  `SELECT`/projection, numeric and date comparisons, `ORDER BY` (asc/desc),
  `LIMIT`/`OFFSET`, `AND`/`OR` combinations, `BETWEEN`-style range filters,
  `LIKE` pattern matching. E.g. `q01.सं` (list all teachers), `q06.सं` (top 3
  highest-paid teachers via `LIMIT`), `q10.सं` (teachers whose name starts
  with 'V' via `LIKE`).
- **Part 2 — JOINs** (`q11`–`q20`, 10 queries): multi-table inner joins
  (classes ⋈ teachers ⋈ subjects), `LEFT JOIN` (students ⋈ hostels, some
  students have no room), joins combined with `WHERE` filters. E.g.
  `q11.सं` (class schedule with teacher/subject names), `q16.सं` (all
  students with room numbers via `LEFT JOIN`).
- **Part 3 — Aggregates & grouping** (`q21`–`q30`, 10 queries): `COUNT`,
  `SUM`, `AVG`, `MAX`/`MIN`, `GROUP BY`, `HAVING`. E.g. `q22.सं` (total salary
  paid), `q28.सं` (hostels grouped by room, average fee `HAVING` > 1600).
- **Part 4 — CASE expressions** (`q31`–`q40`, 10 queries): simple and
  multi-branch searched `CASE` across every table in the domain (salary
  bands, fee tiers, grade letters, designation levels, seniority). E.g.
  `q31.सं` (teacher salary → High/Medium/Low), `q34.सं` (exam marks → letter
  grade).
- **Part 5 — Window functions** (`q41`–`q50`, 10 queries): `RANK() OVER`,
  `LEAD`/`LAG`, running `SUM() OVER (ORDER BY ...)`, `PARTITION BY`. E.g.
  `q41.सं` (rank students by fees), `q48.सं` (rank exam marks within each
  subject via `PARTITION BY`).
- **Part 6 — Self-joins, subqueries & set operations** (`q51`–`q60`, 10
  queries): self-joins (pairs of students/teachers/subjects matching on a
  value), `UNION`, `EXCEPT`, `INTERSECT`, and scalar subqueries (rows above
  the table's own average). E.g. `q52.सं` (`UNION` of teacher and student
  names), `q55.सं` (students paying more than the average student fee, via a
  join `ON 1 = 1` to a one-row CTE that holds the average).

## Deliverables

- Native schema/seed: `SAMSQL/schema.सं`, `SAMSQL/seed.सं`.
- 60 native query files: `SAMSQL/q01.सं`…`q60.सं`. Their SQL is compiled
  output, produced by `verify_all.py` and not tracked.
- No `GUIDE.md` or solution write-ups —
  unlike the numbered tutorial projects, this project has no walkthrough
  documentation, matching its role as a feature-coverage corpus rather than a
  taught lesson.

## Status

The repo-wide `verify_all.py` harness discovers this project as
`gurukula_management/SAMSQL` and compiles and runs its schema, seed, and
each of the 60 queries. Per-query results are written to
`verification_progress.json`, which is produced locally by `verify_all.py`
and is not tracked in the repo.
