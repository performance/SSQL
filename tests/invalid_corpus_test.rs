//! Integration test suite for validating compiler failure on invalid SSQL queries.
//!
//! Scans all `.सं` files under `tests/invalid_queries/` and verifies that
//! `ssql_compiler::compile_ssql_string()` returns an Error diagnostic.

use std::fs;
use std::path::Path;
use ssql_compiler::compile_ssql_string;
use ssql_compiler::SQLDialect;

#[test]
fn test_invalid_query_corpus() {
    let invalid_dir = Path::new("tests/invalid_queries");
    assert!(invalid_dir.exists(), "tests/invalid_queries directory must exist");

    // The cases' shared schema must itself compile, or every case would be
    // rejected for that one reason, and a loose expectation could not tell.
    let schema_path = invalid_dir.join("schema.सं");
    let schema = fs::read_to_string(&schema_path).expect("tests/invalid_queries/schema.सं must exist");
    if let Err(e) = compile_ssql_string(&schema, &schema_path, SQLDialect::SQLite, false) {
        panic!("tests/invalid_queries/schema.सं does not compile: {:#}", e);
    }

    let entries = fs::read_dir(invalid_dir).expect("Failed to read invalid_queries directory");
    let mut tested_count = 0;

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|ext| ext.to_str()) == Some("सं") && path.file_name().unwrap() != "schema.सं" {
            tested_count += 1;
            let query_text = fs::read_to_string(&path)
                .expect(&format!("Failed to read query file {:?}", path));

            // Each case names the error it must fail with (`-- expect: …` on
            // its first line), so it cannot pass by failing for an unrelated
            // reason (an unknown table, say); `schema.सं` beside them declares
            // the tables they use.
            let expected = query_text
                .lines()
                .next()
                .and_then(|l| l.strip_prefix("-- expect:"))
                .map(str::trim)
                .unwrap_or_else(|| panic!("{:?} has no `-- expect: <error>` first line", path.file_name().unwrap()));
            // Against the schema beside the cases, as the CLI and the
            // playground compile; `compile_ssql_string` loads none, so every
            // check that needs the tables (a table's case, say) never ran.
            let compile_result = ssql_compiler::compile_ssql_with_schema_source(&query_text, &schema, SQLDialect::SQLite);
            let error = match compile_result {
                Ok(_) => panic!("FAILURE: Invalid query {:?} was unexpectedly accepted by the compiler!", path.file_name().unwrap()),
                Err(e) => format!("{:#}", e),
            };
            assert!(
                !error.contains("schema file"),
                "{:?} was rejected because of the schema, not the query: {}",
                path.file_name().unwrap(), error
            );
            assert!(
                error.contains(expected),
                "{:?} was rejected for another reason: expected `{}`, got `{}`",
                path.file_name().unwrap(), expected, error
            );

            println!("✅ Correctly rejected invalid query case {:?}", path.file_name().unwrap());
        }
    }

    assert!(tested_count >= 8, "Expected at least 8 negative test cases in corpus, found {}", tested_count);
    println!("\n🎉 Successfully verified {} invalid query test cases in corpus!", tested_count);
}
