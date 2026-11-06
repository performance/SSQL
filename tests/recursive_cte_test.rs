//! The cycle guard of a recursive CTE (`आवर्तिका … चक्रनिवारणेन`), the same on
//! both dialects.

use ssql_compiler::{compile_ssql_with_schema_source, SQLDialect};

const SCHEMA: &str = include_str!("../examples/19_hr_org_chart/schema.सं");
const QUERY: &str = include_str!("../examples/19_hr_org_chart/queries/q1_elegant.सं");

#[test]
fn cycle_guard_delimits_every_id() {
    // The guard is `instr(path, CHAR(31) || id || CHAR(31)) = 0`, an exact
    // search (LIKE read `%` and `_` in a text id as wildcards), so every id
    // on the path must be closed by its own CHAR(31). With only
    // the root's closed (`|1|23`) the guard caught a return to the root and
    // nothing else, and a cycle below it recursed forever.
    let sql = compile_ssql_with_schema_source(QUERY, SCHEMA, SQLDialect::SQLite).expect("q1_elegant compiles");
    let sql = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(sql.contains("CHAR(31) || t1.employee_id || CHAR(31) AS path"), "{}", sql);
    assert!(sql.contains("t3.path || t2.employee_id || CHAR(31) AS path"), "{}", sql);
    assert!(sql.contains("instr(t3.path, CHAR(31) || t2.employee_id || CHAR(31)) = 0"), "{}", sql);
}

#[test]
fn postgres_uses_the_same_guard() {
    // PostgreSQL's `CYCLE … SET is_cycle` keeps the row that closes the
    // cycle unless every reader filters on is_cycle; the path guard drops it
    // as SQLite's does.
    let sql = compile_ssql_with_schema_source(QUERY, SCHEMA, SQLDialect::PostgreSQL).expect("q1_elegant compiles");
    let sql = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(!sql.contains("CYCLE"), "{}", sql);
    assert!(sql.contains("t3.path || t2.employee_id || chr(31) AS path"), "{}", sql);
    assert!(sql.contains("strpos(t3.path, chr(31) || t2.employee_id || chr(31)) = 0"), "{}", sql);
}
