//! Table-driven language-rules suite.
//!
//! Each case compiles a query against the real `university_bookstore` schema
//! through `compile_ssql_with_schema_source` (the same entry point the CLI,
//! WASM and Python bindings use) and asserts either the exact SQL fragment it
//! must produce or the error it must be rejected with. Accepted queries are
//! also prepared against an in-memory SQLite database built from the compiled
//! schema, so a query that "compiles" to invalid SQL fails here.

use rusqlite::Connection;
use ssql_compiler::{compile_ssql_with_schema_source, SQLDialect};

const SCHEMA: &str = include_str!("../examples/university_bookstore/SAMSQL/schema.सं");

enum Expect {
    /// Compiles, and the whitespace-normalised SQL contains this fragment.
    Compiles(&'static str),
    /// Rejected, and the error message contains this fragment.
    Rejects(&'static str),
}
use Expect::*;

struct Case {
    name: &'static str,
    ssql: &'static str,
    expect: Expect,
}

const CASES: &[Case] = &[
    // --- Projection verb agreement (number of projected columns) ---
    Case { name: "select_one_column_singular", ssql: "पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।", expect: Compiles("SELECT t1.title FROM books t1") },
    Case { name: "select_two_columns_dual", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यम् च आह्रियेताम् ।", expect: Compiles("SELECT t1.title, t1.price FROM books t1") },
    Case { name: "select_three_columns_plural", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यम् च भाषा च आह्रियन्ताम् ।", expect: Compiles("SELECT t1.title, t1.price, t1.language FROM books t1") },
    Case { name: "select_three_columns_rejects_dual", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यम् च भाषा च आह्रियेताम् ।", expect: Rejects("Agreement") },
    Case { name: "select_requires_verb", ssql: "लेखकेभ्यः नाम ।", expect: Rejects("Missing action verb: the query projects 1 column and needs 'आह्रियताम्'") },
    Case { name: "select_requires_verb_dual", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यम् च ।", expect: Rejects("needs 'आह्रियेताम्'") },
    Case { name: "statement_requires_purnaviraama", ssql: "लेखकेभ्यः नाम आह्रियताम्", expect: Rejects("the last statement must end with '।'") },
    Case { name: "select_all_is_singular", ssql: "पुस्तकेभ्यः सर्वम् आह्रियताम् ।", expect: Compiles("SELECT * FROM books t1") },
    Case { name: "select_all_rejects_plural", ssql: "पुस्तकेभ्यः सर्वम् आह्रियन्ताम् ।", expect: Rejects("Agreement") },
    // --- Comparisons (ablative standard of comparison, instrumental with सम) ---
    Case { name: "where_greater_than_ablative", ssql: "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.price > 20") },
    // --- BETWEEN: exactly two bounds ---
    Case { name: "between_two_bounds", ssql: "पुस्तकेभ्यः मूल्यम् २० च ५० च इत्यनयोः मध्ये अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.price BETWEEN 20 AND 50") },
    Case { name: "between_rejects_one_bound", ssql: "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् मध्ये अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    Case { name: "between_rejects_three_bounds", ssql: "पुस्तकेभ्यः मूल्यम् १० च २० च ३० च इत्यनयोः मध्ये अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    // --- A function has the gender of the noun naming it (खण्डः is masculine) ---
    Case { name: "substr_masculine_comparison", ssql: "पुस्तकेभ्यः शीर्षके १ स्थानात् आरभ्य ३ वर्णानाम् खण्डः \"क\" इत्यनेन समः अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE substr(t1.title, 1, 3) = 'क'") },
    // The pre-#56 postfix SUBSTR form is retired (final review).
    Case { name: "substr_postfix_form_retired", ssql: "पुस्तकेभ्यः शीर्षकस्य १ इत्यस्मात् ३ खण्डः आह्रियताम् ।", expect: Rejects("Parse errors") },
    Case { name: "substr_needs_a_start", ssql: "पुस्तकेभ्यः शीर्षकस्य खण्डः आह्रियताम् ।", expect: Rejects("needs 2 arguments but was given 1") },
    // --- Every value read once per group is grouped or aggregated: the group filter, CASE, a window, the order, a correlated subquery (second review B1) ---
    Case { name: "grouping_having_ungrouped", ssql: "पुस्तकेभ्यः भाषा इति समूहेन मूल्यम् १० इत्यस्मात् अधिकम् अस्ति इति समूहनियमेन भाषा आह्रियताम् ।", expect: Rejects("`मूल्यम्` is neither grouped") },
    Case { name: "grouping_case_ungrouped", ssql: "पुस्तकेभ्यः भाषा इति समूहेन भाषा च वरणम् यस्मिन् मूल्यम् १० इत्यस्मात् अधिकम् अस्ति तस्मिन् १ अन्यस्मिन् ० च आह्रियेताम् ।", expect: Rejects("`मूल्यम्` is neither grouped") },
    Case { name: "grouping_window_ungrouped", ssql: "पुस्तकेभ्यः भाषा इति समूहेन भाषा च श्रेणी यत्र मूल्यस्य अनुसारम् अवरोहक्रमेण च आह्रियेताम् ।", expect: Rejects("`मूल्यस्य` is neither grouped") },
    Case { name: "grouping_order_aggregate_beside_column", ssql: "पुस्तकेभ्यः मूल्यस्य योगः अवरोहः इति क्रमेण शीर्षकम् आह्रियताम् ।", expect: Rejects("`शीर्षकम्` is neither grouped") },
    Case { name: "grouping_having_without_grouping", ssql: "पुस्तकेभ्यः मूल्यस्य योगः १० इत्यस्मात् अधिकः अस्ति इति समूहनियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("`शीर्षकम्` is neither grouped") },
    Case { name: "grouping_correlated_having_subquery", ssql: "पुस्तकेभ्यः भाषा इति समूहेन यत् लेखकेभ्यः लेखकस्य सङ्ख्या पुस्तकस्य लेखकसङ्ख्या इत्यनया समा अस्ति इति नियमेन नाम आह्रियताम् इति विद्यते इति समूहनियमेन भाषा आह्रियताम् ।", expect: Rejects("`लेखकसङ्ख्या` is neither grouped") },
    Case { name: "grouping_having_on_aggregate", ssql: "पुस्तकेभ्यः भाषा इति समूहेन मूल्यस्य योगः १० इत्यस्मात् अधिकः अस्ति इति समूहनियमेन भाषा आह्रियताम् ।", expect: Compiles("GROUP BY t1.language HAVING SUM(t1.price) > 10") },
    // --- An aggregate is computed per group: never in the row filter, a grouping key, a join, or another aggregate (second review B2) ---
    Case { name: "aggregate_in_row_filter", ssql: "पुस्तकेभ्यः मूल्यस्य योगः १० इत्यस्मात् अधिकः अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("a condition on an aggregate is the group filter `… इति समूहनियमेन`") },
    Case { name: "aggregate_as_grouping_key", ssql: "पुस्तकेभ्यः मूल्यस्य योगः इति समूहेन पङ्क्तीनाम् गणना आह्रियताम् ।", expect: Rejects("an aggregate cannot be a grouping key") },
    Case { name: "aggregate_of_aggregate", ssql: "पुस्तकेभ्यः भाषा इति समूहेन यत् मूल्यस्य योगः इति इत्यस्य महत्तमम् आह्रियताम् ।", expect: Rejects("not of another aggregate") },
    Case { name: "aggregate_in_join", ssql: "पुस्तकेभ्यः लेखकैः सह मूल्यस्य योगः लेखकस्य सङ्ख्या इत्यनया समः अस्ति इति मेलनेन लेखकस्य नाम आह्रियताम् ।", expect: Rejects("an aggregate cannot be in a join condition") },
    // --- Distinct rows are ordered by what they hold ---
    Case { name: "distinct_order_unselected", ssql: "पुस्तकेभ्यः मूल्यम् अवरोहः इति क्रमेण पुनरुक्तिम् विना भाषा आह्रियताम् ।", expect: Rejects("`मूल्यम्` is not selected") },
    Case { name: "distinct_order_selected", ssql: "पुस्तकेभ्यः भाषा आरोहः इति क्रमेण पुनरुक्तिम् विना भाषा आह्रियताम् ।", expect: Compiles("SELECT DISTINCT t1.language FROM books t1 ORDER BY") },
    // --- Grouping and plain aggregates ---
    Case { name: "group_by_count", ssql: "पुस्तकेभ्यः भाषा इति समूहेन भाषा च सङ्ख्यायाः गणना च आह्रियेताम् ।", expect: Compiles("SELECT t1.language, COUNT(t1.id) FROM books t1 GROUP BY t1.language") },
    Case { name: "group_by_two_columns", ssql: "पुस्तकेभ्यः भाषा च विधा च इति समूहेन भाषा च विधा च आह्रियेताम् ।", expect: Compiles("GROUP BY t1.language, t1.genre") },
    // --- AVG is माध्यम्; the non-Sanskrit सरासरी / औसत are rejected with a hint ---
    Case { name: "avg_maadhyam", ssql: "पुस्तकेभ्यः भाषा इति समूहेन भाषा च मूल्यस्य माध्यम् च आह्रियेताम् ।", expect: Compiles("SELECT t1.language, AVG(t1.price) FROM books t1 GROUP BY t1.language") },
    Case { name: "avg_maadhyam_windowed", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यस्य माध्यम् यत्र भाषायाः आधारेण विभाजनम् भाषामाध्यम् इति आख्या च आह्रियेताम् ।", expect: Compiles("AVG(t1.price) OVER (PARTITION BY t1.language) AS भाषामाध्यम्") },
    Case { name: "avg_rejects_sarasari", ssql: "पुस्तकेभ्यः मूल्यस्य सरासरी आह्रियताम् ।", expect: Rejects("is not Sanskrit: `सरासरी` is Marathi/Hindi") }, // lexicon-allow
    Case { name: "avg_rejects_ausata", ssql: "पुस्तकेभ्यः मूल्यस्य औसतम् आह्रियताम् ।", expect: Rejects("Use `माध्यम्`") }, // lexicon-allow
    // --- Arithmetic grouping survives into SQL ---
    Case { name: "arith_group_right", ssql: "पुस्तकेभ्यः मूल्यम् च यत् २ च ५ च इत्यनयोः योगः इति च इत्यनयोः गुणनम् आह्रियताम् ।", expect: Compiles("SELECT t1.price * (2 + 5) FROM books t1") },
    Case { name: "arith_group_left", ssql: "पुस्तकेभ्यः यत् मूल्यम् च २ च इत्यनयोः योगः इति च ५ च इत्यनयोः गुणनम् आह्रियताम् ।", expect: Compiles("SELECT (t1.price + 2) * 5 FROM books t1") },
    Case { name: "arith_group_subtraction", ssql: "पुस्तकेभ्यः यत् मूल्यतः १० विशोध्य अवशिष्टम् इति च २ च इत्यनयोः गुणनम् आह्रियताम् ।", expect: Compiles("SELECT (t1.price - 10) * 2 FROM books t1") },
    Case { name: "arith_no_needless_parens", ssql: "पुस्तकेभ्यः यत् मूल्यम् च २ च इत्यनयोः गुणनम् इति च ५ च इत्यनयोः योगः आह्रियताम् ।", expect: Compiles("SELECT t1.price * 2 + 5 FROM books t1") },
    // --- Window functions: genitive + आधारेण / अनुसारम्, bare form, single इति ---
    Case { name: "window_rank_bare_genitive", ssql: "पुस्तकेभ्यः शीर्षकम् च श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण मूल्यश्रेणी इति आख्या च आह्रियेताम् ।", expect: Compiles("RANK() OVER (PARTITION BY t1.language ORDER BY t1.price DESC NULLS LAST) AS मूल्यश्रेणी") },
    Case { name: "window_sum_bare_genitive", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यस्य योगः यत्र भाषायाः आधारेण विभाजनम् भाषायोगः इति आख्या च आह्रियेताम् ।", expect: Compiles("SUM(t1.price) OVER (PARTITION BY t1.language) AS भाषायोगः") },
    Case { name: "window_count_is_count", ssql: "पुस्तकेभ्यः शीर्षकम् च सङ्ख्यायाः गणना यत्र भाषायाः आधारेण विभाजनम् भाषागणना इति आख्या च आह्रियेताम् ।", expect: Compiles("COUNT(t1.id) OVER (PARTITION BY t1.language) AS भाषागणना") },
    Case { name: "window_partition_list", ssql: "पुस्तकेभ्यः शीर्षकम् च श्रेणी यत्र भाषायाः च विधायाः च आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण मूल्यश्रेणी इति आख्या च आह्रियेताम् ।", expect: Compiles("RANK() OVER (PARTITION BY t1.language, t1.genre ORDER BY t1.price DESC NULLS LAST)") },
    Case { name: "window_order_list", ssql: "पुस्तकेभ्यः शीर्षकम् च श्रेणी यत्र मूल्यस्य च सङ्ख्यायाः च अनुसारम् अवरोहक्रमेण क्रमः इति आख्या च आह्रियेताम् ।", expect: Compiles("RANK() OVER (ORDER BY t1.price DESC NULLS LAST, t1.id DESC NULLS LAST) AS क्रमः") },
    Case { name: "window_function_needs_window", ssql: "पुस्तकेभ्यः शीर्षकम् च पूर्वपङ्क्तेः मूल्यम् पूर्वमूल्यम् इति आख्या च आह्रियेताम् ।", expect: Rejects("") },
    // --- Windows: genitive keys, explicit direction, LAG/LEAD and NTILE words (expert review F6, approved NTILE forms) ---
    Case { name: "window_alias_kramah_survives", ssql: "पुस्तकेभ्यः शीर्षकम् च श्रेणी यत्र मूल्यस्य अनुसारम् अवरोहक्रमेण क्रमः इति आख्या च आह्रियेताम् ।", expect: Compiles("RANK() OVER (ORDER BY t1.price DESC NULLS LAST) AS क्रमः") },
    Case { name: "window_key_must_be_genitive", ssql: "पुस्तकेभ्यः शीर्षकम् च श्रेणी यत्र भाषा आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण क्रमः इति आख्या च आह्रियेताम् ।", expect: Rejects("take the genitive") },
    Case { name: "window_order_needs_direction", ssql: "पुस्तकेभ्यः शीर्षकम् च श्रेणी यत्र मूल्यस्य अनुसारम् क्रमः इति आख्या च आह्रियेताम् ।", expect: Rejects("") },
    Case { name: "window_legacy_iti_kramah_rejected", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यस्य योगः यत्र मूल्यम् इति क्रमः योगफलम् इति आख्या च आह्रियेताम् ।", expect: Rejects("") },
    Case { name: "lag_is_purvapankteh", ssql: "पुस्तकेभ्यः शीर्षकम् च पूर्वपङ्क्तेः मूल्यम् यत्र मूल्यस्य अनुसारम् आरोहक्रमेण पूर्वमूल्यम् इति आख्या च आह्रियेताम् ।", expect: Compiles("LAG(t1.price) OVER (ORDER BY t1.price ASC NULLS LAST) AS पूर्वमूल्यम्") },
    Case { name: "lead_is_anantarapankteh", ssql: "पुस्तकेभ्यः शीर्षकम् च अनन्तरपङ्क्तेः मूल्यम् यत्र मूल्यस्य अनुसारम् आरोहक्रमेण अनन्तरमूल्यम् इति आख्या च आह्रियेताम् ।", expect: Compiles("LEAD(t1.price) OVER (ORDER BY t1.price ASC NULLS LAST) AS अनन्तरमूल्यम्") },
    Case { name: "legacy_pashchat_rejected", ssql: "पुस्तकेभ्यः शीर्षकम् च यत् मूल्यम् पश्चात् इति यत्र मूल्यस्य अनुसारम् आरोहक्रमेण पूर्वमूल्यम् इति आख्या च आह्रियेताम् ।", expect: Rejects("पूर्वपङ्क्तेः") },
    Case { name: "ntile_quartile", ssql: "पुस्तकेभ्यः शीर्षकम् च चतुर्थांशाङ्कः यत्र मूल्यस्य अनुसारम् आरोहक्रमेण चतुर्थांशः इति आख्या च आह्रियेताम् ।", expect: Compiles("NTILE(4) OVER (ORDER BY t1.price ASC NULLS LAST)") },
    Case { name: "ntile_n_parts", ssql: "पुस्तकेभ्यः शीर्षकम् च ५ भागेषु भागाङ्कः यत्र मूल्यस्य अनुसारम् आरोहक्रमेण भागः इति आख्या च आह्रियेताम् ।", expect: Compiles("NTILE(5) OVER (ORDER BY t1.price ASC NULLS LAST)") },
    Case { name: "ntile_two_parts_dual", ssql: "पुस्तकेभ्यः शीर्षकम् च २ भागयोः भागाङ्कः यत्र मूल्यस्य अनुसारम् आरोहक्रमेण भागः इति आख्या च आह्रियेताम् ।", expect: Compiles("NTILE(2) OVER") },
    Case { name: "ntile_two_parts_needs_dual", ssql: "पुस्तकेभ्यः शीर्षकम् च २ भागेषु भागाङ्कः यत्र मूल्यस्य अनुसारम् आरोहक्रमेण भागः इति आख्या च आह्रियेताम् ।", expect: Rejects("") },
    Case { name: "legacy_uttamachaturthamsha_rejected", ssql: "पुस्तकेभ्यः शीर्षकम् च उत्तमचतुर्थांशः यत्र मूल्यस्य अनुसारम् आरोहक्रमेण भागः इति आख्या च आह्रियेताम् ।", expect: Rejects("चतुर्थांशाङ्कः") },
    Case { name: "vibhaga_is_an_ordinary_word_again", ssql: "विभागाः इति पटलम् \"departments\" इति आख्या विभाग इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।", expect: Compiles("CREATE TABLE departments") },
    Case { name: "rank_needs_window", ssql: "पुस्तकेभ्यः शीर्षकम् च श्रेणी क्रमः इति आख्या च आह्रियेताम् ।", expect: Rejects("") },
    Case { name: "window_rejects_compound_partition", ssql: "पुस्तकेभ्यः शीर्षकम् च श्रेणी यत्र भाषाधारेण विभाजनम् मूल्यानुसारम् अवरोहक्रमेण मूल्यश्रेणी इति आख्या च आह्रियेताम् ।", expect: Rejects("") },
    // --- CASE (वरणम् यस्मिन् … तस्मिन् … अन्यस्मिन्) ---
    Case { name: "case_bare_single_iti", ssql: "पुस्तकेभ्यः शीर्षकम् च वरणम् यस्मिन् मूल्यम् २० इत्यस्मात् न्यूनम् अस्ति तस्मिन् \"Budget\" अन्यस्मिन् \"Premium\" वर्गः इति आख्या च आह्रियेताम् ।", expect: Compiles("CASE WHEN t1.price < 20 THEN 'Budget' ELSE 'Premium' END AS वर्गः") },
    Case { name: "case_then_next_column", ssql: "पुस्तकेभ्यः वरणम् यस्मिन् मूल्यम् २० इत्यस्मात् न्यूनम् अस्ति तस्मिन् १ अन्यस्मिन् ० च शीर्षकम् च आह्रियेताम् ।", expect: Compiles("SELECT CASE WHEN t1.price < 20 THEN 1 ELSE 0 END, t1.title FROM books t1") },
    Case { name: "case_grouped_then_aggregated", ssql: "पुस्तकेभ्यः यत् वरणम् यस्मिन् मूल्यम् २० इत्यस्मात् न्यूनम् अस्ति तस्मिन् १ अन्यस्मिन् ० इति इत्यस्य योगः आह्रियताम् ।", expect: Compiles("SELECT SUM(CASE WHEN t1.price < 20 THEN 1 ELSE 0 END) FROM books t1") },
    // --- Boolean grouping with यत् … इति ---
    Case { name: "boolean_group_yat_iti", ssql: "पुस्तकेभ्यः यत् भाषा \"संस्कृतम्\" इत्यनया समा अस्ति वा भाषा \"पालि\" इत्यनया समा अस्ति वा इति च मूल्यम् २० इत्यस्मात् अधिकम् अस्ति च इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE ((t1.language = 'संस्कृतम्' OR t1.language = 'पालि') AND t1.price > 20)") },
    // --- One meaning in every dialect ---
    Case { name: "real_type_is_double", ssql: "मापाः इति पटलम् \"measures\" इति आख्या माप इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र मूल्यम् इति वास्तविकः \"v\" इति आख्या मूल्य इति नपुंसकलिङ्गम् एकवचनम् इति पटलम् ।", expect: Compiles("v REAL") },
    Case { name: "order_by_nulls_last", ssql: "पुस्तकेभ्यः पुस्तकस्य मूल्यम् अवरोहः इति क्रमेण शीर्षकम् आह्रियताम् ।", expect: Compiles("ORDER BY t1.price DESC NULLS LAST") },
    Case { name: "weekday_number", ssql: "समीक्षाभ्यः समीक्षायाः समीक्षादिनाङ्कस्य वारः समीक्षावारः इति आख्या आह्रियताम् ।", expect: Compiles("CAST(strftime('%w', t1.review_date) AS INTEGER) AS समीक्षावारः") },
    // A date part is a number on both dialects: SQLite's strftime text
    // compared with a number was always the greater (`'2023' > 2099`).
    Case { name: "date_part_compares_as_number", ssql: "समीक्षाभ्यः समीक्षायाः समीक्षादिनाङ्कस्य वर्षम् २०९९ इत्यस्मात् अधिकम् अस्ति इति नियमेन मूल्याङ्कनम् आह्रियताम् ।", expect: Compiles("WHERE CAST(strftime('%Y', t1.review_date) AS INTEGER) > 2099") },
    Case { name: "mod_is_real_remainder", ssql: "पुस्तकेभ्यः मूल्यस्य २ इत्यनेन शेषः आह्रियताम् ।", expect: Compiles("mod(t1.price, 2)") },
    // --- Functions: built in, declared, or pure UDF - never an unknown word ---
    Case { name: "unknown_function_rejected", ssql: "लेखकेभ्यः लेखकस्य नाम्नः दीर्घता आह्रियताम् ।", expect: Rejects("Unknown function `दीर्घता`") },
    Case { name: "pure_udf_call", ssql: "द्विगुणितम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च मूल्यम् इति वास्तविकः दीयते मूल्यम् च २ च इत्यनयोः गुणनम् इति उत्तरम् प्रतिदीयताम् इति कार्यम् । पुस्तकेभ्यः शीर्षकम् च मूल्यस्य द्विगुणितम् द्विगुणितमूल्यम् इति आख्या च आह्रियेताम् ।", expect: Compiles("((t1.price) * 2) AS द्विगुणितमूल्यम्") },
    Case { name: "function_binding_is_retired", ssql: "घातः इति क्रिया \"POWER\" इति आख्या रच्यताम् । पुस्तकेभ्यः मूल्यस्य २ इत्यनेन घातः आह्रियताम् ।", expect: Rejects("a function is not bound to a SQL name") },
    Case { name: "builtin_function_by_its_own_name", ssql: "पुस्तकेभ्यः मूल्यस्य २ इत्यनेन घातः आह्रियताम् ।", expect: Compiles("power(t1.price, 2)") },
    // UPPER/LOWER name the letters (वर्ण); दीर्घ/लघु alone is vowel length.
    Case { name: "upper_names_the_letters", ssql: "पुस्तकेभ्यः शीर्षकस्य महावर्णरूपम् आह्रियताम् ।", expect: Compiles("upper(t1.title)") },
    Case { name: "lower_names_the_letters", ssql: "पुस्तकेभ्यः शीर्षकस्य लघुवर्णरूपम् आह्रियताम् ।", expect: Compiles("lower(t1.title)") },
    Case { name: "upper_old_name_retired", ssql: "पुस्तकेभ्यः शीर्षकस्य दीर्घरूपम् आह्रियताम् ।", expect: Rejects("the function is `महावर्णरूपम्`") },
    Case { name: "lower_old_name_retired", ssql: "पुस्तकेभ्यः शीर्षकस्य लघुरूपम् आह्रियताम् ।", expect: Rejects("the function is `लघुवर्णरूपम्`") },
    // --- Formerly playground-only, now core grammar ---
    Case { name: "invisible_format_chars", ssql: "\u{FEFF}पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।\u{200D}", expect: Compiles("SELECT t1.title FROM books t1") },
    Case { name: "comment_double_dash", ssql: "-- टिप्पणी\nपुस्तकेभ्यः शीर्षकम् आह्रियताम् ।", expect: Compiles("SELECT t1.title FROM books t1") },
    Case { name: "distinct_feminine", ssql: "पुस्तकेभ्यः विशिष्टायाः भाषायाः गणना आह्रियताम् ।", expect: Compiles("COUNT(DISTINCT t1.language)") },
    Case { name: "distinct_neuter", ssql: "पुस्तकेभ्यः विशिष्टस्य मूल्यस्य गणना आह्रियताम् ।", expect: Compiles("COUNT(DISTINCT t1.price)") },
    Case { name: "distinct_rejects_wrong_gender", ssql: "पुस्तकेभ्यः विशिष्टस्य भाषायाः गणना आह्रियताम् ।", expect: Rejects("Distinct modifier gender/number mismatch") },
    Case { name: "union_all", ssql: "ग्राहकेभ्यः नाम आह्रियताम् च लेखकेभ्यः नाम आह्रियताम् च इत्यनयोः सर्वसङ्गमः आह्रियताम् ।", expect: Compiles("SELECT t1.name FROM customers t1 UNION ALL SELECT t2.name FROM authors t2") },
    Case { name: "row_number", ssql: "पुस्तकेभ्यः शीर्षकम् च पङ्क्तिसङ्ख्या यत्र मूल्यस्य अनुसारम् अवरोहक्रमेण क्रमाङ्कः इति आख्या च आह्रियेताम् ।", expect: Compiles("ROW_NUMBER() OVER (ORDER BY t1.price DESC NULLS LAST) AS क्रमाङ्कः") },
    Case { name: "dense_rank", ssql: "पुस्तकेभ्यः शीर्षकम् च सघनश्रेणी यत्र मूल्यस्य अनुसारम् अवरोहक्रमेण सघनक्रमः इति आख्या च आह्रियेताम् ।", expect: Compiles("DENSE_RANK() OVER (ORDER BY t1.price DESC NULLS LAST) AS सघनक्रमः") },
    // --- EXISTS: यत् <query> इति [न] विद्यते ---
    Case { name: "not_exists_correlated", ssql: "पुस्तकेभ्यः यत् विक्रयणवस्तुभ्यः विक्रयणवस्तुनः पुस्तकसङ्ख्या पुस्तकस्य सङ्ख्या इत्यनया समा अस्ति इति नियमेन सर्वम् आह्रियताम् इति न विद्यते इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE NOT EXISTS (SELECT * FROM sale_items t2 WHERE t2.book_id = t1.id)") },
    Case { name: "exists_correlated", ssql: "पुस्तकेभ्यः यत् विक्रयणवस्तुभ्यः विक्रयणवस्तुनः पुस्तकसङ्ख्या पुस्तकस्य सङ्ख्या इत्यनया समा अस्ति इति नियमेन सर्वम् आह्रियताम् इति विद्यते इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE EXISTS (SELECT * FROM sale_items t2 WHERE t2.book_id = t1.id)") },
    Case { name: "exists_uncorrelated", ssql: "पुस्तकेभ्यः यत् आपणेभ्यः सर्वम् आह्रियताम् इति विद्यते इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE EXISTS (SELECT * FROM stores t2)") },
    Case { name: "exists_needs_subquery", ssql: "पुस्तकेभ्यः मूल्यम् विद्यते चेत् शीर्षकम् आह्रियताम् ।", expect: Rejects("Parse errors") },
    // --- Derived tables: यत् <query> इति आश्रित्य ---
    Case { name: "derived_table", ssql: "यत् पुस्तकेभ्यः शीर्षकम् च मूल्यम् च आह्रियेताम् इति आश्रित्य मूल्यम् २० इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WITH derived_1 AS ( SELECT t1.title, t1.price FROM books t1 ) SELECT t2.title FROM derived_1 t2 WHERE t2.price > 20;") },
    Case { name: "derived_table_grouped_alias", ssql: "यत् पुस्तकेभ्यः भाषा इति समूहेन भाषा च मूल्यस्य माध्यम् भाषामाध्यम् इति आख्या च आह्रियेताम् इति आश्रित्य भाषामाध्यम् २० इत्यस्मात् अधिकम् अस्ति इति नियमेन भाषा आह्रियताम् ।", expect: Compiles("SELECT t2.language FROM derived_1 t2 WHERE t2.भाषामाध्यम् > 20;") },
    Case { name: "derived_table_nested", ssql: "यत् यत् पुस्तकेभ्यः शीर्षकम् च मूल्यम् च आह्रियेताम् इति आश्रित्य शीर्षकम् च मूल्यम् च आह्रियेताम् इति आश्रित्य शीर्षकम् आह्रियताम् ।", expect: Compiles("derived_2 AS ( SELECT t2.title, t2.price FROM derived_1 t2 ) SELECT t3.title FROM derived_2 t3;") },
    // --- यत् … इति subqueries may contain their own इति ---
    Case { name: "subquery_with_inner_where", ssql: "पुस्तकेभ्यः यत् आपणेभ्यः नाम \"x\" इत्यनेन समम् अस्ति इति नियमेन सर्वम् आह्रियताम् इति विद्यते इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE EXISTS (SELECT * FROM stores t2 WHERE t2.name = 'x')") },
    Case { name: "subquery_with_inner_group_by", ssql: "पुस्तकेभ्यः मूल्यम् यत् पुस्तकेभ्यः भाषा इति समूहेन मूल्यस्य माध्यम् आह्रियताम् इति इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("t1.price > (SELECT AVG(t2.price) FROM books t2 GROUP BY t2.language)") },
    // A plural ablative is never a genitive, so it cannot qualify a column.
    Case { name: "plural_ablative_is_not_a_qualifier", ssql: "पुस्तकेभ्यः आपणेभ्यः नाम आह्रियताम् ।", expect: Rejects("") },
    // --- DELETE: filtered, or explicitly everything ---
    Case { name: "delete_filtered", ssql: "ग्राहकेभ्यः सङ्ख्या ५ इत्यस्याः अधिका अस्ति इति नियमेन निष्कास्यताम् ।", expect: Compiles("DELETE FROM customers WHERE id > 5;") },
    Case { name: "delete_everything_explicit", ssql: "ग्राहकेभ्यः सर्वम् निष्कास्यताम् ।", expect: Compiles("DELETE FROM customers;") },
    Case { name: "delete_needs_filter_or_sarvam", ssql: "ग्राहकेभ्यः निष्कास्यताम् ।", expect: Rejects("Parse errors") },
    // --- One list, one connective: mixing च and वा needs a यत् … इति group ---
    Case { name: "mixed_connectives_rejected", ssql: "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् अस्ति च भाषा \"संस्कृतम्\" इत्यनया समा अस्ति वा भाषा \"पालि\" इत्यनया समा अस्ति वा इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("Parse errors") },
    Case { name: "grouped_or_inside_and", ssql: "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् अस्ति च यत् भाषा \"संस्कृतम्\" इत्यनया समा अस्ति वा भाषा \"पालि\" इत्यनया समा अस्ति वा इति च इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE (t1.price > 20 AND (t1.language = 'संस्कृतम्' OR t1.language = 'पालि'))") },
    // A group around a one-connective list is redundant (expert review Q7);
    // around a select list it used to compile to `title AND price`.
    Case { name: "redundant_group_select_list", ssql: "पुस्तकेभ्यः यत् शीर्षकम् च मूल्यम् च इति आह्रियताम् ।", expect: Rejects("group around a च-list is needed only as an item of a वा-list") },
    Case { name: "redundant_group_whole_filter", ssql: "पुस्तकेभ्यः यत् मूल्यम् २० इत्यस्मात् अधिकम् अस्ति वा भाषा \"पालि\" इत्यनया समा अस्ति वा इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("group around a वा-list is needed only as an item of a च-list") },
    // A group that changes nothing is an error (approved decision
    // "Grouping"); one around an embedded statement is always kept.
    Case { name: "redundant_group_single_column", ssql: "पुस्तकेभ्यः यत् मूल्यम् इति आह्रियताम् ।", expect: Rejects("group changes nothing") },
    Case { name: "redundant_group_sum_item", ssql: "पुस्तकेभ्यः यत् मूल्यम् च २ च इत्यनयोः योगः इति आह्रियताम् ।", expect: Rejects("group changes nothing") },
    Case { name: "redundant_group_product_aggregated", ssql: "पुस्तकेभ्यः यत् मूल्यम् च २ च इत्यनयोः गुणनम् इति इत्यस्य योगः आह्रियताम् ।", expect: Rejects("group changes nothing") },
    Case { name: "redundant_group_having", ssql: "पुस्तकेभ्यः भाषा इति समूहेन यत् मूल्यस्य योगः १० इत्यस्मात् अधिकः अस्ति इति इति समूहनियमेन भाषा आह्रियताम् ।", expect: Rejects("group changes nothing") },
    Case { name: "redundant_group_limit", ssql: "पुस्तकेभ्यः यत् ५ इति इति मित्या शीर्षकम् आह्रियताम् ।", expect: Rejects("group changes nothing") },
    Case { name: "needed_group_sum_operand", ssql: "पुस्तकेभ्यः प्रकाशनवर्षम् च यत् मूल्यम् च २ च ३ च इत्येषाम् योगः इति च आह्रियेताम् ।", expect: Compiles("SELECT t1.publication_year, t1.price + 2 + 3 FROM books t1") },
    // A second group around a subquery that has its own turned INSERT…SELECT
    // into VALUES ((SELECT …)).
    Case { name: "doubled_group_around_subquery", ssql: "प्रकाशकेषु नाम यत् यत् लेखकेभ्यः नाम आह्रियताम् इति इति निवेश्यताम् ।", expect: Rejects("already has its `यत् … इति`") },
    // DISTINCT is said of an aggregate's values; elsewhere it was dropped.
    Case { name: "distinct_aggregate_kept", ssql: "पुस्तकेभ्यः विशिष्टायाः भाषायाः गणना आह्रियताम् ।", expect: Compiles("COUNT(DISTINCT t1.language)") },
    Case { name: "distinct_window_rejected", ssql: "पुस्तकेभ्यः शीर्षकम् च विशिष्टायाः भाषायाः गणना यत्र विधायाः आधारेण विभाजनम् भाषागणना इति आख्या च आह्रियेताम् ।", expect: Rejects("not of a windowed aggregate") },
    Case { name: "distinct_row_count_rejected", ssql: "पुस्तकेभ्यः विशिष्टानाम् पङ्क्तीनाम् गणना आह्रियताम् ।", expect: Rejects("`विशिष्ट-` (DISTINCT) is said of an aggregate's values") },
    Case { name: "distinct_function_rejected", ssql: "पुस्तकेभ्यः विशिष्टायाः भाषायाः दैर्घ्यम् आह्रियताम् ।", expect: Rejects("not of a function or date part") },
    // Built-ins said of a column, by their inflected names.
    Case { name: "trim_of_column", ssql: "पुस्तकेभ्यः शीर्षकस्य मार्जनम् आह्रियताम् ।", expect: Compiles("SELECT trim(t1.title) FROM books t1") },
    Case { name: "random_alone", ssql: "पुस्तकेभ्यः यादृच्छिकः आह्रियताम् ।", expect: Compiles("random()") },
    // Pronoun number: an IN list by its count of values, arithmetic by its operands.
    Case { name: "in_list_three_values_dual_rejected", ssql: "पुस्तकेभ्यः मूल्यम् १ च २ च ३ च इत्यनयोः अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("the list has 3 values, so its pronoun is the locative `इत्येषु`") },
    Case { name: "two_operands_plural_rejected", ssql: "पुस्तकेभ्यः मूल्यम् च ५० च इत्येषाम् योगः आह्रियताम् ।", expect: Rejects("two operands take the dual pronoun `इत्यनयोः`") },
    // The object of a gerund is accusative (a masculine column shows it).
    Case { name: "division_object_accusative", ssql: "वस्तुसूचीभ्यः भण्डारम् २ विभज्य लब्धम् आह्रियताम् ।", expect: Compiles("SELECT t1.quantity / 2 FROM inventory t1") },
    Case { name: "division_object_nominative_rejected", ssql: "वस्तुसूचीभ्यः भण्डारः २ विभज्य लब्धम् आह्रियताम् ।", expect: Rejects("write `भण्डारम्`, not `भण्डारः`") },
    // A date part has its noun's gender: मासः is masculine.
    Case { name: "date_part_agreement", ssql: "समीक्षाभ्यः समीक्षायाः समीक्षादिनाङ्कस्य मासः ९ इत्यनेन समम् अस्ति इति नियमेन मूल्याङ्कनम् आह्रियताम् ।", expect: Rejects("Grammar error") },
    // SQL no database accepts is a compile error (final review, parser).
    Case { name: "window_in_filter_rejected", ssql: "पुस्तकेभ्यः श्रेणी यत्र मूल्यस्य अनुसारम् अवरोहक्रमेण ३ इत्यस्याः न्यूना अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("cannot be used in them") },
    Case { name: "in_subquery_two_columns_rejected", ssql: "पुस्तकेभ्यः सङ्ख्या यत् विक्रयणवस्तुभ्यः पुस्तकसङ्ख्या च परिमाणम् च आह्रियेताम् इति इत्यासु अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("returns one column; this one returns 2") },
    Case { name: "update_same_column_twice_rejected", ssql: "पुस्तकेषु सर्वत्र मूल्यम् १ इति च मूल्यम् २ इति च परिवर्त्येताम् ।", expect: Rejects("`मूल्यम्` is changed twice") },
    Case { name: "insert_same_column_twice_rejected", ssql: "पुस्तकेषु शीर्षकम् \"क\" च शीर्षकम् \"ख\" च निवेश्येताम् ।", expect: Rejects("`शीर्षकम्` is inserted twice") },
    Case { name: "limit_fraction_rejected", ssql: "पुस्तकेभ्यः २.५ इति मित्या शीर्षकम् आह्रियताम् ।", expect: Rejects("takes a whole number of rows") },
    Case { name: "set_op_branch_limit_rejected", ssql: "ग्राहकेभ्यः नाम आरोहः इति क्रमेण २ इति मित्या नाम आह्रियताम् च लेखकेभ्यः नाम आह्रियताम् च इत्यनयोः सङ्गमः आह्रियताम् ।", expect: Rejects("cannot be ordered or limited on its own") },
    Case { name: "frame_reversed_rejected", ssql: "विक्रयणेभ्यः सकलमूल्यम् च सकलमूल्यस्य योगः यत्र विक्रयणदिनाङ्कस्य अनुसारम् आरोहक्रमेण पङ्क्तयः २ अनुवर्तितः आरभ्य वर्तमानपङ्क्तिपर्यन्तम् चलयोगः इति आख्या च आह्रियेताम् ।", expect: Rejects("starts after it ends") },
    Case { name: "cte_belongs_to_one_statement", ssql: "सुलभाः इति आख्या सुलभ इति पुंलिङ्गम् बहुवचनम् पुस्तकेभ्यः शीर्षकम् आह्रियताम् । सुलभेभ्यः शीर्षकम् आह्रियताम् । सुलभेभ्यः पङ्क्तीनाम् गणना आह्रियताम् ।", expect: Rejects("declare it again here") },
    Case { name: "two_primary_keys_rejected", ssql: "परीक्षाः इति पटलम् \"exams\" इति आख्या परीक्षा इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च प्राथमिकः अङ्कः इति पूर्णाङ्कः \"k\" इति आख्या अङ्क इति पुंलिङ्गम् एकवचनम् च इति पटलम् ।", expect: Rejects("a table has one primary key") },
    Case { name: "ungrouped_column_rejected", ssql: "पुस्तकेभ्यः भाषा इति समूहेन शीर्षकम् च मूल्यस्य योगः च आह्रियेताम् ।", expect: Rejects("`शीर्षकम्` is neither grouped") },
    Case { name: "grouped_column_kept", ssql: "पुस्तकेभ्यः भाषा इति समूहेन भाषा च मूल्यस्य योगः च आह्रियेताम् ।", expect: Compiles("SELECT t1.language, SUM(t1.price) FROM books t1 GROUP BY t1.language") },
    // A compound operand of a subtraction or division is named by its result
    // noun in the slot's case (expert review, grouped operands).
    Case { name: "sum_as_minuend", ssql: "पुस्तकेभ्यः मूल्यम् च २ च इत्यनयोः योगात् १ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("SELECT t1.price + 2 - 1 FROM books t1") },
    Case { name: "sum_as_subtrahend", ssql: "पुस्तकेभ्यः मूल्यात् मूल्यम् च २ च इत्यनयोः योगम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("SELECT t1.price - (t1.price + 2) FROM books t1") },
    Case { name: "subtrahend_list", ssql: "पुस्तकेभ्यः मूल्यात् प्रकाशनवर्षम् च १ च विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("SELECT t1.price - t1.publication_year - 1 FROM books t1") },
    Case { name: "subtraction_chain", ssql: "पुस्तकेभ्यः मूल्यात् प्रकाशनवर्षम् विशोध्य अवशिष्टात् १ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("SELECT t1.price - t1.publication_year - 1 FROM books t1") },
    Case { name: "sum_as_dividend", ssql: "पुस्तकेभ्यः मूल्यम् च प्रकाशनवर्षम् च इत्यनयोः योगम् २ विभज्य लब्धम् आह्रियताम् ।", expect: Compiles("SELECT (t1.price + t1.publication_year) / 2 FROM books t1") },
    Case { name: "sum_as_divisor", ssql: "पुस्तकेभ्यः मूल्यम् प्रकाशनवर्षम् च १ च इत्यनयोः योगेन विभज्य लब्धम् आह्रियताम् ।", expect: Compiles("SELECT t1.price / (t1.publication_year + 1) FROM books t1") },
    Case { name: "remainder_as_dividend", ssql: "पुस्तकेभ्यः मूल्यात् ५ विशोध्य अवशिष्टम् २ विभज्य लब्धम् आह्रियताम् ।", expect: Compiles("SELECT (t1.price - 5) / 2 FROM books t1") },
    Case { name: "division_chain", ssql: "पुस्तकेभ्यः मूल्यम् २ विभज्य लब्धम् ३ विभज्य लब्धम् आह्रियताम् ।", expect: Compiles("SELECT t1.price / 2 / 3 FROM books t1") },
    Case { name: "subtrahend_list_needs_last_cha", ssql: "पुस्तकेभ्यः मूल्यात् प्रकाशनवर्षम् च १ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("add `च` after `१`") },
    // A column is read in its declared (singular) number; the genitive
    // plural is its aggregate's argument (expert review, final).
    Case { name: "aggregate_of_genitive_plural", ssql: "पुस्तकेभ्यः मूल्यानाम् योगः आह्रियताम् ।", expect: Compiles("SELECT SUM(t1.price) FROM books t1") },
    Case { name: "column_in_plural_rejected", ssql: "पुस्तकेभ्यः मूल्यानि आह्रियताम् ।", expect: Rejects("मूल्यानि") },
    Case { name: "plural_column_declaration_rejected", ssql: "लेखाः इति पटलम् \"posts\" इति आख्या लेख इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च दृष्टयः इति पूर्णाङ्कः \"views\" इति आख्या दृष्टि इति स्त्रीलिङ्गम् बहुवचनम् च इति पटलम् ।", expect: Rejects("is declared plural") },
    // A view's columns are its query's, so a later statement reads them.
    Case { name: "view_is_queryable", ssql: "भाषागणनाः इति दृश्यम् \"language_counts\" इति आख्या भाषागणना इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत् पुस्तकेभ्यः भाषा इति समूहेन भाषा च पङ्क्तीनाम् गणना पुस्तकगणना इति आख्या च आह्रियेताम् । इति दृश्यम् । भाषागणनाभ्यः पुस्तकगणना आह्रियताम् ।", expect: Compiles("SELECT t2.पुस्तकगणना FROM language_counts t2") },
    // UPDATE empties a column with the adjective that agrees with it.
    Case { name: "update_sets_null", ssql: "पुस्तकेषु सङ्ख्या १ इत्यनया समा अस्ति इति नियमेन विधा रिक्ता इति परिवर्त्यताम् ।", expect: Compiles("UPDATE books SET genre = NULL WHERE id = 1;") },
    Case { name: "update_null_agreement", ssql: "पुस्तकेषु सर्वत्र विधा रिक्तम् इति परिवर्त्यताम् ।", expect: Rejects("the emptied column `विधा` takes `रिक्ता`") },
    // A required or key column always holds a value; an UPDATE or DELETE reads only its own table (second review B3, B5).
    // A CTE's alias column agrees in its own gender (`पुस्तकगणना` feminine), not always neuter (second review B4).
    Case { name: "cte_alias_feminine_agrees", ssql: "सुलभाः इति आख्या सुलभ इति पुंलिङ्गम् बहुवचनम् पुस्तकेभ्यः भाषा इति समूहेन भाषा च पङ्क्तीनाम् गणना पुस्तकगणना इति आख्या च आह्रियेताम् । सुलभेभ्यः पुस्तकगणना २ इत्यस्याः अधिका अस्ति इति नियमेन भाषा आह्रियताम् ।", expect: Compiles("WHERE t2.पुस्तकगणना > 2") },
    Case { name: "cte_alias_feminine_rejects_neuter", ssql: "सुलभाः इति आख्या सुलभ इति पुंलिङ्गम् बहुवचनम् पुस्तकेभ्यः भाषा इति समूहेन भाषा च पङ्क्तीनाम् गणना पुस्तकगणना इति आख्या च आह्रियेताम् । सुलभेभ्यः पुस्तकगणना २ इत्यस्मात् अधिकम् अस्ति इति नियमेन भाषा आह्रियताम् ।", expect: Rejects("write `अधिका`") },
    Case { name: "update_null_required_column", ssql: "पुस्तकेषु सर्वत्र शीर्षकम् रिक्तम् इति परिवर्त्यताम् ।", expect: Rejects("`शीर्षकम्` is declared required") },
    Case { name: "update_null_key_column", ssql: "पुस्तकेषु सर्वत्र सङ्ख्या रिक्ता इति परिवर्त्यताम् ।", expect: Rejects("`सङ्ख्या` is declared required") },
    Case { name: "update_value_of_other_table", ssql: "पुस्तकेषु सर्वत्र मूल्यम् भण्डारः इति परिवर्त्यताम् ।", expect: Rejects("`भण्डारः` is not a column of any table this statement reads (पुस्तक)") },
    Case { name: "update_filter_on_other_table", ssql: "पुस्तकेषु भण्डारः ५ इत्यस्मात् अधिकः अस्ति इति नियमेन मूल्यम् ५ इति परिवर्त्यताम् ।", expect: Rejects("`भण्डारः` is not a column of any table this statement reads") },
    Case { name: "delete_filter_on_other_table", ssql: "पुस्तकेभ्यः भण्डारः ५ इत्यस्मात् अधिकः अस्ति इति नियमेन निष्कास्यताम् ।", expect: Rejects("`भण्डारः` is not a column of any table this statement reads") },
    // A CTE's columns are those it selects; another table's is an error.
    Case { name: "cte_unselected_column_rejected", ssql: "सुलभाः इति आख्या सुलभ इति पुंलिङ्गम् बहुवचनम् पुस्तकेभ्यः शीर्षकम् आह्रियताम् । सुलभेभ्यः मूल्यम् आह्रियताम् ।", expect: Rejects("`मूल्यम्` is not a column of any table this statement reads") },
    Case { name: "cte_qualified_unselected_column_rejected", ssql: "सुलभाः इति आख्या सुलभ इति पुंलिङ्गम् बहुवचनम् पुस्तकेभ्यः शीर्षकम् आह्रियताम् । सुलभेभ्यः सुलभस्य मूल्यम् आह्रियताम् ।", expect: Rejects("`मूल्यम्` is not a column of `सुलभ`") },
    Case { name: "quotient_as_minuend", ssql: "पुस्तकेभ्यः मूल्यम् २ विभज्य लब्धात् ५ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("SELECT t1.price / 2 - 5 FROM books t1") },
    Case { name: "remainder_as_subtrahend", ssql: "पुस्तकेभ्यः मूल्यात् यत् प्रकाशनवर्षात् १ विशोध्य अवशिष्टम् इति इतीदम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("SELECT t1.price - (t1.publication_year - 1) FROM books t1") },
    // `इतीमम्` and `इतीमाम्` are gone: what has an accusative of its own takes it,
    // so only `इतीदम्` remains, and a group around anything else is redundant.
    Case { name: "grouped_subtrahend_only_itidam", ssql: "पुस्तकेभ्यः मूल्यात् यत् प्रकाशनवर्षात् १ विशोध्य अवशिष्टम् इति इतीमम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("`इतीमम्`") },
    Case { name: "grouped_column_subtrahend_redundant", ssql: "पुस्तकेभ्यः मूल्यात् यत् प्रकाशनवर्षम् इति इतीदम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("group is redundant") },
    Case { name: "grouped_number_subtrahend_redundant", ssql: "पुस्तकेभ्यः मूल्यात् यत् १ इति इतीदम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("group is redundant") },
    Case { name: "grouped_sum_subtrahend_redundant", ssql: "पुस्तकेभ्यः मूल्यात् यत् मूल्यम् च २ च इत्यनयोः योगः इति इतीदम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("`योगम्`") },
    Case { name: "grouped_product_subtrahend_redundant", ssql: "पुस्तकेभ्यः मूल्यात् यत् मूल्यम् च २ च इत्यनयोः गुणनम् इति इतीदम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("`गुणनम्`") },
    Case { name: "grouped_quotient_subtrahend_redundant", ssql: "पुस्तकेभ्यः मूल्यात् यत् मूल्यम् २ विभज्य लब्धम् इति इतीदम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("`लब्धम्`") },
    Case { name: "grouped_aggregate_subtrahend_redundant", ssql: "पुस्तकेभ्यः मूल्यात् यत् मूल्यस्य योगः इति इतीदम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("`मूल्यस्य योगम्`") },
    Case { name: "subquery_as_subtrahend", ssql: "पुस्तकेभ्यः मूल्यात् यत् पुस्तकेभ्यः मूल्यस्य माध्यम् आह्रियताम् इति इतीदम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("SELECT t1.price - (") },
    // The ablative singular is spelled like the genitive outside the a-stems.
    Case { name: "feminine_ablative_minuend", ssql: "पुस्तकेभ्यः सङ्ख्यायाः ५ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("SELECT t1.id - 5 FROM books t1") },
    Case { name: "qualified_feminine_ablative_minuend", ssql: "पुस्तकेभ्यः पुस्तकस्य सङ्ख्यायाः ५ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("SELECT t1.id - 5 FROM books t1") },
    Case { name: "a_stem_genitive_is_no_minuend", ssql: "पुस्तकेभ्यः मूल्यस्य ५ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("") },
    // A result noun has its genitive, as the operand of an aggregate or function.
    Case { name: "remainder_genitive_of_aggregate", ssql: "पुस्तकेभ्यः मूल्यात् १ विशोध्य अवशिष्टस्य योगः आह्रियताम् ।", expect: Compiles("SELECT SUM(t1.price - 1) FROM books t1") },
    Case { name: "sum_genitive_of_aggregate", ssql: "पुस्तकेभ्यः मूल्यम् च २ च इत्यनयोः योगस्य माध्यम् आह्रियताम् ।", expect: Compiles("AVG(t1.price + 2)") },
    Case { name: "product_genitive_of_aggregate", ssql: "पुस्तकेभ्यः मूल्यम् च २ च इत्यनयोः गुणनस्य योगः आह्रियताम् ।", expect: Compiles("SUM(t1.price * 2)") },
    Case { name: "quotient_genitive_of_aggregate", ssql: "पुस्तकेभ्यः मूल्यम् २ विभज्य लब्धस्य योगः आह्रियताम् ।", expect: Compiles("SUM(t1.price / 2") },
    Case { name: "quotient_genitive_of_function", ssql: "पुस्तकेभ्यः मूल्यम् २ विभज्य लब्धस्य निरपेक्षमानम् आह्रियताम् ।", expect: Compiles("abs(t1.price / 2") },
    Case { name: "genitive_result_takes_no_pronoun", ssql: "पुस्तकेभ्यः मूल्यात् १ विशोध्य अवशिष्टस्य इत्यस्य योगः आह्रियताम् ।", expect: Rejects("") },
    Case { name: "nominative_result_keeps_pronoun", ssql: "पुस्तकेभ्यः मूल्यात् १ विशोध्य अवशिष्टम् इत्यस्य योगः आह्रियताम् ।", expect: Compiles("SELECT SUM(t1.price - 1) FROM books t1") },
    // An aggregate is an operand by its noun's case, with or without a window.
    Case { name: "aggregates_as_minuend_and_subtrahend", ssql: "पुस्तकेभ्यः भाषा इति समूहेन भाषा च मूल्यस्य महत्तमात् मूल्यस्य लघुतमम् विशोध्य अवशिष्टम् च आह्रियेताम् ।", expect: Compiles("MAX(t1.price) - MIN(t1.price)") },
    Case { name: "aggregates_as_dividend_and_divisor", ssql: "पुस्तकेभ्यः मूल्यस्य योगम् पङ्क्तीनाम् गणनया विभज्य लब्धम् आह्रियताम् ।", expect: Compiles("SUM(t1.price) / COUNT(*)") },
    Case { name: "count_distinct_as_divisor", ssql: "पुस्तकेभ्यः मूल्यस्य योगम् विशिष्टायाः भाषायाः गणनया विभज्य लब्धम् आह्रियताम् ।", expect: Compiles("COUNT(DISTINCT t1.language)") },
    Case { name: "count_as_minuend", ssql: "पुस्तकेभ्यः पङ्क्तीनाम् गणनायाः १ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("COUNT(*) - 1") },
    Case { name: "windowed_aggregate_as_subtrahend", ssql: "पुस्तकेभ्यः मूल्यात् मूल्यस्य माध्यम् यत्र भाषायाः आधारेण विभाजनम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("t1.price - AVG(t1.price) OVER (PARTITION BY t1.language)") },
    Case { name: "aggregate_subtrahend_in_a_filter_rejected", ssql: "पुस्तकेभ्यः मूल्यात् मूल्यस्य माध्यम् विशोध्य अवशिष्टम् १ इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    Case { name: "quotient_as_divisor", ssql: "पुस्तकेभ्यः मूल्यम् प्रकाशनवर्षम् २ विभज्य लब्धेन विभज्य लब्धम् आह्रियताम् ।", expect: Compiles("t1.price / (t1.publication_year / 2") },
    // A function's further operand is an instrumental agreeing with the first.
    Case { name: "function_operand_pronoun_gender", ssql: "पुस्तकेभ्यः मूल्यस्य २ इत्यनया पूर्णलब्धिः आह्रियताम् ।", expect: Rejects("write `इत्यनेन`, not `इत्यनया`") },
    Case { name: "function_operand_feminine", ssql: "पुस्तकेभ्यः सङ्ख्यायाः ३ इत्यनया शेषः आह्रियताम् ।", expect: Compiles("mod(t1.id, 3)") },
    Case { name: "function_operand_feminine_needs_inaya", ssql: "पुस्तकेभ्यः सङ्ख्यायाः ३ इत्यनेन शेषः आह्रियताम् ।", expect: Rejects("write `इत्यनया`, not `इत्यनेन`") },
    Case { name: "function_operand_not_genitive", ssql: "पुस्तकेभ्यः सङ्ख्यायाः ३ इत्यस्य शेषः आह्रियताम् ।", expect: Rejects("further operand is in the instrumental") },
    Case { name: "function_operand_needs_a_case", ssql: "पुस्तकेभ्यः मूल्यस्य २ शेषः आह्रियताम् ।", expect: Rejects("further operand is in the instrumental") },
    Case { name: "function_operand_column_instrumental", ssql: "पुस्तकेभ्यः मूल्यस्य सङ्ख्यया शेषः आह्रियताम् ।", expect: Compiles("mod(t1.price, t1.id)") },
    Case { name: "replace_takes_a_genitive", ssql: "पुस्तकेभ्यः शीर्षकस्य \"a\" इत्यनेन \"o\" इत्यनेन प्रतिस्थापनम् आह्रियताम् ।", expect: Rejects("is in the genitive") },
    // Mistakes named in plain words, not by some alternative's expectation.
    Case { name: "says_remainder_is_grouped", ssql: "पुस्तकेभ्यः मूल्यात् प्रकाशनवर्षात् १ विशोध्य अवशिष्टम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("a remainder as a subtrahend is grouped") },
    Case { name: "says_compound_operand_is_grouped", ssql: "पुस्तकेभ्यः मूल्यम् च २ च इत्यनयोः योगः च ३ च इत्यनयोः गुणनम् आह्रियताम् ।", expect: Rejects("as an operand of a sum or product is grouped") },
    Case { name: "says_aggregate_operand_is_grouped", ssql: "पुस्तकेभ्यः मूल्यस्य योगः च २ च इत्यनयोः योगः आह्रियताम् ।", expect: Rejects("as an operand of a sum or product is grouped") },
    Case { name: "says_case_branch_arithmetic_is_grouped", ssql: "पुस्तकेभ्यः वरणम् यस्मिन् मूल्यम् २० इत्यस्मात् अधिकम् अस्ति तस्मिन् मूल्यात् ५ विशोध्य अवशिष्टम् अन्यस्मिन् मूल्यम् आह्रियताम् ।", expect: Rejects("arithmetic there is grouped") },
    Case { name: "says_case_division_is_grouped", ssql: "पुस्तकेभ्यः वरणम् यस्मिन् मूल्यम् २० इत्यस्मात् अधिकम् अस्ति तस्मिन् मूल्यम् २ विभज्य लब्धम् अन्यस्मिन् मूल्यम् आह्रियताम् ।", expect: Rejects("arithmetic there is grouped") },
    Case { name: "says_case_condition_mixes_cha_va", ssql: "पुस्तकेभ्यः वरणम् यस्मिन् मूल्यम् २० इत्यस्मात् अधिकम् अस्ति वा भाषा \"Sanskrit\" इत्यनया समा अस्ति वा च प्रकाशनवर्षम् २००० इत्यस्मात् न्यूनम् अस्ति च तस्मिन् १ अन्यस्मिन् ० आह्रियताम् ।", expect: Rejects("mixes 'च' (and) with 'वा' (or)") },
    Case { name: "says_distinct_goes_first", ssql: "पुस्तकेभ्यः भाषा पुनरुक्तिम् विना आह्रियताम् ।", expect: Rejects("before the first selected item") },
    Case { name: "says_distinct_goes_first_in_a_list", ssql: "पुस्तकेभ्यः भाषा च पुनरुक्तिम् विना विधा च आह्रियेताम् ।", expect: Rejects("before the first selected item") },
    Case { name: "says_divisor_is_instrumental", ssql: "पुस्तकेभ्यः मूल्यम् सङ्ख्या विभज्य लब्धम् आह्रियताम् ।", expect: Rejects("what divides is in the instrumental") },
    Case { name: "says_minuend_is_ablative", ssql: "पुस्तकेभ्यः मूल्यम् ५ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("what is subtracted from is in the ablative") },
    Case { name: "says_genitive_is_no_minuend", ssql: "पुस्तकेभ्यः मूल्यस्य ५ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("what is subtracted from is in the ablative") },
    Case { name: "says_itidam_follows_a_group", ssql: "पुस्तकेभ्यः मूल्यात् प्रकाशनवर्षात् १ विशोध्य अवशिष्टम् इतीदम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("`इतीदम्` follows a group") },
    Case { name: "says_compared_column_takes_the_pronoun", ssql: "पुस्तकेभ्यः पुस्तकस्य प्रकाशनवर्षेण समम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    Case { name: "subtrahend_list_with_a_sum", ssql: "पुस्तकेभ्यः मूल्यात् प्रकाशनवर्षम् च मूल्यम् च २ च इत्यनयोः योगम् च विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("t1.price - t1.publication_year - (t1.price + 2)") },
    // A function of several operands over an aggregate or arithmetic result.
    Case { name: "round_of_an_average", ssql: "पुस्तकेभ्यः मूल्यस्य माध्यस्य २ इत्यनेन सन्निकटनम् आह्रियताम् ।", expect: Compiles("round(AVG(t1.price), 2)") },
    Case { name: "round_of_a_quotient", ssql: "पुस्तकेभ्यः मूल्यम् ३ विभज्य लब्धस्य २ इत्यनेन सन्निकटनम् आह्रियताम् ।", expect: Compiles("round(t1.price / 3") },
    Case { name: "window_key_of_an_aggregate_unaffected", ssql: "पुस्तकेभ्यः भाषा इति समूहेन भाषा च श्रेणी यत्र मूल्यस्य योगस्य अनुसारम् अवरोहक्रमेण च आह्रियेताम् ।", expect: Compiles("RANK() OVER (ORDER BY SUM(t1.price) DESC") },
    // A date is no number.
    Case { name: "date_arithmetic_rejected", ssql: "विक्रयणेभ्यः विक्रयणदिनाङ्कः च ७ च इत्यनयोः योगः आह्रियताम् ।", expect: Rejects("arithmetic on a date") },
    Case { name: "sum_of_text_rejected", ssql: "पुस्तकेभ्यः शीर्षकस्य योगः आह्रियताम् ।", expect: Rejects("`योगः` is of numbers, and this column holds text") },
    Case { name: "average_of_date_rejected", ssql: "विक्रयणेभ्यः विक्रयणदिनाङ्कस्य माध्यम् आह्रियताम् ।", expect: Rejects("`माध्यम्` is of numbers, and this column holds a date") },
    Case { name: "maximum_of_text_compiles", ssql: "पुस्तकेभ्यः शीर्षकस्य महत्तमम् आह्रियताम् ।", expect: Compiles("MAX(t1.title)") },
    // A negative constant is `-५`; a negative column follows `ऋणात्मक`, which
    // agrees with it. Nothing else takes either.
    Case { name: "negative_constant_in_a_comparison", ssql: "पुस्तकेभ्यः मूल्यम् -५ इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("t1.price > -5") },
    Case { name: "negative_constant_subtrahend", ssql: "पुस्तकेभ्यः मूल्यात् -५ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("t1.price - -5") },
    Case { name: "negative_constant_sum_operand", ssql: "पुस्तकेभ्यः मूल्यम् च -५ च इत्यनयोः योगः आह्रियताम् ।", expect: Compiles("t1.price + -5") },
    Case { name: "negative_constant_divisor", ssql: "पुस्तकेभ्यः मूल्यम् -२ विभज्य लब्धम् आह्रियताम् ।", expect: Compiles("/ -2") },
    Case { name: "negative_constant_update_value", ssql: "पुस्तकेषु सर्वत्र मूल्यम् -५ इति परिवर्त्यताम् ।", expect: Compiles("SET price = -5") },
    Case { name: "negative_constant_in_a_case", ssql: "पुस्तकेभ्यः वरणम् यस्मिन् मूल्यम् ५ इत्यस्मात् अधिकम् अस्ति तस्मिन् ऋणात्मकम् मूल्यम् अन्यस्मिन् -१ आह्रियताम् ।", expect: Compiles("THEN (-(t1.price)) ELSE -1") },
    Case { name: "sign_apart_from_digits_rejected", ssql: "पुस्तकेभ्यः मूल्यम् - ५ इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("against its digits") },
    Case { name: "sign_as_subtraction_of_a_word_rejected", ssql: "पुस्तकेभ्यः मूल्यम्-५ इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("against its digits") },
    Case { name: "sign_as_subtraction_of_digits_rejected", ssql: "पुस्तकेभ्यः मूल्यम् ५-३ इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("against its digits") },
    Case { name: "sign_on_a_column_rejected", ssql: "पुस्तकेभ्यः -मूल्यम् आह्रियताम् ।", expect: Rejects("against its digits") },
    Case { name: "negative_column_selected", ssql: "पुस्तकेभ्यः ऋणात्मकम् मूल्यम् आह्रियताम् ।", expect: Compiles("SELECT (-(t1.price)) FROM") },
    Case { name: "negative_qualified_column", ssql: "पुस्तकेभ्यः ऋणात्मकम् पुस्तकस्य मूल्यम् आह्रियताम् ।", expect: Compiles("SELECT (-(t1.price)) FROM") },
    Case { name: "negative_feminine_column", ssql: "पुस्तकेभ्यः ऋणात्मिका लेखकसङ्ख्या आह्रियताम् ।", expect: Compiles("SELECT (-(t1.author_id)) FROM") },
    Case { name: "negative_column_compared_agrees", ssql: "पुस्तकेभ्यः ऋणात्मकम् मूल्यम् ३ इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("(-(t1.price)) > 3") },
    Case { name: "negative_column_compared_disagrees", ssql: "पुस्तकेभ्यः ऋणात्मकम् मूल्यम् ३ इत्यस्मात् अधिकः अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    Case { name: "negative_of_a_named_value", ssql: "सकलाः इति आख्या सकल इति पुंलिङ्गम् बहुवचनम् पुस्तकेभ्यः मूल्यम् च २ च इत्यनयोः योगः सकलमूल्यम् इति आख्या आह्रियताम् । सकलेभ्यः ऋणात्मकम् सकलमूल्यम् आह्रियताम् ।", expect: Compiles("(-(t2.सकलमूल्यम्))") },
    Case { name: "negative_of_a_named_value_agrees", ssql: "सकलाः इति आख्या सकल इति पुंलिङ्गम् बहुवचनम् पुस्तकेभ्यः मूल्यम् च २ च इत्यनयोः योगः सकलमूल्यम् इति आख्या आह्रियताम् । सकलेभ्यः ऋणात्मकः सकलमूल्यम् आह्रियताम् ।", expect: Rejects("write `ऋणात्मकम्`") },
    Case { name: "negative_minuend", ssql: "पुस्तकेभ्यः ऋणात्मकात् मूल्यात् ५ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("(-(t1.price)) - 5") },
    Case { name: "negative_subtrahend", ssql: "पुस्तकेभ्यः मूल्यात् ऋणात्मकम् प्रकाशनवर्षम् विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Compiles("t1.price - (-(t1.publication_year))") },
    Case { name: "negative_divisor", ssql: "पुस्तकेभ्यः मूल्यम् ऋणात्मकेन प्रकाशनवर्षेण विभज्य लब्धम् आह्रियताम् ।", expect: Compiles("t1.price / (-(t1.publication_year))") },
    Case { name: "negative_wrong_gender", ssql: "पुस्तकेभ्यः ऋणात्मकः मूल्यम् आह्रियताम् ।", expect: Rejects("write `ऋणात्मकम्`") },
    Case { name: "negative_wrong_gender_feminine", ssql: "पुस्तकेभ्यः ऋणात्मकम् लेखकसङ्ख्या आह्रियताम् ।", expect: Rejects("write `ऋणात्मिका`") },
    Case { name: "negative_wrong_case", ssql: "पुस्तकेभ्यः ऋणात्मकम् मूल्यात् ५ विशोध्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("write `ऋणात्मकात्`") },
    Case { name: "negative_bare_stem_rejected", ssql: "पुस्तकेभ्यः ऋणात्मक मूल्यम् आह्रियताम् ।", expect: Rejects("Use `ऋणात्मकम्`") },
    Case { name: "negative_word_before_a_constant_rejected", ssql: "पुस्तकेभ्यः ऋणात्मकम् ५ आह्रियताम् ।", expect: Rejects("written with its sign") },
    Case { name: "negative_word_before_a_group_rejected", ssql: "पुस्तकेभ्यः ऋणात्मकम् यत् मूल्यम् च २ च इत्यनयोः योगः इति आह्रियताम् ।", expect: Rejects("name a longer value first") },
    Case { name: "negative_of_text_rejected", ssql: "पुस्तकेभ्यः ऋणात्मकम् शीर्षकम् आह्रियताम् ।", expect: Rejects("does not hold numbers") },
    // The genitive plural is only an aggregate's argument.
    Case { name: "genitive_plural_of_function_rejected", ssql: "पुस्तकेभ्यः शीर्षकाणाम् महावर्णरूपम् आह्रियताम् ।", expect: Rejects("genitive plural") },
    Case { name: "genitive_plural_partition_rejected", ssql: "पुस्तकेभ्यः पङ्क्तिसङ्ख्या यत्र भाषाणाम् आधारेण विभाजनम् मूल्यस्य अनुसारम् आरोहक्रमेण आह्रियताम् ।", expect: Rejects("genitive plural") },
    Case { name: "genitive_plural_window_order_rejected", ssql: "पुस्तकेभ्यः पङ्क्तिसङ्ख्या यत्र मूल्यानाम् अनुसारम् आरोहक्रमेण आह्रियताम् ।", expect: Rejects("genitive plural") },
    Case { name: "genitive_plural_group_key_rejected", ssql: "पुस्तकेभ्यः भाषाणाम् इति समूहेन पङ्क्तीनाम् गणना आह्रियताम् ।", expect: Rejects("genitive plural") },
    Case { name: "genitive_plural_windowed_aggregate", ssql: "पुस्तकेभ्यः मूल्यानाम् योगः यत्र भाषायाः आधारेण विभाजनम् आह्रियताम् ।", expect: Compiles("SUM(t1.price) OVER (PARTITION BY t1.language)") },
    Case { name: "count_distinct_of_genitive_plural", ssql: "पुस्तकेभ्यः विशिष्टानाम् भाषाणाम् गणना आह्रियताम् ।", expect: Compiles("COUNT(DISTINCT t1.language)") },
    Case { name: "distinct_agrees_with_the_word_it_qualifies", ssql: "पुस्तकेभ्यः विशिष्टायाः भाषाणाम् गणना आह्रियताम् ।", expect: Rejects("Distinct modifier") },
    // Messages and checks from the post-review batch.
    Case { name: "order_key_needs_direction", ssql: "लेखकेभ्यः नाम इति क्रमेण नाम आह्रियताम् ।", expect: Rejects("an ordering key names its direction") },
    Case { name: "bare_stem_current_row_rejected", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यस्य योगः यत्र मूल्यस्य अनुसारम् आरोहक्रमेण पङ्क्तयः अपरिमितपूर्ववर्तितः आरभ्य वर्तमानपङ्क्ति पर्यन्तम् चलयोगः इति आख्या च आह्रियेताम् ।", expect: Rejects("it is a bare stem") },
    Case { name: "mixed_list_in_filter_says_so", ssql: "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् अस्ति च भाषा \"क\" इत्यनया समा अस्ति वा इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("A list mixes 'च' (and) with 'वा' (or)") },
    Case { name: "set_op_pronoun_number", ssql: "ग्राहकेभ्यः नाम आह्रियताम् च लेखकेभ्यः नाम आह्रियताम् च इत्येषाम् सङ्गमः आह्रियताम् ।", expect: Rejects("these are 2, so write 'इत्यनयोः'") },
    Case { name: "reserved_word_physical_name_rejected", ssql: "आदेशाः इति पटलम् \"order\" इति आख्या आदेश इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।", expect: Rejects("is an SQL reserved word") },
    Case { name: "system_column_physical_name_rejected", ssql: "आदेशाः इति पटलम् \"orders\" इति आख्या आदेश इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"ctid\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।", expect: Rejects("is an SQL reserved word") },
    Case { name: "name_reserved_but_allowed_as_function_rejected", ssql: "आदेशाः इति पटलम् \"verbose\" इति आख्या आदेश इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।", expect: Rejects("is an SQL reserved word") },
    Case { name: "digit_first_physical_name_rejected", ssql: "आदेशाः इति पटलम् \"2nd\" इति आख्या आदेश इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।", expect: Rejects("starts with a digit") },
    Case { name: "punctuated_physical_name_rejected", ssql: "आदेशाः इति पटलम् \"a-b\" इति आख्या आदेश इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।", expect: Rejects("holds only letters, digits and `_`") },
    // A declared column is declined; the इति-pronoun is for an alias (pcf.md item 28).
    Case { name: "declared_column_takes_genitive", ssql: "पुस्तकेभ्यः मूल्यम् इत्यस्य योगः आह्रियताम् ।", expect: Rejects("is declined, `मूल्यस्य`") },
    // LAG / LEAD by n rows, by ordinal compounds (pcf.md item 6).
    Case { name: "lag_two_rows", ssql: "पुस्तकेभ्यः शीर्षकम् च द्वितीयपूर्वपङ्क्तेः मूल्यम् यत्र मूल्यस्य अनुसारम् आरोहक्रमेण पूर्वमूल्यम् इति आख्या च आह्रियेताम् ।", expect: Compiles("LAG(t1.price, 2) OVER (ORDER BY t1.price ASC NULLS LAST)") },
    Case { name: "lead_three_rows", ssql: "पुस्तकेभ्यः शीर्षकम् च तृतीयानन्तरपङ्क्तेः मूल्यम् यत्र मूल्यस्य अनुसारम् आरोहक्रमेण अग्रमूल्यम् इति आख्या च आह्रियेताम् ।", expect: Compiles("LEAD(t1.price, 3) OVER (ORDER BY t1.price ASC NULLS LAST)") },
    Case { name: "redundant_group_same_connective", ssql: "पुस्तकेभ्यः यत् मूल्यम् २० इत्यस्मात् अधिकम् अस्ति च भाषा \"पालि\" इत्यनया समा अस्ति च इति च शीर्षकम् \"क\" इत्यनेन समम् अस्ति च इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("group around a च-list is needed only as an item of a वा-list") },
    // --- Nested set operations keep their grouping ---
    Case { name: "setop_nested_left", ssql: "यत् ग्राहकेभ्यः नाम आह्रियताम् च लेखकेभ्यः नाम आह्रियताम् च इत्यनयोः सङ्गमः इति च प्रकाशकेभ्यः नाम आह्रियताम् च इत्यनयोः प्रतिच्छेदः आह्रियताम् ।", expect: Compiles("SELECT * FROM (SELECT t1.name FROM customers t1 UNION SELECT t2.name FROM authors t2) AS t3 INTERSECT SELECT t4.name FROM publishers t4") },
    // --- INSERT (locative target) ---
    Case { name: "insert_one_column_singular", ssql: "पुस्तकेषु शीर्षकम् \"नवग्रन्थः\" निवेश्यताम् ।", expect: Compiles("INSERT INTO books (title) VALUES ('नवग्रन्थः')") },
    // A particle joins items, so a single item takes none (keywords.md).
    Case { name: "insert_one_column_rejects_lone_cha", ssql: "पुस्तकेषु शीर्षकम् \"नवग्रन्थः\" च निवेश्यताम् ।", expect: Rejects("a single item takes no च") },
    Case { name: "select_one_column_rejects_lone_cha", ssql: "पुस्तकेभ्यः शीर्षकम् च आह्रियताम् ।", expect: Rejects("a single item takes no च") },
    Case { name: "filter_one_condition_rejects_lone_va", ssql: "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् अस्ति वा इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("a single item takes no वा") },
    Case { name: "insert_two_columns_dual", ssql: "पुस्तकेषु शीर्षकम् \"नवग्रन्थः\" च मूल्यम् ३० च निवेश्येताम् ।", expect: Compiles("INSERT INTO books (title, price) VALUES ('नवग्रन्थः', 30)") },
    Case { name: "insert_three_columns_plural", ssql: "पुस्तकेषु शीर्षकम् \"नवग्रन्थः\" च मूल्यम् ३० च अङ्कः \"९७८\" च निवेश्यन्ताम् ।", expect: Compiles("INSERT INTO books (title, price, isbn) VALUES") },
    Case { name: "insert_three_columns_rejects_singular", ssql: "पुस्तकेषु शीर्षकम् \"नवग्रन्थः\" च मूल्यम् ३० च अङ्कः \"९७८\" च निवेश्यताम् ।", expect: Rejects("inserts 3 columns, which needs 'निवेश्यन्ताम्'") },
    Case { name: "update_two_columns_dual", ssql: "पुस्तकेषु सङ्ख्या १ इत्यनया समा अस्ति इति नियमेन मूल्यम् ६० इति च भाषा \"पालि\" इति च परिवर्त्येताम् ।", expect: Compiles("UPDATE books SET price = 60, language = 'पालि' WHERE id = 1;") },
    // --- One verb per statement kind: former synonyms are rejected with the canonical form ---
    Case { name: "active_ahara_is_not_a_verb", ssql: "पुस्तकेभ्यः शीर्षकम् आहर ।", expect: Rejects("Use `आह्रियताम्`") },
    Case { name: "chiyatam_is_not_a_verb", ssql: "पुस्तकेभ्यः शीर्षकम् चियताम् ।", expect: Rejects("Use `आह्रियताम्`") },
    Case { name: "navikriyatam_is_not_a_verb", ssql: "पुस्तकेषु सङ्ख्या १ इत्यनया समा अस्ति इति नियमेन मूल्यम् ६० इति नवीक्रियताम् ।", expect: Rejects("Use `परिवर्त्यताम्`") },
    // --- An UPDATE or DELETE filter agrees like a query's ---
    Case { name: "update_filter_agrees", ssql: "पुस्तकेषु भाषा \"पालि\" इत्यनया समा अस्ति इति नियमेन मूल्यम् १० इति परिवर्त्यताम् ।", expect: Compiles("UPDATE books SET price = 10 WHERE language = 'पालि';") },
    Case { name: "update_filter_pronoun_gender_checked", ssql: "पुस्तकेषु भाषा \"पालि\" इत्यनेन समा अस्ति इति नियमेन मूल्यम् १० इति परिवर्त्यताम् ।", expect: Rejects("Pronoun Gender Mismatch") },
    Case { name: "delete_filter_pronoun_gender_checked", ssql: "पुस्तकेभ्यः भाषा \"पालि\" इत्यनेन समा अस्ति इति नियमेन निष्कास्यताम् ।", expect: Rejects("Pronoun Gender Mismatch") },
    // --- A feminine subject of अधिका/न्यूना takes the feminine ablative इत्यस्याः (HAVING and DML too) ---
    Case { name: "feminine_greater_takes_ityasyah", ssql: "पुस्तकेभ्यः सङ्ख्या ५ इत्यस्याः अधिका अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.id > 5") },
    Case { name: "feminine_greater_rejects_ityasmat", ssql: "पुस्तकेभ्यः सङ्ख्या ५ इत्यस्मात् अधिका अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("Pronoun Gender Mismatch") },
    Case { name: "having_count_takes_ityasyah", ssql: "पुस्तकेभ्यः भाषा इति समूहेन सङ्ख्यायाः गणना १ इत्यस्याः अधिका अस्ति इति समूहनियमेन भाषा च सङ्ख्यायाः गणना च आह्रियेताम् ।", expect: Compiles("HAVING COUNT(t1.id) > 1") },
    Case { name: "having_count_rejects_ityasmat", ssql: "पुस्तकेभ्यः भाषा इति समूहेन सङ्ख्यायाः गणना १ इत्यस्मात् अधिका अस्ति इति समूहनियमेन भाषा च सङ्ख्यायाः गणना च आह्रियेताम् ।", expect: Rejects("Pronoun Gender Mismatch") },
    Case { name: "delete_feminine_rejects_ityasmat", ssql: "ग्राहकेभ्यः सङ्ख्या ५ इत्यस्मात् अधिका अस्ति इति नियमेन निष्कास्यताम् ।", expect: Rejects("Pronoun Gender Mismatch") },
    // --- A comparison states its predicate; nothing defaults to `=` (expert review F4) ---
    Case { name: "comparison_without_adjective_rejected", ssql: "पुस्तकेभ्यः मूल्यम् ५ इत्यस्मात् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    Case { name: "comparison_never_chains_equalities", ssql: "पुस्तकेभ्यः भाषा \"x\" इत्यनया विधा अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    Case { name: "delete_comparison_never_chains", ssql: "ग्राहकेभ्यः नाम \"x\" इत्यनेन विद्युत्पत्रम् अस्ति इति नियमेन निष्कास्यताम् ।", expect: Rejects("") },
    Case { name: "greater_takes_ablative_not_instrumental", ssql: "पुस्तकेभ्यः मूल्यम् ५ इत्यनेन अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    Case { name: "comparison_requires_asti", ssql: "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    Case { name: "bare_equality_retired", ssql: "पुस्तकेभ्यः अङ्कः \"978\" अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    Case { name: "bhinna_retired", ssql: "पुस्तकेभ्यः सङ्ख्या ५ इत्यस्याः भिन्ना अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("Use `इत्यनया समा नास्ति`") },
    Case { name: "antarbhavati_retired", ssql: "पुस्तकेभ्यः भाषा \"English\" च \"Sanskrit\" च अन्तर्भवति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("no longer SSQL") },
    Case { name: "not_less_is_greater_equal", ssql: "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् न्यूनम् नास्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.price >= 20") },
    Case { name: "not_greater_is_less_equal", ssql: "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् नास्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.price <= 20") },
    Case { name: "not_equal_is_negated_sama", ssql: "पुस्तकेभ्यः सङ्ख्या ५ इत्यनया समा नास्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.id <> 5") },
    Case { name: "like_feminine_is_sadrushi", ssql: "पुस्तकेभ्यः भाषा \"S%\" इत्यनया सदृशी अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.language LIKE 'S%'") },
    Case { name: "like_gender_checked", ssql: "पुस्तकेभ्यः भाषा \"S%\" इत्यनया सदृशम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("Adjective Gender Mismatch") },
    Case { name: "not_like", ssql: "पुस्तकेभ्यः भाषा \"S%\" इत्यनया सदृशी नास्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.language NOT LIKE 'S%'") },
    Case { name: "not_between", ssql: "पुस्तकेभ्यः मूल्यम् २० च ५० च इत्यनयोः मध्ये नास्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.price NOT BETWEEN 20 AND 50") },
    Case { name: "not_in", ssql: "पुस्तकेभ्यः भाषा \"English\" च \"Hindi\" च इत्यनयोः नास्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.language NOT IN ('English', 'Hindi')") },
    // --- A grouped right operand keeps its grouping (integer division) ---
    Case { name: "grouped_division_right_operand", ssql: "पुस्तकेभ्यः ३ च यत् प्रकाशनवर्षम् २ विभज्य लब्धम् इति च इत्यनयोः गुणनम् आह्रियताम् ।", expect: Compiles("3 * (t1.publication_year / 2)") },
    // --- A view whose query filters (it used to panic in codegen) ---
    Case { name: "create_view_with_filter", ssql: "महार्घाणि इति दृश्यम् \"pricey\" इति आख्या महार्घ इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत् पुस्तकेभ्यः भाषा \"x\" इत्यनया समा अस्ति इति नियमेन शीर्षकम् आह्रियताम् इति दृश्यम् ।", expect: Compiles("WHERE t1.language = 'x'") },
    // --- Pronouns are the इदम् forms; a list's pronoun agrees with the column (expert review Q2) ---
    Case { name: "dual_pronoun_is_ityanayoh", ssql: "पुस्तकेभ्यः मूल्यम् २० च ५० च इत्यनयोः मध्ये अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.price BETWEEN 20 AND 50") },
    Case { name: "ityayoh_retired", ssql: "पुस्तकेभ्यः मूल्यम् २० च ५० च इत्ययोः मध्ये अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("Use `इत्यनयोः`") },
    Case { name: "feminine_in_list_is_ityasu", ssql: "पुस्तकेभ्यः भाषा \"a\" च \"b\" च \"c\" च इत्यासु अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.language IN ('a', 'b', 'c')") },
    Case { name: "feminine_in_list_rejects_ityeshu", ssql: "पुस्तकेभ्यः भाषा \"a\" च \"b\" च \"c\" च इत्येषु अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("Pronoun Gender Mismatch") },
    Case { name: "neuter_in_list_is_ityeshu", ssql: "पुस्तकेभ्यः मूल्यम् १० च २० च ३० च इत्येषु अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.price IN (10, 20, 30)") },
    // --- A row filter is `इति नियमेन`; चेत् conditions a whole statement (expert review Q3) ---
    Case { name: "where_chet_rejected", ssql: "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् अस्ति चेत् शीर्षकम् आह्रियताम् ।", expect: Rejects("") },
    // --- One form per construct (expert review Q4) ---
    Case { name: "delete_synonym_retired", ssql: "ग्राहकेभ्यः सङ्ख्या ५ इत्यस्याः अधिका अस्ति इति नियमेन विलोप्यताम् ।", expect: Rejects("Use `निष्कास्यताम्`") },
    Case { name: "nivarya_retired", ssql: "पुस्तकेभ्यः मूल्यात् १० निवार्य अवशिष्टम् आह्रियताम् ।", expect: Rejects("Use `विशोध्य`") },
    Case { name: "keyword_class_nasal_spelling", ssql: "लक्ष्यम् इति बाह्या संज्ञा । पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।", expect: Rejects("Use `सञ्ज्ञा`") },
    Case { name: "udf_passive_params", ssql: "द्विगुणितम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च मूल्यम् इति वास्तविकः दीयते मूल्यम् च २ च इत्यनयोः गुणनम् इति उत्तरम् प्रतिदीयताम् इति कार्यम् । पुस्तकेभ्यः मूल्यस्य द्विगुणितम् द्विगुणम् इति आख्या आह्रियताम् ।", expect: Compiles("((t1.price) * 2) AS द्विगुणम्") },
    Case { name: "udf_absolutive_params_retired", ssql: "द्विगुणितम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च मूल्यम् इति वास्तविकः आदाय मूल्यम् च २ च इत्यनयोः गुणनम् इति उत्तरम् प्रतिदीयताम् इति कार्यम् । पुस्तकेभ्यः मूल्यस्य द्विगुणितम् द्विगुणम् इति आख्या आह्रियताम् ।", expect: Rejects("Use `दीयते / दीयेते / दीयन्ते`") },
    Case { name: "udf_old_return_verb_retired", ssql: "द्विगुणितम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिददाति च मूल्यम् इति वास्तविकः दीयते मूल्यम् च २ च इत्यनयोः गुणनम् इति उत्तरम् प्रतिदीयताम् इति कार्यम् । पुस्तकेभ्यः शीर्षकम् आह्रियताम् ।", expect: Rejects("Use `प्रतिदीयते`") },
    // --- A subquery whose own filter starts with a qualified column (it was read as a group ending at its own इति) ---
    Case { name: "subquery_filter_with_qualified_column", ssql: "पुस्तकेभ्यः यत् आपणेभ्यः आपणस्य नाम \"x\" इत्यनेन समम् अस्ति इति नियमेन सर्वम् आह्रियताम् इति विद्यते इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE EXISTS (SELECT * FROM stores t2 WHERE t2.name = 'x')") },
    Case { name: "derived_table_with_qualified_filter", ssql: "यत् आपणेभ्यः आपणस्य नाम \"x\" इत्यनेन समम् अस्ति इति नियमेन सर्वम् आह्रियताम् इति आश्रित्य सर्वम् आह्रियताम् ।", expect: Compiles("WHERE t1.name = 'x'") },
    // --- UPDATE: `<col> <value> इति`, a filter or सर्वत्र (expert review F3, Q5) ---
    Case { name: "update_every_row_is_sarvatra", ssql: "पुस्तकेषु सर्वत्र मूल्यम् ६० इति परिवर्त्यताम् ।", expect: Compiles("UPDATE books SET price = 60;") },
    Case { name: "update_needs_filter_or_sarvatra", ssql: "पुस्तकेषु मूल्यम् ६० इति परिवर्त्यताम् ।", expect: Rejects("") },
    Case { name: "update_pair_needs_iti", ssql: "पुस्तकेषु सर्वत्र मूल्यम् ६० परिवर्त्यताम् ।", expect: Rejects("") },
    Case { name: "update_parivartanena_retired", ssql: "पुस्तकेषु सर्वत्र मूल्यम् ६० इति परिवर्तनेन परिवर्त्यताम् ।", expect: Rejects("no longer SSQL") },
    Case { name: "update_list_needs_each_cha", ssql: "पुस्तकेषु सर्वत्र मूल्यम् ६० इति भाषा \"पालि\" इति च परिवर्त्येताम् ।", expect: Rejects("") },
    Case { name: "update_filter_may_hold_subquery", ssql: "पुस्तकेषु सङ्ख्या यत् विक्रयणवस्तुभ्यः विक्रयणवस्तुनः पुस्तकसङ्ख्या आह्रियताम् इति इत्यासु नास्ति इति नियमेन मूल्यम् १० इति परिवर्त्यताम् ।", expect: Compiles("WHERE id NOT IN (SELECT") },
    Case { name: "delete_filter_may_hold_subquery", ssql: "पुस्तकेभ्यः सङ्ख्या यत् विक्रयणवस्तुभ्यः विक्रयणवस्तुनः पुस्तकसङ्ख्या आह्रियताम् इति इत्यासु नास्ति इति नियमेन निष्कास्यताम् ।", expect: Compiles("DELETE FROM books WHERE id NOT IN (SELECT") },
    // --- COUNT(*) is `पङ्क्तीनाम् गणना` (expert review F5) ---
    Case { name: "count_rows", ssql: "पुस्तकेभ्यः पङ्क्तीनाम् गणना आह्रियताम् ।", expect: Compiles("SELECT COUNT(*) FROM books t1;") },
    Case { name: "count_rows_windowed", ssql: "पुस्तकेभ्यः शीर्षकम् च पङ्क्तीनाम् गणना यत्र भाषायाः आधारेण विभाजनम् भाषागणना इति आख्या च आह्रियेताम् ।", expect: Compiles("COUNT(*) OVER (PARTITION BY t1.language) AS भाषागणना") },
    Case { name: "count_rows_having", ssql: "पुस्तकेभ्यः भाषा इति समूहेन पङ्क्तीनाम् गणना २ इत्यस्याः अधिका अस्ति इति समूहनियमेन भाषा आह्रियताम् ।", expect: Compiles("HAVING COUNT(*) > 2") },
    Case { name: "count_sarvam_retired", ssql: "पुस्तकेभ्यः सर्वम् इत्यस्य गणना आह्रियताम् ।", expect: Rejects("पङ्क्तीनाम् गणना") },
    // --- Alias `<alias> इति आख्या` (G1); aggregate window keys decline (G2); ORDER/LIMIT before the select list (H1) ---
    Case { name: "alias_is_name_iti_akhya", ssql: "पुस्तकेभ्यः मूल्यस्य योगः सकलमूल्यम् इति आख्या आह्रियताम् ।", expect: Compiles("SELECT SUM(t1.price) AS सकलमूल्यम्") },
    Case { name: "alias_iti_name_retired", ssql: "पुस्तकेभ्यः मूल्यस्य योगः इति सकलमूल्यम् आह्रियताम् ।", expect: Rejects("") },
    Case { name: "window_key_declined_aggregate", ssql: "पुस्तकेभ्यः भाषा इति समूहेन भाषा च श्रेणी यत्र मूल्यस्य योगस्य अनुसारम् अवरोहक्रमेण क्रमः इति आख्या च आह्रियेताम् ।", expect: Compiles("RANK() OVER (ORDER BY SUM(t1.price) DESC NULLS LAST) AS क्रमः") },
    Case { name: "order_limit_before_select", ssql: "पुस्तकेभ्यः मूल्यम् अवरोहः इति क्रमेण ३ इति मित्या शीर्षकम् च मूल्यम् च आह्रियेताम् ।", expect: Compiles("ORDER BY t1.price DESC NULLS LAST LIMIT 3") },
    Case { name: "order_after_select_retired", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यम् च यत् मूल्यम् अवरोहः इति क्रमेण आह्रियेताम् ।", expect: Rejects("") },
    // --- No parentheses; native forms for what only parentheses could say (expert review Q6, Q7) ---
    Case { name: "parentheses_rejected", ssql: "पुस्तकेभ्यः ( शीर्षकम् ) आह्रियताम् ।", expect: Rejects("parentheses are not used") },
    // CAST: the operand in the genitive, the type as its form (pcf.md item 4).
    Case { name: "cast_native", ssql: "पुस्तकेभ्यः मूल्यस्य पूर्णाङ्करूपेण परिणामः पूर्णमूल्यम् इति आख्या आह्रियताम् ।", expect: Compiles("CAST(t1.price AS INTEGER) AS पूर्णमूल्यम्") },
    Case { name: "cast_literal_by_pronoun", ssql: "पुस्तकेभ्यः \"५\" इत्यस्य पूर्णाङ्करूपेण परिणामः आह्रियताम् ।", expect: Compiles("CAST('५' AS INTEGER)") },
    Case { name: "cast_previous_form_retired", ssql: "पुस्तकेभ्यः मूल्यम् इत्यस्य पूर्णाङ्कः इति परिणामः आह्रियताम् ।", expect: Rejects("a cast is now written with the operand in the genitive") },
    Case { name: "cast_old_keyword_retired", ssql: "पुस्तकेभ्यः मूल्यम् इति पूर्णाङ्कः परिवर्तनम् आह्रियताम् ।", expect: Rejects("परिणामः") },
    Case { name: "index_two_columns", ssql: "पुस्तकानाम् शीर्षकम् आरोहः च मूल्यम् अवरोहः च इत्यनयोः आधारे \"idx_tp\" इति सूची रच्यताम् ।", expect: Compiles("CREATE INDEX idx_tp ON books (title ASC, price DESC)") },
    Case { name: "index_one_directed_column", ssql: "पुस्तकानाम् मूल्यम् अवरोहः इत्यस्य आधारे \"idx_p\" इति सूची रच्यताम् ।", expect: Compiles("CREATE INDEX idx_p ON books (price DESC)") },
    Case { name: "index_list_pronoun_must_match", ssql: "पुस्तकानाम् शीर्षकम् आरोहः च मूल्यम् अवरोहः च इत्येषाम् आधारे \"idx_tp\" इति सूची रच्यताम् ।", expect: Rejects("") },
    Case { name: "intersect_is_praticchedah", ssql: "ग्राहकेभ्यः नाम आह्रियताम् च लेखकेभ्यः नाम आह्रियताम् च इत्यनयोः प्रतिच्छेदः आह्रियताम् ।", expect: Compiles("INTERSECT") },
    Case { name: "intersect_old_word_retired", ssql: "ग्राहकेभ्यः नाम आह्रियताम् च लेखकेभ्यः नाम आह्रियताम् च इत्यनयोः सम्मिलनम् आह्रियताम् ।", expect: Rejects("प्रतिच्छेदः") },
    Case { name: "cross_join_no_condition", ssql: "विक्रयाधिकारिभ्यः आपणैः सह सर्वमेलनेन विक्रयाधिकारिणः नाम आह्रियताम् ।", expect: Compiles("CROSS JOIN stores t2") },
    Case { name: "left_join_marker", ssql: "विक्रयाधिकारिभ्यः आपणैः सह विक्रयाधिकारिणः आपणसङ्ख्या आपणस्य सङ्ख्या इत्यनया समा अस्ति इति वाममेलनेन विक्रयाधिकारिणः नाम आह्रियताम् ।", expect: Compiles("LEFT JOIN stores t2 ON") },
    Case { name: "insert_select_grouped", ssql: "प्रकाशकेषु नाम यत् लेखकेभ्यः नाम आह्रियताम् इति निवेश्यताम् ।", expect: Compiles("INSERT INTO publishers (name) SELECT t1.name FROM authors t1;") },
    // --- Names match their declined forms exactly (B18/B19) ---
    Case { name: "bare_stem_is_not_a_word", ssql: "पुस्तकेभ्यः शीर्षक आह्रियताम् ।", expect: Rejects("is not a form of `शीर्षक`") },
    Case { name: "final_anusvara_is_misspelled", ssql: "पुस्तकेभ्यः मूल्यं आह्रियताम् ।", expect: Rejects("ends in an anusvāra; write `म्`") },
    Case { name: "missed_natva_is_not_a_form", ssql: "पुस्तकेभ्यः लेखकैः सह पुस्तकस्य लेखकसङ्ख्या लेखकाणाम् सङ्ख्या इत्यनया समा अस्ति इति मेलनेन शीर्षकम् आह्रियताम् ।", expect: Rejects("is not a form of") },
    // --- An aggregate / date part takes a genitive or the genitive pronoun, never a bare nominative ---
    Case { name: "aggregate_nominative_argument_rejected", ssql: "पुस्तकेभ्यः मूल्यम् योगः आह्रियताम् ।", expect: Rejects("") },
    Case { name: "date_part_nominative_argument_rejected", ssql: "विक्रयणेभ्यः विक्रयणदिनाङ्कः वर्षम् आह्रियताम् ।", expect: Rejects("") },
    // Orthography is checked by the lexer (lexicon.rs); a bare stem is not a word (P1b).
    Case { name: "anusvara_before_stop_is_misspelled", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यम् च आह्रियेताम् । पुस्तकेभ्यः संख्या आह्रियताम् ।", expect: Rejects("write the class nasal") },
    Case { name: "builtin_bare_stem_rejected", ssql: "पुस्तकेभ्यः मूल्यस्य वर्गमूल आह्रियताम् ।", expect: Rejects("write `वर्गमूलम्`") },
    Case { name: "builtin_declined_name", ssql: "पुस्तकेभ्यः मूल्यस्य वर्गमूलम् आह्रियताम् ।", expect: Compiles("sqrt(t1.price)") },
    Case { name: "gender_word_bare_stem_rejected", ssql: "परीक्षाः इति पटलम् \"exams\" इति आख्या परीक्षा इति स्त्रीलिङ्ग बहुवचनम् रच्यताम् यत्र सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।", expect: Rejects("Use `स्त्रीलिङ्गम्`") },
    Case { name: "external_parameter_agrees", ssql: "लक्ष्यम् इति बाह्या सञ्ज्ञा । पुस्तकेभ्यः मूल्यम् लक्ष्यम् इत्यनेन समम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.price =") },
    // K2b: a CTE is read in the ablative of the number it is declared in.
    Case { name: "cte_source_agrees_with_declared_number", ssql: "सुलभाः इति आख्या सुलभ इति पुंलिङ्गम् बहुवचनम् पुस्तकेभ्यः शीर्षकम् आह्रियताम् । सुलभात् शीर्षकम् आह्रियताम् ।", expect: Rejects("is declared plural, so it is read in the ablative plural") },
    Case { name: "cte_collective_singular_source", ssql: "पुस्तकसारः इति आख्या पुस्तकसार इति पुंलिङ्गम् एकवचनम् पुस्तकेभ्यः शीर्षकम् आह्रियताम् । पुस्तकसारात् शीर्षकम् आह्रियताम् ।", expect: Compiles("FROM") },
    // N1: SELECT DISTINCT is a row-level phrase before the select list.
    Case { name: "select_distinct_without_repetition", ssql: "पुस्तकेभ्यः पुनरुक्तिम् विना भाषा आह्रियताम् ।", expect: Compiles("SELECT DISTINCT t1.language") },
    Case { name: "select_distinct_adjective_retired", ssql: "पुस्तकेभ्यः विशिष्टम् भाषा आह्रियताम् ।", expect: Rejects("Use `पुनरुक्तिम् विना`") },
    // O1: a constraint adjective agrees with its column.
    Case { name: "constraint_agrees_with_column", ssql: "परीक्षाः इति पटलम् \"exams\" इति आख्या परीक्षा इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी क्रमिका सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।", expect: Compiles("id INTEGER PRIMARY KEY") },
    Case { name: "constraint_wrong_gender_rejected", ssql: "परीक्षाः इति पटलम् \"exams\" इति आख्या परीक्षा इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत्र क्रमिकः प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।", expect: Rejects("write `क्रमिका`") },
    Case { name: "constraint_bare_stem_rejected", ssql: "परीक्षाः इति पटलम् \"exams\" इति आख्या परीक्षा इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत्र अनिवार्य सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् इति पटलम् ।", expect: Rejects("a constraint adjective agrees with its column") },
    // M1: a foreign key names its table in the locative plural, "tied to".
    Case { name: "foreign_key_locative_plural", ssql: "परीक्षाः इति पटलम् \"exams\" इति आख्या परीक्षा इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च लेखकेषु बद्धा लेखकसङ्ख्या इति पूर्णाङ्कः \"author_id\" इति आख्या लेखकसङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च इति पटलम् ।", expect: Compiles("REFERENCES authors") },
    Case { name: "foreign_key_prati_retired", ssql: "परीक्षाः इति पटलम् \"exams\" इति आख्या परीक्षा इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च लेखकाः प्रति बद्धा लेखकसङ्ख्या इति पूर्णाङ्कः \"author_id\" इति आख्या लेखकसङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च इति पटलम् ।", expect: Rejects("is no longer SSQL: a foreign key names its table in the locative plural") },
    Case { name: "foreign_key_must_be_locative", ssql: "परीक्षाः इति पटलम् \"exams\" इति आख्या परीक्षा इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत्र प्राथमिकी सङ्ख्या इति पूर्णाङ्कः \"id\" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च लेखकैः बद्धा लेखकसङ्ख्या इति पूर्णाङ्कः \"author_id\" इति आख्या लेखकसङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च इति पटलम् ।", expect: Rejects("a foreign key names its table in the locative plural, `लेखकेषु`") },
    // O2: प्रति takes the accusative.
    Case { name: "rollback_to_savepoint_accusative", ssql: "\"sp\" इति सङ्ग्रहबिन्दुम् प्रति व्यवहारः प्रत्याह्रियताम् ।", expect: Compiles("ROLLBACK TO SAVEPOINT sp") },
    Case { name: "rollback_to_savepoint_nominative_rejected", ssql: "\"sp\" इति सङ्ग्रहबिन्दुः प्रति व्यवहारः प्रत्याह्रियताम् ।", expect: Rejects("") },
    // L1: every item of a च/वा list takes its own particle, the last one too.
    Case { name: "select_list_last_item_takes_cha", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यम् आह्रियेताम् ।", expect: Rejects("add `च` after `मूल्यम्`") },
    Case { name: "select_list_trailing_cha", ssql: "पुस्तकेभ्यः शीर्षकम् च मूल्यम् च आह्रियेताम् ।", expect: Compiles("SELECT t1.title, t1.price FROM books t1;") },
    Case { name: "condition_list_last_item_takes_cha", ssql: "पुस्तकेभ्यः मूल्यम् ५० इत्यस्मात् अधिकम् अस्ति च भाषा \"Sanskrit\" इत्यनया समा अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("add `च` after `अस्ति`") },
    Case { name: "condition_list_trailing_cha", ssql: "पुस्तकेभ्यः मूल्यम् ५० इत्यस्मात् अधिकम् अस्ति च भाषा \"Sanskrit\" इत्यनया समा अस्ति च इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE (t1.price > 50 AND t1.language = 'Sanskrit')") },
    Case { name: "or_list_last_item_takes_va", ssql: "पुस्तकेभ्यः भाषा \"Hindi\" इत्यनया समा अस्ति वा भाषा \"Sanskrit\" इत्यनया समा अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Rejects("add `वा` after `अस्ति`") },
    Case { name: "lone_condition_takes_no_particle", ssql: "पुस्तकेभ्यः मूल्यम् ५० इत्यस्मात् अधिकम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", expect: Compiles("WHERE t1.price > 50") },
    Case { name: "table_qualifier_is_genitive_singular", ssql: "पुस्तकेभ्यः पुस्तकस्य शीर्षकम् आह्रियताम् ।", expect: Compiles("SELECT t1.title FROM books t1;") },
    Case { name: "table_qualifier_rejects_genitive_plural", ssql: "पुस्तकेभ्यः पुस्तकानाम् शीर्षकम् आह्रियताम् ।", expect: Rejects("a column's table is named in the genitive singular") },
    Case { name: "update_one_column_rejects_plural", ssql: "पुस्तकेषु सङ्ख्या १ इत्यनया समा अस्ति इति नियमेन मूल्यम् ६० इति परिवर्त्यन्ताम् ।", expect: Rejects("changes 1 column, which needs 'परिवर्त्यताम्'") },
];

/// Postgres-only expectations (fragments of the generated SQL).
const PG_CASES: &[(&str, &str, &str)] = &[
    ("pg_time_of_a_date_column", "विक्रयणेभ्यः विक्रयणदिनाङ्कस्य समयः आह्रियताम् ।", "AS TIME))"),
    ("pg_datetime_whole_seconds", "विक्रयणेभ्यः विक्रयणदिनाङ्कस्य दिनाङ्कसमयः आह्रियताम् ।", "date_trunc('second', CAST("),
    ("pg_epoch_whole_seconds", "विक्रयणेभ्यः विक्रयणदिनाङ्कस्य युगसमयः आह्रियताम् ।", "FLOOR(EXTRACT(EPOCH"),
    ("pg_real_type_is_double", "मापाः इति पटलम् \"measures\" इति आख्या माप इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र मूल्यम् इति वास्तविकः \"v\" इति आख्या मूल्य इति नपुंसकलिङ्गम् एकवचनम् इति पटलम् ।", "v NUMERIC"),
    // SQLite's LIKE ignores case; ILIKE is the same match on PostgreSQL.
    ("pg_like_is_ilike", "लेखकेभ्यः नाम \"k%\" इत्यनेन सदृशम् अस्ति इति नियमेन नाम आह्रियताम् ।", "WHERE t1.name ILIKE 'k%'"),
    // PostgreSQL has no datetime(); a timestamp cast is the same value.
    ("pg_datetime_is_cast", "विक्रयणेभ्यः विक्रयणदिनाङ्कस्य दिनाङ्कसमयः आह्रियताम् ।", "CAST(t1.sale_date AS TIMESTAMP)"),
    // PostgreSQL's reverse() takes text; a number is cast first.
    ("pg_reverse_of_number", "पुस्तकेभ्यः मूल्यस्य विपरीतम् आह्रियताम् ।", "reverse(CAST(t1.price AS TEXT))"),
    // One meaning on both dialects: text sorts by byte value, division by
    // zero is NULL, the current moment is UTC to the second.
    ("pg_text_sort_bytewise", "लेखकेभ्यः नाम आरोहः इति क्रमेण नाम आह्रियताम् ।", "ORDER BY t1.name COLLATE \"C\" ASC NULLS LAST"),
    ("pg_division_by_zero_null", "पुस्तकेभ्यः मूल्यम् ० विभज्य लब्धम् आह्रियताम् ।", "t1.price / NULLIF(0, 0)"),
    ("pg_now_utc", "पुस्तकेभ्यः अद्यतनक्षणः आह्रियताम् ।", "date_trunc('second', now() AT TIME ZONE 'UTC')"),
    ("pg_weekday_number", "समीक्षाभ्यः समीक्षायाः समीक्षादिनाङ्कस्य वारः समीक्षावारः इति आख्या आह्रियताम् ।", "CAST(EXTRACT(DOW FROM t1.review_date) AS INTEGER) AS समीक्षावारः"),
    ("pg_order_by_nulls_last", "पुस्तकेभ्यः पुस्तकस्य मूल्यम् अवरोहः इति क्रमेण शीर्षकम् आह्रियताम् ।", "ORDER BY t1.price DESC NULLS LAST"),
    // Second review, section C: the same result as SQLite.
    ("pg_round_half_away_from_zero", "पुस्तकेभ्यः मूल्यस्य २ इत्यनेन सन्निकटनम् आह्रियताम् ।", "round(CAST(t1.price AS NUMERIC), 2)"),
    ("pg_cast_to_integer_truncates", "पुस्तकेभ्यः मूल्यस्य पूर्णाङ्करूपेण परिणामः आह्रियताम् ।", "CAST(TRUNC(CAST(t1.price AS NUMERIC)) AS BIGINT)"),
    ("pg_remainder_by_zero_null", "पुस्तकेभ्यः मूल्यस्य ० इत्यनेन शेषः आह्रियताम् ।", "mod(t1.price, NULLIF(0, 0))"),
    ("pg_root_of_negative_null", "पुस्तकेभ्यः मूल्यात् १०० विशोध्य अवशिष्टम् इत्यस्य वर्गमूलम् आह्रियताम् ।", "sqrt(CASE WHEN (t1.price - 100) >= 0 THEN t1.price - 100 END)"),
    ("pg_like_backslash_ordinary", "पुस्तकेभ्यः शीर्षकम् \"A%\" इत्यनेन सदृशम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।", "t1.title ILIKE 'A%' ESCAPE ''"),
    ("pg_comparison_as_value", "पुस्तकेभ्यः मूल्यम् २० इत्यस्मात् अधिकम् अस्ति महार्घम् इति आख्या आह्रियताम् ।", "CAST((t1.price > 20) AS INTEGER) AS महार्घम्"),
    ("pg_days_since_utc", "विक्रयणेभ्यः विक्रयणदिनाङ्कस्य अन्तरम् आह्रियताम् ।", "(CAST(now() AT TIME ZONE 'UTC' AS DATE) - CAST(t1.sale_date AS DATE))"),
];

fn normalise(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn sqlite_with_schema() -> Connection {
    let ddl = compile_ssql_with_schema_source(SCHEMA, "", SQLDialect::SQLite)
        .expect("bookstore schema must compile");
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(&ddl).expect("compiled schema DDL must execute in SQLite");
    conn
}

/// Prepare the last statement after running the ones before it (a view or
/// table it reads from), all rolled back.
fn prepare_in_order(conn: &Connection, sql: &str) -> rusqlite::Result<()> {
    let statements: Vec<&str> = sql.split(";\n").map(str::trim).filter(|s| !s.is_empty() && !s.lines().all(|l| l.trim_start().starts_with("--"))).collect();
    let Some((last, earlier)) = statements.split_last() else { return Ok(()) };
    conn.execute_batch("SAVEPOINT rules_case")?;
    let result = earlier.iter().try_for_each(|s| conn.execute_batch(s)).and_then(|_| conn.prepare(last).map(|_| ()));
    conn.execute_batch("ROLLBACK TO rules_case; RELEASE rules_case")?;
    result
}

/// Parse time must not grow exponentially with groups or their nesting: 4 mixed
/// groups took 23 s, 4 grouped arithmetic items 52 s, and these reached the
/// 30 s timeout. The bound is far above the time they take, even in a debug
/// build, and far below that timeout.
#[test]
fn groups_parse_in_reasonable_time() {
    let group = "यत् मूल्यम् २० इत्यस्मात् अधिकम् अस्ति वा भाषा \"Sanskrit\" इत्यनया समा अस्ति वा इति च ";
    let groups = format!("पुस्तकेभ्यः {} प्रकाशनवर्षम् २००० इत्यस्मात् न्यूनम् अस्ति च इति नियमेन शीर्षकम् आह्रियताम् ।", group.repeat(8));
    let item = "यत् मूल्यात् १० विशोध्य अवशिष्टम् इति च २ च इत्यनयोः गुणनम् च ";
    let items = format!("पुस्तकेभ्यः {} आह्रियन्ताम् ।", item.repeat(6));
    let nested = "पुस्तकेभ्यः यत् यत् यत् मूल्यम् च २ च इत्यनयोः योगः इति च ३ च इत्यनयोः गुणनम् इति च २ च इत्यनयोः योगः इति च ३ च इत्यनयोः गुणनम् आह्रियताम् ।".to_string();
    for query in [groups, items, nested] {
        let started = std::time::Instant::now();
        let compiled = compile_ssql_with_schema_source(&query, SCHEMA, SQLDialect::SQLite);
        assert!(compiled.is_ok(), "{}: {:?}", query, compiled.err());
        assert!(started.elapsed().as_secs() < 15, "{} took {:?}", query, started.elapsed());
    }
}

#[test]
fn language_rules() {
    let conn = sqlite_with_schema();
    let mut failures = Vec::new();
    for case in CASES {
        let result = compile_ssql_with_schema_source(case.ssql, SCHEMA, SQLDialect::SQLite);
        match (&case.expect, result) {
            (Compiles(fragment), Ok(sql)) => {
                let got = normalise(&sql);
                if !got.contains(&normalise(fragment)) {
                    failures.push(format!("{}: expected SQL containing `{}`, got `{}`", case.name, fragment, got));
                } else if let Err(e) = prepare_in_order(&conn, &sql) {
                    failures.push(format!("{}: compiled SQL is invalid in SQLite ({}): `{}`", case.name, e, got));
                }
            }
            (Compiles(_), Err(e)) => failures.push(format!("{}: expected to compile, got error: {:#}", case.name, e)),
            (Rejects(fragment), Ok(sql)) => failures.push(format!("{}: expected rejection containing `{}`, but it compiled to `{}`", case.name, fragment, normalise(&sql))),
            (Rejects(fragment), Err(e)) => {
                let msg = format!("{:#}", e);
                if !msg.contains(fragment) {
                    failures.push(format!("{}: rejected for the wrong reason; expected `{}`, got `{}`", case.name, fragment, msg));
                }
            }
        }
    }
    for (name, ssql, fragment) in PG_CASES {
        match compile_ssql_with_schema_source(ssql, SCHEMA, SQLDialect::PostgreSQL) {
            Ok(sql) if normalise(&sql).contains(&normalise(fragment)) => {}
            Ok(sql) => failures.push(format!("{}: expected Postgres SQL containing `{}`, got `{}`", name, fragment, normalise(&sql))),
            Err(e) => failures.push(format!("{}: expected to compile for Postgres, got error: {:#}", name, e)),
        }
    }
    assert!(failures.is_empty(), "{} of {} language-rule cases failed:\n{}", failures.len(), CASES.len(), failures.join("\n"));
}
