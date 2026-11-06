//! Error types for the SSQL compiler
//!
//! This module defines all error types used throughout the compilation pipeline.
//! Each phase (lexing, parsing, semantic analysis, code generation) has its own
//! error variants for precise error reporting.

use thiserror::Error;

/// Top-level compiler error type
#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Lexer error: {0}")]
    Lexer(#[from] LexerError),
    
    #[error("Parser error: {0}")]
    Parser(#[from] ParserError),
    
    // Not `#[from]`: that also makes the error its own source, and the
    // message was printed twice (`… शीर्षकम्: … शीर्षकम्`).
    #[error("Grammar error: {0}")]
    Semantic(SemanticError),
    
    #[error("Code generation error: {0}")]
    CodeGen(#[from] CodeGenError),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<SemanticError> for CompilerError {
    fn from(e: SemanticError) -> Self {
        CompilerError::Semantic(e)
    }
}

/// Errors during lexical analysis
#[derive(Error, Debug)]
pub enum LexerError {
    #[error("Invalid character '{0}' at line {1}, column {2}")]
    InvalidChar(char, usize, usize),
    
    #[error("Unexpected end of file while parsing {0}")]
    UnexpectedEof(String),
}

/// Errors during parsing
#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Unexpected token '{found}' at line {line}, expected {expected}")]
    UnexpectedToken {
        found: String,
        expected: String,
        line: usize,
    },
    
    #[error("Ambiguous vibhakti form '{form}' at line {line}. Could be: {possibilities}")]
    AmbiguousVibhakti {
        form: String,
        possibilities: String,
        line: usize,
    },
}

/// Errors during semantic analysis
#[derive(Error, Debug)]
pub enum SemanticError {
    #[error("Undeclared identifier '{name}' at line {line}")]
    UndeclaredIdentifier { name: String, line: usize },
    
    #[error("`{word}` is not a form of `{stem}`; write one of: {forms}")]
    NotAFormOf { word: String, stem: String, forms: String },

    #[error("`{word}` at byte offset {position}: a constraint agrees with its column `{column}`; write `{expected}`")]
    ConstraintAgreement { word: String, position: usize, column: String, expected: String },

    #[error("`{word} {pronoun}` at byte offset {position}: a declared column is an ordinary noun and is declined, `{genitive}`; the `इति`-pronoun is for a name with no declared stem (an alias)")]
    PronounForDeclaredColumn { word: String, pronoun: String, position: usize, genitive: String },

    #[error("`{word}` at byte offset {position}: the emptied column `{column}` takes `{expected}`")]
    EmptyValueAgreement { word: String, position: usize, column: String, expected: String },

    #[error("`{word} {column}`: a column's table is named in the genitive singular, `{singular} {column}`")]
    PluralTableQualifier { word: String, singular: String, column: String },

    #[error("`{word}` is the plural of a column; a column holds one value per row and is read in the singular: write `{singular}` (its genitive plural is only an aggregate's argument, `… योगः`)")]
    ColumnInPlural { word: String, singular: String },

    #[error("`{word}` is the locative, the case of a table written to; a query reads from a table in the ablative: `{ablative}`")]
    QuerySourceCase { word: String, ablative: String },

    #[error("a condition ends with `इति नियमेन` (or joins the next with `च` / `वा`), but `{after}` follows `{verb}`")]
    ConditionWithoutMarker { verb: String, after: String },

    #[error("Invalid Sanskrit word '{word}' at line {line}. Does not follow any known declension pattern.")]
    InvalidSanskritWord { word: String, line: usize },
    
    #[error("Declaration must use प्रथमा (nominative) form. Found '{found}' (vibhakti: {vibhakti}) at line {line}")]
    NonPrathamaDeclaration {
        found: String,
        vibhakti: String,
        line: usize,
    },
    
    #[error("Type mismatch: expected {expected}, found {found} at line {line}")]
    TypeMismatch {
        expected: String,
        found: String,
        line: usize,
    },
}

/// Errors during code generation
#[derive(Error, Debug)]
pub enum CodeGenError {
    #[error("Unsupported feature '{feature}' for target dialect {dialect}")]
    UnsupportedFeature { feature: String, dialect: String },
    
    #[error("Invalid query structure: {0}")]
    InvalidQuery(String),
}

/// Result type alias for compiler operations
pub type CompilerResult<T> = Result<T, CompilerError>;
