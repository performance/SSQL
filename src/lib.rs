//! # SSQL Compiler
//! 
//! A compiler for the Samskrtam SQL (SSQL) language - a Sanskrit-based query language
//! that compiles to SQL (SQLite, PostgreSQL, etc.)
//!
//! ## Architecture
//!
//! ```text
//! SSQL Source (.सं)
//!     ↓
//! Lexer (`src/lexer.rs`) → Grapheme-aware Devanagari tokens
//!     ↓
//! Pass 1: Symbol Table & DDL Schema Builder (`src/schema.rs`, `src/morphology.rs`)
//!     ↓
//! Pass 2: Vibhakti Annotator (`src/annotator.rs`) → Enriches tokens with grammatical case info
//!     ↓
//! Parser (`src/ssql_parser/`) → Chumsky combinator AST construction
//!     ↓
//! Semantic Validator (`src/validator.rs`) → Strict Kāraka, Linga & Vachana concordance
//!     ↓
//! Code Generator (`src/ast_codegen.rs`, `src/codegen/`) → SQL output (SQLite/PostgreSQL)
//! ```
//!
//! ## Key Principles
//!
//! - **Grammar-Based Programming**: Uses Sanskrit vibhakti (grammatical cases) for semantics
//! - **Zero Loan Words**: Pure Devanagari syntax (see constitution.md)
//! - **Safe by Default**: Strict error checking, explicit declarations required
//! - **Real-World Ready**: Interop with existing databases via native SSQL schema DDL (`इति आख्या ... रच्यताम्`)

pub mod annotator;
pub mod ast;
pub mod codegen;       // Dialect-specific SQL adaptors
pub mod compiler;      // Compiler pipeline orchestration
pub mod error;
pub mod lexer;
pub mod morphology;
pub mod schema;        // Native SSQL DDL schema registry
pub mod symbol_table;
pub mod types;
pub mod resolution;
pub mod ssql_parser;   // Chumsky combinator parser
pub mod ast_codegen;   // AST to SQL code generator
pub mod reserved_words; // Shared builtin-function-name reservation list
pub mod validator;
pub mod ast_transformer; // AST transformer for identifier resolution
pub mod analysis;      // Analysis for LSP
pub mod sql_importer;  // Automated SQL -> SSQL reverse transpiler
pub mod sql_converter; // SQL scripts -> SSQL, shared by every front end
pub mod formatter;     // Canonical SSQL code formatter (--fmt)
pub mod linter;        // SSQL style and purity linter (--lint)
pub mod declaration_check; // Declared name must be the nominative of its stem/gender/number
pub mod lexicon;       // Denied (non-Sanskrit) words and orthography rules

/// Every fixed word of SSQL: the grammar's keywords, the words it matches by
/// spelling, and the built-in function names. The editors (LSP completion,
/// VS Code, the playground) take their word lists from here, and a test
/// checks theirs against it.
pub fn vocabulary() -> Vec<&'static str> {
    // Forms the lexer knows only so the compiler can reject them with the
    // right word: a misagreeing adjective (क्रमिकी, वैकल्पिका), a bare stem
    // (घात), a retired verb. Editors must not offer them.
    const REJECTED: &[&str] = &["क्रमिकी", "वैकल्पिका", "घात", "चियेताम्", "चियन्ताम्", "ऋणात्मक"];
    let mut words: Vec<&'static str> = ssql_parser::token::ALL_KEYWORD_STRINGS
        .iter()
        .chain(ssql_parser::token::SPELLED_WORDS)
        .chain(reserved_words::BUILTIN_FUNCTION_WORDS)
        .copied()
        .filter(|w| !REJECTED.contains(w))
        .chain(morphology::negative_words().iter().map(String::as_str))
        .collect();
    words.sort_unstable();
    words.dedup();
    words
}

#[cfg(test)]
pub mod repro_test;
#[cfg(test)]
mod reserved_words_test;

use std::path::Path;
use anyhow::{Context, Result};

/// Which SQL dialect to compile to. The only two dialects this project
/// targets (see `src/codegen/{sqlite,postgres}.rs`'s `SqlDialectAdaptor`
/// implementations, which do the real per-dialect codegen work - this enum
/// is just the public-API selector for them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SQLDialect {
    SQLite,
    PostgreSQL,
}

/// Main entry point for compiling SSQL source to SQL
/// Uses the AST-based compiler pipeline
pub fn compile_ssql(source_path: &Path, target: SQLDialect, dump_ast: bool) -> Result<String> {
    log::info!("Compiling SSQL source: {}", source_path.display());
    
    // Determine dialect string for compiler
    let dialect = match target {
        SQLDialect::SQLite => "sqlite",
        SQLDialect::PostgreSQL => "postgres",
    };
    
    // Create compiler
    let config = compiler::CompilerConfig {
        dialect: dialect.to_string(),
        verbose: false,
        dump_ast,
        dump_resolved: false,
        dump_symtable: false,
    };
    
    let mut v2_compiler = compiler::Compiler::new(config)
        .context("Failed to create v2 compiler")?;
    
    // Run compiler pipeline
    match v2_compiler.compile(source_path) {
        Ok(sql) => {
            log::info!("V2 compiler succeeded");
            Ok(sql)
        }
        Err(e) => {
            log::warn!("V2 compiler failed: {}", e);
            Err(e)
        }
    }
}

/// Compile SSQL source with full config
pub fn compile_ssql_with_config(
    source_path: &Path,
    config: compiler::CompilerConfig
) -> Result<String> {
    compile_ssql_with_dumps(source_path, config).map(|(sql, _)| sql)
}

/// Compile and also return the dumps the config's `dump_*` flags asked for.
pub fn compile_ssql_with_dumps(
    source_path: &Path,
    config: compiler::CompilerConfig
) -> Result<(String, Vec<String>)> {
    log::info!("Compiling SSQL source: {}", source_path.display());

    let mut v2_compiler = compiler::Compiler::new(config)
        .context("Failed to create v2 compiler")?;

    // Run compiler pipeline
    match v2_compiler.compile(source_path) {
        Ok(sql) => {
            log::info!("V2 compiler succeeded");
            Ok((sql, std::mem::take(&mut v2_compiler.dumps)))
        }
        Err(e) => {
            log::warn!("V2 compiler failed: {}", e);
            Err(e)
        }
    }
}


/// Compile SSQL source string to SQL
/// Useful for LSP or in-memory compilation
pub fn compile_ssql_string(
    source: &str,
    source_path: &Path,
    target: SQLDialect,
    dump_ast: bool
) -> Result<String> {
    // Determine dialect
    let dialect = match target {
        SQLDialect::SQLite => "sqlite",
        SQLDialect::PostgreSQL => "postgres",
    };

    // Configure compiler for string compilation
    let config = compiler::CompilerConfig {
        dialect: dialect.to_string(),
        verbose: false,
        dump_ast,
        dump_resolved: false,
        dump_symtable: false,
    };

    let mut v2_compiler = compiler::Compiler::new(config)
        .context("Failed to create v2 compiler")?;

    match v2_compiler.compile_string_with_path(source, source_path) {
        Ok(sql) => {
            log::info!("V2 compiler succeeded");
            Ok(sql)
        }
        Err(e) => {
            log::warn!("V2 compiler failed: {}", e);
            Err(e)
        }
    }
}


/// Compile SSQL source against a `schema.सं` given as text.
///
/// This is the one shared entry point for every front-end that does not read
/// files itself (WASM playground, Python binding, LSP, tests): it runs exactly
/// the CLI pipeline (`Compiler::load_schema_from_ssql_string` followed by
/// `Compiler::compile_string`), so all front-ends accept and reject the same
/// language and emit the same SQL. An empty `schema_source` compiles with
/// in-file declarations only.
pub fn compile_ssql_with_schema_source(
    source: &str,
    schema_source: &str,
    target: SQLDialect,
) -> Result<String> {
    let dialect = match target {
        SQLDialect::SQLite => "sqlite",
        SQLDialect::PostgreSQL => "postgres",
    };
    let config = compiler::CompilerConfig {
        dialect: dialect.to_string(),
        verbose: false,
        dump_ast: false,
        dump_resolved: false,
        dump_symtable: false,
    };
    let mut v2_compiler = compiler::Compiler::new(config)
        .context("Failed to create v2 compiler")?;
    if !schema_source.trim().is_empty() {
        v2_compiler
            .load_schema_from_ssql_string(schema_source)
            .context("Failed to load schema.सं")?;
    }
    v2_compiler.compile_string(source)
}

/// Analyze SSQL source string for LSP (Symbols, etc.)
pub fn analyze_ssql_string(
    source: &str,
    source_path: &Path,
    target: SQLDialect,
) -> Result<crate::analysis::AnalysisData> {
    // Determine dialect via dummy map (not strictly needed for analysis but for compiler init)
    let dialect = match target {
        SQLDialect::SQLite => "sqlite",
        SQLDialect::PostgreSQL => "postgres",
    };

    let config = compiler::CompilerConfig {
        dialect: dialect.to_string(),
        verbose: false,
        dump_ast: false,
        dump_resolved: false,
        dump_symtable: false,
    };

    let mut v2_compiler = compiler::Compiler::new(config)
        .context("Failed to create v2 compiler")?;

    if let Ok(ddl_source) = std::fs::read_to_string(source_path) {
        let _ = v2_compiler.compile_string_with_path(&ddl_source, source_path);
    }
    v2_compiler.analyze_string(source)
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_placeholder() {
        // This will be replaced with actual tests
        assert!(true);
    }
}
