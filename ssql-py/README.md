# SSQL Python Bindings

This package (`ssql`) provides Python bindings for the **Samskrtam SQL (SSQL)** compiler.

📘 **[Read the SSQL Language Tour](SSQL_QUICKSTART.md)** to learn how to write SSQL queries.

## Prerequisites

- **Python**: 3.10+
- **Rust**: 1.70+ (required for building from source)
- **Maturin**: Build tool for Rust+Python (`pip install maturin`)

## Installation

### Option 1: Development (Local)
To build and install the package locally in your current environment:

**Using standard pip:**
```bash
cd ssql-py
maturin develop
```

**Using uv (Recommended):**
```bash
cd ssql-py
uv venv
source .venv/bin/activate
uv pip install .
```

### Option 2: Production (Wheels)
To build a distributable wheel file (`.whl`):

```bash
maturin build --release
```
The wheel will be located in `target/wheels/`. You can install it anywhere:
```bash
pip install target/wheels/ssql-0.2.1-cp3XX-...whl
```

## Usage

The `ssql` module exposes a `Compiler` class.

```python
import ssql

# 1. Initialize Compiler
# Pass native SSQL schema DDL (the same CREATE TABLE syntax used in a
# project's schema.सं file) as a string, plus the target dialect
# ("sqlite" or "postgres").
schema_ddl = """
पुस्तकानि इति पटलम् "books" इति आख्या पुस्तक इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
    मूल्यम् इति वाक्यम् "price" इति आख्या मूल्य इति नपुंसकलिङ्गम् एकवचनम्
  इति पटलम् ।
"""
compiler = ssql.Compiler(schema_ddl, "sqlite")

ssql_source = """
पुस्तकेभ्यः मूल्यम् आह्रियताम् ।
"""

# 2. Compile (Safe Mode - Recommended)
# Returns a result object with .success, .sql, and .error fields
result = compiler.compile_safe(ssql_source)

if result.success:
    print("Generated SQL:")
    print(result.sql)
else:
    print(f"Compilation Failed: {result.error}")

# 3. Compile (Standard Mode)
# Raises ssql.CompileError on failure
try:
    sql = compiler.compile(ssql_source)
    print(sql)
except ssql.CompileError as e:
    print(f"Error: {e}")
```

## Distribution

To distribute this package to other users or servers:

1.  **Build Wheels**: Run `maturin build --release`. This creates optimized binaries.
2.  **Cross Compilation**: Maturin supports cross-compilation (e.g., building Linux wheels on Mac) using Docker.
    ```bash
    maturin build --release --zig
    ```
3.  **PyPI**: You can publish directly to PyPI:
    ```bash
    maturin publish
    ```

## Error Handling

The package exports a custom exception `CompileError` for catching SSQL-specific compilation issues (syntax errors, type mismatches, etc.).

```python
from ssql import Compiler, CompileError
```
