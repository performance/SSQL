# SSQL (Samskrit SQL) Compiler

> [!NOTE]
> **Joins are explicit.** A column of another table can be used only once that
> table is joined, with `<table> सह <condition> इति मेलनेन`:
>
> ```ssql
> विक्रयणेभ्यः
>     विक्रयाधिकारिभिः सह विक्रयणस्य विक्रयाधिकारिसङ्ख्या विक्रयाधिकारिणः सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
>     विक्रयाधिकारिणः नाम आह्रियताम् ।
> ```
>
> A table is named in the plural as a source (`विक्रयणेभ्यः`) or a join
> (`विक्रयाधिकारिभिः सह`); a column's table is named in the genitive singular
> (`विक्रयाधिकारिणः नाम`, "the rep's name"). Without the join,
> `विक्रयणेभ्यः विक्रयाधिकारिणः नाम आह्रियताम् ।` is an error.

A compiler for **SSQL (Samskrtam SQL)** - a Sanskrit-based query language that compiles to SQL (SQLite, PostgreSQL). This project enables database operations using pure Devanagari syntax with grammatically-correct Sanskrit vibhakti (case) system.

## 🎯 Project Status

**Current Phase:** Core language implemented and under active verification.

- ✅ **Automatic Morphology**: Define only Prathama (Nominative) forms; compiler generates all 7 vibhaktis.
- ✅ **Multi-Engine Support**: Generates SQL for both SQLite and PostgreSQL.
- ✅ **Real-World Verification**: `uv run verify_all.py` compiles and executes the full corpus of 37 worked example projects under `examples/` against both dialects.
- ✅ **Type System**: Full support for Vachan (Number) and Vibhakti (Case).
- ✅ **Symbol Table**: Context-aware identifier resolution.
- ⚠️ Development status: core features are verified against a large worked-example corpus; some advanced constructs are still being hardened.

## 🌟 Key Features

### 1. Grammar-Based Semantics
SSQL uses Sanskrit `vibhakti` (grammatical cases) to strictly enforce semantic intent, eliminating SQL keywords like `FROM`, `WHERE`, `SELECT`:

- **प्रथमा (Nominative)**: Subjects (`Table`), Select Columns (`Column`).
- **तृतीया (Instrumental)**: Conditions (`WHERE`), Joins (`WITH`), Grouping (`BY`).
- **पञ्चमी (Ablative)**: Source (`FROM table`).
- **षष्ठी (Genitive)**: Property access (`table.field`).
- **सप्तमी (Locative)**: Target for DML (`INSERT INTO`, `UPDATE IN`).

### 2. Pure Devanagari Syntax
Adheres to the [constitution](docs/reference/constitution.md) of maintaining graphemic purity. 
> *Example*: Instead of `SELECT title FROM books`, SSQL says `पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।` ("from the books, let the title be fetched").

### 3. Database Interoperability
Schemas are declared directly in SSQL itself, using native `इति आख्या ... रच्यताम्` CREATE TABLE syntax - there is no separate mapping config file.

## 📁 Project Structure

```
SSQL/
├── src/                  # Core Compiler, Formatter, Linter & Reverse Transpiler (Rust)
├── ssql-lsp/             # Language Server Protocol (LSP) for .सं files
├── ssql-py/              # Native Python bindings (PyO3 / Maturin)
├── editors/vscode/       # VS Code Extension (Devanagari syntax, LSP client & commands)
├── website/              # Launch Website & WebAssembly Interactive Playground
├── docs/tutorials/       # Dual-Track Tutorials (SQL Devs & Sanskrit Learners)
└── examples/
    ├── university_bookstore/   # 🏆 Main Reference Project (50+ queries)
    │   ├── SAMSQL/             # SSQL Source Files (.सं), including schema.सं
    │   └── src/                # Rust Runner (Test Harness)
    └── ...                     # 30+ real-world domain benchmark suites
```

## 🚀 Quick Start

**Platforms:** the prebuilt compiler, language server and Python wheel on the
website's Downloads section are for macOS on Apple silicon only; the VS Code
extension and the WebAssembly package run anywhere. On Linux, Windows or
Intel macOS, build from source with a stable Rust toolchain
(`cargo build --release -p ssql_compiler -p ssql-lsp`); CI builds and tests
on Linux. Prebuilt packages for other platforms are a welcome contribution.

### 1. Interactive Web Playground (WebAssembly)
Sync the locked `uv` virtual environment and launch the interactive SSQL playground locally in your browser (compiles SSQL in WebAssembly and executes queries against the University Bookstore SQLite database):

```bash
uv sync --frozen
uv run --frozen python website/server.py --port 8080
# Open http://127.0.0.1:8080

# Run Python bindings & website end-to-end tests via the frozen uv venv
uv run --frozen pytest
```

### 2. Schema (Native Adjectival DDL)
Declare each table's stem, grammatical gender, and vacana; the compiler generates all 21 declined forms automatically. Save this as `schema.सं`:

```ssql
छात्राः इति पटलम् "students" इति आख्या छात्र इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    प्राथमिकी क्रमिका सङ्ख्या इति पूर्णाङ्कः "id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    अनिवार्यम् नाम इति वाक्यम् "name" इति आख्या नामन् इति नपुंसकलिङ्गम् एकवचनम् च
    वयोमानम् इति पूर्णाङ्कः "age" इति आख्या वयोमान इति नपुंसकलिङ्गम् एकवचनम् च
  इति पटलम् ।
```

### 3. Compile, Format, Lint & Reverse-Transpile

```bash
# Compile SSQL (.सं) to SQL (auto-discovers sibling schema.सं)
cargo run --bin ssql_compiler -- "examples/university_bookstore/SAMSQL/01_books_published_after_2010.सं" --dialect sqlite

# Format SSQL code canonically (--fmt) or run Pāṇinian purity linter (--lint)
cargo run --bin ssql_compiler -- --fmt query.सं
cargo run --bin ssql_compiler -- --lint query.सं

# Reverse-transpile existing SQL into strict-mode SSQL (--from-sql)
cargo run --bin ssql_compiler -- --from-sql legacy.sql --schema schema.सं -o query.सं

# Verify the full example corpus against both dialects
uv run verify_all.py --dialect sqlite
uv run verify_all.py --dialect postgres   # needs a local Postgres; see docs/Testing.md
```

## 📚 Language Guide

### Multi-Vibhakti Resolution
A single Sanskrit word can map to multiple grammatical slots. The compiler's **Morphology Engine** generates all potential forms, and the **Parser** resolves ambiguity based on context.

### Pure User-Defined Functions (`शुद्धम् कार्यम्`)
Define side-effect-free functions whose parameters are given with the passive (`दीयते`/`दीयेते`/`दीयन्ते`, agreeing in number), and invoke them with Genitive (`षष्ठी`) or `प्रदाय` ("having supplied"):

```ssql
करसहितमूल्यम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
    मूल्यम् इति वास्तविकः च करः इति वास्तविकः च दीयेते
    मूल्यम् च करः च इत्यनयोः योगः इति उत्तरम् प्रतिदीयताम्
इति कार्यम् ।

पुस्तकेभ्यः शीर्षकम् च मूल्यम् च ५ च प्रदाय करसहितमूल्यम् सकलमूल्यम् इति आख्या च आह्रियेताम् ।
```

## 📚 Documentation
- **Dual-Track Tutorials**:
  - [SSQL for SQL Developers](docs/tutorials/ssql-for-sql-developers/01-introduction.md)
  - [Samskrutam Through SSQL](docs/tutorials/samskrutam-through-ssql/01-vibhakti-overview.md)
- **Editor & IDE Support**: See [editors/vscode/README.md](editors/vscode/README.md) and `ssql-lsp/`.
- **Web Playground**: See [website/README.md](website/README.md).
- **Design Philosophy**: See [docs/README.md](docs/README.md) and [constitution.md](docs/reference/constitution.md).

## 🤝 Contributing & License
Licensed under [Apache 2.0](LICENSE).

