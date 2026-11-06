# SSQL Testing

SSQL source is the only thing anyone writes; SQL is compiler output. Every
check below therefore starts from `.सं` files and judges the SQL by what it
does, never by comparing it to hand-written SQL.

## The example corpus: `verify_all.py`

```bash
uv run verify_all.py --dialect all      # SQLite and PostgreSQL
uv run verify_all.py --dialect sqlite   # no PostgreSQL needed
```

For each project under `examples/`, on each dialect, it:

1. compiles `schema.सं` and creates the tables;
2. compiles `seed.सं` (where present) and loads the data;
3. compiles every query and runs it;
4. checks that SQLite and PostgreSQL return the same rows;
5. compares the rows with the reviewed golden results in `tests/golden/results/`.

It also compiles every SSQL snippet in `docs/tutorials/` (comparing it with
its paired SQL where the chapter gives one) and grades the certification
quiz. Queries whose output cannot have a fixed result, such as a query plan
or the current time, are reported as skipped.

- PostgreSQL is reached through `SSQL_PG_URL` (default
  `postgres://localhost/bookstore_pg`).
- `--compile-only` compiles without running anything, so it checks no results.
- `--bless-results` records the current rows as the golden results, only
  where both dialects agree. Read the diff before committing it.

## Compiler tests

```bash
cargo test --release -p ssql_compiler -p ssql-lsp
```

- **Language rules** (`tests/language_rules_test.rs`): a single query per case,
  either compiling to the expected SQL or failing with the expected message.
- **Invalid queries** (`tests/invalid_queries/`): each file names the error
  it must produce.
- **Ratchets** (`tests/ratchets/`): known findings may only shrink.
- **Unit tests**: inside `src/`, including the morphology engine.

## Front ends

The CLI is the reference. The playground (WebAssembly), the Python binding
and the CLI must accept the same language and emit the same SQL:

```bash
(cd website/ssql-wasm && cargo test --release)
uv run python3 website/build_site.py          # rebuilds the WASM bundle and site data
uv run python3 scripts/check_frontend_parity.py
uv run pytest -q                              # website and Python binding
```

## Adding a query

1. Write the query as a `.सं` file in the project's query folder and describe
   its objective in the project's `PROJECT.md`.
2. Make sure the seed data exercises it: a filter that removes nothing, or a
   tie that never occurs, proves nothing.
3. Run `uv run verify_all.py --dialect all`, read the new rows, and bless them
   with `--bless-results` once they are what the objective says.
