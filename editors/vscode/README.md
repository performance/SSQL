# SSQL for VS Code

Language support for **SSQL (Samskrtam Structured Query Language)** — Devanagari-aware syntax highlighting, Language Server Protocol (`ssql-lsp`) integration, formatting, linting, grammatical explanation, and bidirectional SSQL $\leftrightarrow$ SQL compilation for `.सं` files.

## Features

- **Devanagari Syntax Highlighting**: Highlights SSQL verbs, *Kāraka* postpositions, types, constraints, aggregates, window functions, and Devanagari numerals (`०`–`९`).
- **Language Server (`ssql-lsp`)**:
  - Automatic discovery of project-level `schema.सं` as well as in-file DDL and CTE declarations.
  - Line- and token-accurate diagnostics from the lexer, parser, Pāṇinian static validator, and style/purity linter (`LINT-001`–`LINT-004`).
  - Hover documentation showing resolved SQL table/column mappings and grammatical roles.
  - Go-to-Definition (`F12`) across in-file DDL/CTE definitions and project `schema.सं`.
  - Document formatting (`Shift+Alt+F`).
- **Commands** (Command Palette or Editor Context Menu):
  - `SSQL: Generate SQL (SQLite)` / `SSQL: Generate SQL (PostgreSQL)` — compile the active `.सं` file to SQL.
  - `SSQL: Format Document (--fmt)` — format the current `.सं` file in canonical SSQL layout.
  - `SSQL: Lint Document (--lint)` — run the SSQL style and purity linter.
  - `SSQL: Transpile SQL to SSQL (--from-sql)` — reverse-transpile a `.sql` file into idiomatic, parenthesis-free SSQL.
  - `SSQL: View AST` / `SSQL: View Symbol Table` — inspect the compiler AST and resolved symbol table.
  - `SSQL: Install CLI Tools` — build and install `ssql_compiler` and `ssql-lsp` via Cargo.

## Requirements

The extension looks for `target/release/ssql-lsp` first, then `target/debug/ssql-lsp`, and finally `ssql-lsp` on your `PATH`. Build the release binaries from the repository root:

```bash
cargo build --release -p ssql_compiler -p ssql-lsp
```

Or configure an explicit binary path via the `ssql.serverPath` setting.

## Configuration

| Setting | Description |
|---|---|
| `ssql.serverPath` | Path to the `ssql-lsp` binary, if not on `PATH`. |
| `ssql.trace.server` | Trace communication between VS Code and the language server (`off` / `messages` / `verbose`). |

## Learn SSQL

See the main repository at [github.com/performance/SSQL](https://github.com/performance/SSQL) and `docs/tutorials/` for the dual-track tutorial curriculum and 30 domain benchmarks.
