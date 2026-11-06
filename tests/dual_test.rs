//! A table named by a dual word (`दम्पती`, "a couple") is declared in the dual
//! and read in the dual.

use ssql_compiler::{compile_ssql_with_schema_source, SQLDialect};

const SCHEMA: &str = r#"
दम्पती इति पटलम् "couples" इति आख्या दम्पति इति पुंलिङ्गम् द्विवचनम् रच्यताम् यत्र
    प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    मूल्यम् इति पूर्णाङ्कः "price" इति आख्या मूल्य इति नपुंसकलिङ्गम् एकवचनम् च
  इति पटलम् ।
"#;

fn compile(ssql: &str) -> Result<String, String> {
    compile_ssql_with_schema_source(ssql, SCHEMA, SQLDialect::SQLite)
        .map(|sql| sql.split_whitespace().collect::<Vec<_>>().join(" "))
        .map_err(|e| format!("{:#}", e))
}

#[test]
fn read_in_the_dual() {
    let sql = compile("दम्पतिभ्याम् मूल्यम् ५ इत्यस्मात् अधिकम् अस्ति इति नियमेन सङ्ख्या आह्रियताम् ।").unwrap();
    assert!(sql.contains("SELECT t1.id FROM couples t1 WHERE t1.price > 5"), "{}", sql);
}

#[test]
fn qualified_by_its_singular() {
    let sql = compile("दम्पतिभ्याम् दम्पतेः मूल्यम् आह्रियताम् ।").unwrap();
    assert!(sql.contains("SELECT t1.price FROM couples t1"), "{}", sql);
}

#[test]
fn plural_is_not_the_table() {
    let error = compile("दम्पतिभ्यः मूल्यम् आह्रियताम् ।").unwrap_err();
    assert!(error.contains("read in the ablative dual"), "{}", error);
}

#[test]
fn written_to_in_the_locative_dual() {
    let sql = compile("दम्पत्योः सङ्ख्या १ च मूल्यम् २ च निवेश्येताम् ।").unwrap();
    assert!(sql.contains("INSERT INTO couples (id, price) VALUES (1, 2)"), "{}", sql);
    let sql = compile("दम्पत्योः सङ्ख्या ५ इत्यस्याः अधिका अस्ति इति नियमेन मूल्यम् १ इति परिवर्त्यताम् ।").unwrap();
    assert!(sql.contains("UPDATE couples SET price = 1 WHERE id > 5"), "{}", sql);
}

#[test]
fn deleted_from_in_the_ablative_dual() {
    let sql = compile("दम्पतिभ्याम् सङ्ख्या ५ इत्यस्याः अधिका अस्ति इति नियमेन निष्कास्यताम् ।").unwrap();
    assert!(sql.contains("DELETE FROM couples WHERE id > 5"), "{}", sql);
}

#[test]
fn a_column_is_never_dual() {
    let schema = r#"
वस्तूनि इति पटलम् "items" इति आख्या वस्तु इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
    मूल्ये इति पूर्णाङ्कः "prices" इति आख्या मूल्य इति नपुंसकलिङ्गम् द्विवचनम्
  इति पटलम् ।
"#;
    let error = compile_ssql_with_schema_source("वस्तुभ्यः सर्वम् आह्रियताम् ।", schema, SQLDialect::SQLite).unwrap_err();
    let error = format!("{:#}", error);
    assert!(error.contains("is declared dual"), "{}", error);
}
