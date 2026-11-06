# Contributing to SSQL

Thanks for your interest in contributing! This project welcomes bug
reports, example projects, documentation fixes, and feature proposals.

## Before you start

By participating in this project you agree to abide by the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Building, testing, and the contribution workflow

See [DEVELOPMENT.md](DEVELOPMENT.md) for prerequisites, build/test/verify
commands, the repository layout, and the step-by-step contribution
workflow (an issue first for non-trivial changes, adding example queries and
tests, what to run before opening a PR).

## Help wanted: other platforms

Release packages are currently built only for macOS on Apple silicon. If you
build and test the compiler, the language server or the Python wheel on
Linux or Windows, please report what worked, or open a PR that adds a
release build for that platform.

## Reporting bugs

Open a [GitHub issue](https://github.com/performance/SSQL/issues) with:

- The SSQL source (`.सं`) that triggers the problem, and the exact command
  you ran.
- What you expected vs. what happened (compiler error text, wrong SQL
  output, etc.).
- Your OS and Rust toolchain version (`rustc --version`).

If you believe you've found a security vulnerability, please follow
[SECURITY.md](SECURITY.md) instead of opening a public issue.

## Proposing grammar or syntax changes

SSQL's Sanskrit surface syntax is a deliberate design surface, not an
implementation detail - changes to it affect every existing query. For
anything beyond a small bug fix, open a feature-request issue describing
the syntax, the grammatical reasoning, and alternatives considered, before
opening a PR with the implementation.

## Pull requests

1. Keep PRs focused - one feature or fix per PR.
2. Add or update example queries under `examples/` and/or tests in
   `src/repro_test.rs` that exercise the change.
3. Make sure `cargo build --release`, `cargo test --lib`, and
   `uv run verify_all.py --dialect sqlite` are all clean before requesting
   review.
4. For Sanskrit-grammar changes, explain the reasoning (which vibhakti/rule
   applies and why) in the PR description - that's usually the crux of the
   review.
