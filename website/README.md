# संस्कृतम् SQL (SSQL) — Launch Website & Interactive Web Playground

This directory contains the static launch website and WebAssembly-powered interactive playground for **SSQL (संस्कृतम् SQL)**.

## Architecture

- **[`index.html`](index.html), [`styles.css`](styles.css), [`app.js`](app.js):**
  - **Hero & Kāraka Reference:** Explains Pāṇinian *Kāraka*-to-relational mapping (`पञ्चमी` $\to$ `FROM`, `सहार्थे तृतीया` $\to$ `JOIN`, `नियमेन` $\to$ `WHERE`, `समूहेन` $\to$ `GROUP BY`, `षष्ठी` $\to$ `table.column`, `सप्तमी` $\to$ `INSERT/UPDATE/PARTITION`, `कर्मणि लोट्` $\to$ `SELECT` verb concordance).
  - **Interactive Playground:**
    - 8 curated scenarios on the **University Bookstore** dataset (Basic Filtering, Joins & Aggregations, `LEFT JOIN` + `IS NULL`, `HAVING` + `ORDER BY`, Window `RANK()`, CTEs, Pure UDFs with `दीयते`/`प्रदाय`, and Strict Gender/Number Concordance Diagnostics).
    - Live **SQLite** and **PostgreSQL** dialect codegen.
    - Live query execution against the University Bookstore SQLite database (via in-browser `sql.js` WebAssembly or the offline-capable [`server.py`](server.py) fallback).
    - **अध्येता (Adhyetā) Grammatical Breakdown:** Inspects every token's surface form, *Prātipadika* (stem), *Vibhakti* (case), *Vacana* (number), *Liṅga* (gender), *Kāraka* role, and SQL mapping.
    - **One-Click Tooling:** Canonical Formatter (`--fmt`), Linguistic Purity Linter (`--lint`), and Reverse SQL $\to$ SSQL Transpiler (`--from-sql`).
- **[`ssql-wasm/`](ssql-wasm/) & [`pkg/`](pkg/):**
  - Standalone Rust-to-WebAssembly crate (`wasm-bindgen`) wrapping `ssql_compiler` on the browser's main thread without modifying any files in `src/`.

## Running & Testing Locally (`uv` Frozen Venv)

Sync the locked `uv` virtual environment (`uv.lock`) and start the local server:

```bash
uv sync --frozen
uv run --frozen python website/server.py --port 8080
```

Then open `http://127.0.0.1:8080` in your browser.

To run the automated end-to-end website and Python bindings test suite against the frozen environment:

```bash
uv run --frozen pytest
```

## Rebuilding the WebAssembly Package

If you update [`ssql-wasm/src/lib.rs`](ssql-wasm/src/lib.rs), rebuild `website/pkg/` with:

```bash
cd website/ssql-wasm
cargo test
wasm-pack build --target web --out-dir ../pkg --no-opt
rm -f ../pkg/.gitignore
```
