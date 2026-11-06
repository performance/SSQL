//! SQL Dialect Adaptors
//!
//! This module provides a trait-based abstraction for generating SQL
//! for different database engines. Each adaptor implements dialect-specific
//! SQL generation while sharing a common interface.

pub mod traits;
pub mod sqlite;
pub mod postgres;

pub use traits::*;
pub use sqlite::SQLiteAdaptor;
pub use postgres::PostgresAdaptor;
