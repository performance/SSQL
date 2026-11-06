# Project: University Book Store — SSQL Verification Corpus

**Domain:** Retail/bookstore · **Kind:** 🧪 SQL-feature verification corpus with its own Rust harness (not a single-narrative tutorial)

## What this is

This is the most elaborate non-numbered project in `examples/`: it has its
own `Cargo.toml`/`Cargo.lock`/`src/main.rs`, its own `design.md`, and its own
`SQL_PG`/`SQLite`/`results` directories, none of which any
numbered tutorial project has. Per [`design.md`](design.md), the project began life as a **plain Rust
application** — "Setup Rust Project: Initialize `university_bookstore` with
`sqlx` and `tokio`," build 30 SQL scenarios, verify them against SQLite —
and only *afterward* was it "ported to SSQL" (`design.md` §4, step 6). `src/main.rs` is the surviving piece of
that original Rust runner: a generic SQL-file executor (`AnyPool` via
`sqlx`) that loads whatever `.sql` files sit in `generated_sql/{sqlite,postgres}/`
and dumps results into `results/`. So this project is best understood as a
test harness for the SSQL compiler itself (dogfooding a real,
if simplified, business schema), not a teaching example — it has grown well
past its original 30-scenario design into 68 query files plus schema/seed.

## Schema

`design.md`'s ER diagram sketches 10 tables including a `book_authors`
many-to-many bridge; the schema actually implemented in `SAMSQL/schema.सं`
simplified that to a direct FK (one author per book) and added a sales-rep
hierarchy not in the original draft at all:

| Table | Sanskrit name | Notes |
|---|---|---|
| Books | पुस्तकानि (`books`) | isbn, title, price, language, publication_year, genre, author_id FK, publisher_id FK |
| Authors | लेखकाः (`authors`) | name |
| Publishers | प्रकाशकाः (`publishers`) | name |
| Customers | ग्राहकाः (`customers`) | name, email, membership level |
| Sales | विक्रयणानि (`sales`) | customer_id FK, store_id FK, sale_date, total |
| Sale items | विक्रयणवस्तूनि (`sale_items`) | sale_id FK, book_id FK, quantity, price_at_sale |
| Stores | आपणाः (`stores`) | name, city |
| Inventory | वस्तुसूच्यः (`inventory`) | store_id FK, book_id FK, quantity |
| Reviews | समीक्षाः (`reviews`) | book_id FK, customer_id FK, rating, comment |
| Sales reps | विक्रयाधिकारिणः (`sales_reps`) | rep/manager hierarchy for queries 31–35; query 33 names the manager role `प्रबन्धकाः` with a CTE |

Every table in `schema.सं` is a domain entity. Intermediate results (CTEs,
views) are declared in the query that builds them, never as schema tables.

## Query categories (68 query files, beyond schema/seed)

`design.md` §3 defines its own 6-category, 30-scenario taxonomy for
`01`–`30`; the project then grew organically to `50` and beyond with files
`design.md` never anticipated:

- **Basic operations** (`01`–`05`, plus `37`/`37b` `IN` and `37c` `NOT IN`,
  `39` `DISTINCT`, `40` Devanagari string literals, `43` `BETWEEN`, `48`
  `DISTINCT` aggregate, `21` keyword search): filtering, projection, a single
  `UPDATE`/`INSERT`.
- **Filters & first joins** (`06`–`10`): books under a price, customers at one email domain,
  sales on a date, one customer's purchase history, sales per store.
- **Aggregates & grouping** (`11`–`15`): low inventory, per-store revenue,
  per-language average price, top-5 spenders, monthly sales counts and revenue.
- **Window functions & analytics** (`16`–`20`, plus `17_running_total_sales`,
  `41_cte`, `18` `PARTITION BY` for most-expensive-per-language): running
  totals, price-vs-publisher-average, churned customers.
- **Search, changes & subqueries** (`21`–`25`): keyword search, a price update,
  above-average ratings, a new book, high-value inventory; never-sold books are
  `16`.
- **Views & indexes** (`26`–`30`, plus `49_create_view`/`50_drop_table`):
  `top_selling_books`/`customer_activity` views, index creation,
  `EXPLAIN QUERY PLAN`.
- **Sales-rep hierarchy extension** (`31`–`35`, not in `design.md` at all):
  rep revenue, commission calculation, manager's-team lookup, zero-sales
  reps, top performer per store — a self-referencing-hierarchy feature
  probe added after the original design.
- **One-feature-per-file micro-tests** (`36`–`47`): `CASE`, `UNION`/`UNION ALL`
  (`42`/`42a`), `INTERSECT`, `EXCEPT`, date/timestamp extraction functions.
- **Ad hoc grammar/regression probes** (unnumbered `test_*.सं` files,
  `books_with_high_rating_low_reviews.सं`, `new_math_rules.सं`): `HAVING`,
  `OFFSET`, string concatenation, scalar functions, field access, "natural"
  functions, `CHET` conditionals, and three separate window-function
  variants — these read as compiler edge-case regression tests rather than
  anything a bookstore stakeholder would ask.

## Deliverables

- `design.md` — the original scoping document (Rust/sqlx-first, SSQL-ported
  later).
- `SAMSQL/` — 70 `.सं` files (`schema.सं`, `seed.सं`, and 68 queries),
  whose `.sql` is compiled output (not tracked); `SQL_PG`/`SQLite` (22 reference
  SQL files per dialect); `results/` (27 captured query outputs from
  `src/main.rs` runs). `generated_sql/{postgres,sqlite}`, which
  `src/main.rs` reads, is produced locally and is not tracked.
- `src/main.rs` + `Cargo.toml`/`Cargo.lock` — the standalone Rust/`sqlx`
  generic SQL-file runner described above.
- No `GUIDE.md` or solution write-ups — same as `gurukula_management`,
  this project isn't structured as a taught lesson.

## Status

The repo-wide `verify_all.py` harness discovers this project as
`university_bookstore/SAMSQL` and compiles and runs its schema, seed, and
each query file. Per-query results are written to
`verification_progress.json`, which is produced locally by `verify_all.py`
and is not tracked in the repo.
