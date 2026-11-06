# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in SSQL (the compiler, the Python
bindings, the language server, or the VS Code extension), please report it
privately rather than opening a public issue.

Email **sriram.durbha@gmail.com** with:

- A description of the vulnerability and its potential impact.
- Steps to reproduce it (a minimal SSQL source file and/or command is
  ideal).
- Any suggested fix or mitigation, if you have one.

You should expect an initial response within a few days. Once a fix is
available, we'll coordinate on disclosure timing with you.

## Scope

Examples of in-scope issues:

- SQL injection via SSQL identifiers, string literals, or generated SQL
  (the compiler generates SQL text that is executed against real
  databases; identifier/name validation is a real security boundary).
- Path traversal or arbitrary file access via compiler CLI arguments or
  schema auto-discovery.
- Memory-safety issues in the Rust compiler or Python bindings (`unsafe`
  code, FFI boundaries).

## Supported Versions

This project does not yet have a stable release line with long-term
support; security fixes are applied to the latest version on the default
branch.
