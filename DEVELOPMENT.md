# Developing SSQL

This guide covers building, testing, and contributing to the SSQL compiler
itself.

## Prerequisites

- **Rust** (stable toolchain) - install via [rustup.rs](https://rustup.rs/).
- **Python 3** with [uv](https://docs.astral.sh/uv/) - used to run `verify_all.py` and other repo scripts.
- **PostgreSQL** (optional) - only needed to run the Postgres half of the verification suite; SQLite needs no separate server.

## Building

```bash
cargo build --release
```

The compiler binary is at `target/release/ssql_compiler`.

## Running the compiler

```bash
# Compiles <file>.सं to SQL, auto-discovering a sibling schema.सं
cargo run --bin ssql_compiler -- path/to/query.सं --dialect sqlite
```

See the [README](README.md)'s Quick Start for a minimal schema+query example.

## Testing

```bash
cargo test --lib
```

## Verifying against the example corpus

`verify_all.py` compiles and executes every worked example under `examples/`
against a real database, for both dialects:

```bash
uv run verify_all.py --dialect sqlite
uv run verify_all.py --dialect postgres   # needs a local Postgres - see docs/Testing.md
```

Run the two dialects **sequentially**, not concurrently - they share a
progress-log file and will corrupt each other's output if run in parallel.

If your local Postgres role/database differs from the default, override it:

```bash
export SSQL_PG_URL="postgres://youruser@localhost/yourdb"
```

## Repository layout

```
SSQL/
├── src/                  # Compiler source (Rust)
│   ├── lexer.rs          # Tokenizes .सं source
│   ├── annotator.rs      # Two-pass vibhakti annotation
│   ├── ssql_parser/      # chumsky-based parser
│   ├── ast_codegen.rs    # AST -> SQL
│   └── codegen/          # Per-dialect (SQLite/Postgres) SQL emission
├── examples/             # 37 worked example projects, each schema.सं + queries
├── docs/                 # Tutorials and reference documentation
├── src/design/           # Design notes on the grammar rules
├── ssql-lsp/             # Language server
├── ssql-py/              # Python bindings
└── editors/vscode/       # VS Code extension
```

## Contributing

1. For a non-trivial feature or grammar change, open a feature-request issue
   first, describing the syntax, the grammatical reasoning, and the
   alternatives considered.
2. Add or update example queries under `examples/` and/or tests in
   `src/repro_test.rs` that exercise the change.
3. Make sure `cargo build --release`, `cargo test --lib`, and
   `uv run verify_all.py --dialect sqlite` (and `postgres`, if you have it
   set up) are all clean.
4. Open a pull request. Sanskrit-grammar changes should include the
   reasoning (which vibhakti/rule applies and why) in the PR description,
   since that's usually the crux of the review.

See `docs/reference/constitution.md` for the project's core design philosophy
(graphemic purity, grammar-driven semantics) and `src/design/rules.md` for
the grammar rules the parser implements.
