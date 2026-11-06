/**
 * संस्कृतम् SQL (SSQL) Launch Website & Interactive Playground Application
 *
 * - Runs the actual Rust `ssql_compiler` in-browser via WebAssembly (`./pkg/ssql_wasm.js`)
 * - Browses and edits every example project (`data/corpus.json`) + 8 featured scenarios
 * - Provides live Devanagari syntax highlighting, span-accurate error/lint squigglies,
 *   gutter markers, interactive hover/cursor grammar help, and autocomplete (`Ctrl+Space`)
 * - Provides a 7-category Chip Palette covering all SSQL keywords & declined schema stems
 * - Renders all 10 tutorial chapters (`data/content.json`) with runnable end-of-chapter exercises
 * - Renders direct download links (`/downloads/*`) and the inline 10-pillar SSQL Constitution
 */

import initWasm, {
  compile_ssql_wasm,
  format_ssql_wasm,
  lint_ssql_wasm,
  transpile_sql_to_ssql_wasm,
} from "./pkg/ssql_wasm.js";

const FEATURED_SCENARIOS = {
  filter: {
    title: "1. Basic Filtering & Dual Verb • पुस्तकेभ्यः ... आह्रियेताम्",
    description:
      "Fetch title and price of books costing under 20 using Ablative plural पुस्तकेभ्यः for FROM, Neuter न्यूनम् agreeing with मूल्यम्, and Dual आह्रियेताम् for 2 columns.",
    sample_sql: "SELECT title, price\nFROM books\nWHERE price < 20;",
    ssql: `पुस्तकेभ्यः
    मूल्यम् २० इत्यस्मात् न्यूनम् अस्ति इति नियमेन
    शीर्षकम् च मूल्यम् च
आह्रियेताम् ।`,
  },
  join_agg: {
    title: "2. Multi Table Join & Aggregation • आपणैः सह ... योगः",
    description:
      "Compute total sales revenue per store using Instrumental plural आपणैः सह for JOIN stores, Genitive आपणस्य सङ्ख्या for stores.id, and Feminine समा अस्ति.",
    sample_sql:
      "SELECT stores.name, SUM(sales.total_amount)\nFROM sales\nJOIN stores ON sales.store_id = stores.id\nGROUP BY stores.name;",
    ssql: `विक्रयणेभ्यः
    आपणैः सह विक्रयणस्य आपणसङ्ख्या आपणस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    आपणस्य नाम इति समूहेन
    आपणस्य नाम च विक्रयणस्य सकलमूल्यस्य योगः च
आह्रियेताम् ।`,
  },
  left_join: {
    title: "3. Left Join & NULL Check • वाममेलनेन + रिक्ता अस्ति",
    description:
      "Find books without reviews: इति वाममेलनेन performs a LEFT JOIN with समीक्षाभिः and रिक्ता अस्ति checks IS NULL with Feminine concordance.",
    sample_sql:
      "SELECT books.id, books.title\nFROM books\nLEFT JOIN reviews ON books.id = reviews.book_id\nWHERE reviews.id IS NULL;",
    ssql: `पुस्तकेभ्यः
    समीक्षाभिः सह पुस्तकस्य सङ्ख्या समीक्षायाः पुस्तकसङ्ख्या इत्यनया समा अस्ति इति वाममेलनेन
    समीक्षायाः सङ्ख्या रिक्ता अस्ति इति नियमेन
    पुस्तकस्य सङ्ख्या च पुस्तकस्य शीर्षकम् च
आह्रियेताम् ।`,
  },
  having_order: {
    title: "4. GROUP BY, HAVING & ORDER BY • समूहनियमेन + अवरोहः",
    description:
      "Find stores with total sales over 30 ordered descending: योगः is Masculine, so HAVING uses Masculine अधिकः अस्ति.",
    sample_sql:
      "SELECT stores.name, SUM(sales.total_amount) AS store_revenue\nFROM stores\nJOIN sales ON stores.id = sales.store_id\nGROUP BY stores.name\nHAVING SUM(sales.total_amount) > 30\nORDER BY SUM(sales.total_amount) DESC;",
    ssql: `आपणेभ्यः
    विक्रयणैः सह आपणस्य सङ्ख्या विक्रयणस्य आपणसङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    आपणस्य नाम इति समूहेन
    विक्रयणस्य सकलमूल्यस्य योगः ३० इत्यस्मात् अधिकः अस्ति इति समूहनियमेन
    विक्रयणस्य सकलमूल्यस्य योगः अवरोहः इति क्रमेण
    आपणस्य नाम च विक्रयणस्य सकलमूल्यस्य योगः आपणराजस्वम् इति आख्या च
आह्रियेताम् ।`,
  },
  window_rank: {
    title: "5. Window Function Ranking • यत्र भाषा आधारेण ... श्रेणी",
    description:
      "Rank books by price descending per language using the window clause यत् श्रेणी यत्र भाषा आधारेण विभाजनम् मूल्यम् अनुसारम् अवरोहक्रमेण इति and Plural verb आह्रियन्ताम्.",
    sample_sql: "SELECT title, language, price\nFROM books\nORDER BY price DESC\nLIMIT 5;",
    ssql: `पुस्तकेभ्यः
    शीर्षकम् च
    भाषा च
    मूल्यम् च
    श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण मूल्यश्रेणी इति आख्या च
आह्रियन्ताम् ।`,
  },
  cte_join: {
    title: "6. Common Table Expression • CTE इति आख्या + Join",
    description:
      "Count reviews per book in a Common Table Expression पुस्तकसमीक्षासारः and join back to पुस्तकेभ्यः via Instrumental Singular पुस्तकसमीक्षासारेण सह.",
    sample_sql: "SELECT book_id, COUNT(id) AS review_count\nFROM reviews\nGROUP BY book_id;",
    ssql: `पुस्तकसमीक्षासारः इति आख्या पुस्तकसमीक्षासार इति पुंलिङ्गम् एकवचनम्
    समीक्षाभ्यः
    पुस्तकसङ्ख्या इति समूहेन
    पुस्तकसङ्ख्या च सङ्ख्यायाः गणना समीक्षागणना इति आख्या च
आह्रियेताम् ।

पुस्तकेभ्यः
    पुस्तकसमीक्षासारेण सह पुस्तकस्य सङ्ख्या पुस्तकसमीक्षासारस्य पुस्तकसङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    पुस्तकस्य शीर्षकम् च पुस्तकसमीक्षासारस्य समीक्षागणना च
आह्रियेताम् ।`,
  },
  pure_udf: {
    title: "7. Pure UDF with दीयेते & प्रदाय • शुद्धम् कार्यम्",
    description:
      "Define a two parameter Pure UDF करसहितमूल्यम् whose parameters are given with the dual passive दीयेते, and invoke it in SELECT using मूल्यम् च ५ च प्रदाय.",
    sample_sql: "SELECT title, price\nFROM books\nWHERE price > 25\nORDER BY price DESC\nLIMIT 5;",
    ssql: `करसहितमूल्यम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
    मूल्यम् इति वास्तविकः च करः इति वास्तविकः च दीयेते
    मूल्यम् च करः च इत्यनयोः योगः इति उत्तरम् प्रतिदीयताम्
इति कार्यम् ।

पुस्तकेभ्यः
    शीर्षकम् च
    मूल्यम् च ५ च प्रदाय करसहितमूल्यम् सकलमूल्यम् इति आख्या च
आह्रियेताम् ।`,
  },
  concordance_err: {
    title: "8. Strict Grammar Validator Demo • Gender & Verb Number Error",
    description:
      "Intentional concordance error: try fixing Masculine अधिकः to Neuter अधिकम् for मूल्यम्, and Singular आह्रियताम् to Dual आह्रियेताम् for 2 columns.",
    sample_sql: "SELECT title, price\nFROM books\nWHERE price > 20;",
    ssql: `पुस्तकेभ्यः
    मूल्यम् २० इत्यस्मात् अधिकः अस्ति इति नियमेन
    शीर्षकम् च मूल्यम् च
आह्रियताम् ।`,
  },
};

/* ==========================================================================
   Categorized All-Keywords Palette & Grammar Dictionary
   ========================================================================== */
const CHIP_CATEGORIES = {
  clauses: [
    { label: "।", insert: "।", sql: "; (Statement Terminator)", cat: "Punctuation" },
    { label: "च", insert: "च ", sql: "AND / ,", cat: "Conjunction" },
    { label: "वा", insert: "वा ", sql: "OR", cat: "Conjunction" },
    { label: "यत्", insert: "यत् ", sql: "( Group / Window opener", cat: "Particle" },
    { label: "इति", insert: "इति ", sql: "AS / Clause closer", cat: "Particle" },
    { label: "यत्र", insert: "यत्र ", sql: "WHERE / AS / OVER (", cat: "Particle" },
    { label: "इति नियमेन", insert: "इति नियमेन ", sql: "WHERE ...", cat: "Clause" },
    { label: "इति समूहेन", insert: "इति समूहेन ", sql: "GROUP BY ...", cat: "Clause" },
    { label: "इति समूहनियमेन", insert: "इति समूहनियमेन ", sql: "HAVING ...", cat: "Clause" },
    { label: "आरोहः इति क्रमेण", insert: "आरोहः इति क्रमेण ", sql: "ORDER BY ... ASC", cat: "Clause" },
    { label: "अवरोहः इति क्रमेण", insert: "अवरोहः इति क्रमेण ", sql: "ORDER BY ... DESC", cat: "Clause" },
    { label: "सह", insert: "सह ", sql: "JOIN <table>", cat: "Postposition" },
    { label: "इति मेलनेन", insert: "इति मेलनेन ", sql: "INNER JOIN ON", cat: "Clause" },
    { label: "इति वाममेलनेन", insert: "इति वाममेलनेन ", sql: "LEFT JOIN ON", cat: "Clause" },
    { label: "इति दक्षिणमेलनेन", insert: "इति दक्षिणमेलनेन ", sql: "RIGHT JOIN ON", cat: "Clause" },
    { label: "इति पूर्णमेलनेन", insert: "इति पूर्णमेलनेन ", sql: "FULL JOIN ON", cat: "Clause" },
    { label: "इति मित्या", insert: "इति मित्या ", sql: "LIMIT <n>", cat: "Clause" },
    { label: "इति मित्या", insert: "इति मित्या ", sql: "LIMIT <n> (with अपसारेण)", cat: "Clause" },
    { label: "अपसारेण", insert: "अपसारेण ", sql: "OFFSET <m>", cat: "Clause" },
    { label: "पुनरुक्तिम् विना", insert: "पुनरुक्तिम् विना ", sql: "DISTINCT", cat: "Modifier" },
    { label: "सर्वम्", insert: "सर्वम् ", sql: "SELECT *", cat: "Pronoun" },
    { label: "मध्ये अस्ति", insert: "मध्ये अस्ति ", sql: "BETWEEN low AND high", cat: "Clause" },
    { label: "सङ्गमः", insert: "सङ्गमः ", sql: "UNION", cat: "Clause" },
    { label: "सर्वसङ्गमः", insert: "सर्वसङ्गमः ", sql: "UNION ALL", cat: "Clause" },
    { label: "तस्मिन्", insert: "तस्मिन् ", sql: "THEN", cat: "Conditional" },
    { label: "अन्यस्मिन्", insert: "अन्यस्मिन् ", sql: "ELSE", cat: "Conditional" },
  ],
  verbs: [
    { label: "आह्रियताम् ।", insert: "आह्रियताम् ।", sql: "SELECT (1 col, एकवचनम्)", cat: "Verb" },
    { label: "आह्रियेताम् ।", insert: "आह्रियेताम् ।", sql: "SELECT (2 cols, द्विवचनम्)", cat: "Verb" },
    { label: "आह्रियन्ताम् ।", insert: "आह्रियन्ताम् ।", sql: "SELECT (3+ cols, बहुवचनम्)", cat: "Verb" },
    { label: "रच्यताम्", insert: "रच्यताम् ", sql: "CREATE TABLE / VIEW / INDEX", cat: "Verb" },
    { label: "निवेश्यताम् ।", insert: "निवेश्यताम् ।", sql: "INSERT INTO ...", cat: "Verb" },
    { label: "परिवर्त्यताम् ।", insert: "परिवर्त्यताम् ।", sql: "UPDATE ... SET", cat: "Verb" },
    { label: "निष्कास्यताम् ।", insert: "निष्कास्यताम् ।", sql: "DELETE FROM ...", cat: "Verb" },
    { label: "लुप्यताम् ।", insert: "लुप्यताम् ।", sql: "DROP TABLE ...", cat: "Verb" },
    { label: "रच्यताम्", insert: "रच्यताम् ", sql: "CREATE FUNCTION (UDF)", cat: "Verb" },
    { label: "प्रतिदीयते", insert: "प्रतिदीयते ", sql: "RETURNS <type>", cat: "Verb" },
    { label: "प्रतिदीयताम्", insert: "प्रतिदीयताम् ", sql: "RETURN <expr>", cat: "Verb" },
    { label: "अभिधीयताम् ।", insert: "अभिधीयताम् ।", sql: "EXPLAIN QUERY PLAN", cat: "Verb" },
    { label: "दीयते", insert: "दीयते ", sql: "1 Parameter (एकवचनम्)", cat: "Verb" },
    { label: "दीयेते", insert: "दीयेते ", sql: "2 Parameters (द्विवचनम्)", cat: "Verb" },
    { label: "दीयन्ते", insert: "दीयन्ते ", sql: "3+ Parameters (बहुवचनम्)", cat: "Verb" },
  ],
  comparisons: [
    { label: "अधिकः (M)", insert: "अधिकः ", sql: "> (Masculine)", cat: "Comparison" },
    { label: "अधिका (F)", insert: "अधिका ", sql: "> (Feminine)", cat: "Comparison" },
    { label: "अधिकम् (N)", insert: "अधिकम् ", sql: "> (Neuter)", cat: "Comparison" },
    { label: "न्यूनः (M)", insert: "न्यूनः ", sql: "< (Masculine)", cat: "Comparison" },
    { label: "न्यूना (F)", insert: "न्यूना ", sql: "< (Feminine)", cat: "Comparison" },
    { label: "न्यूनम् (N)", insert: "न्यूनम् ", sql: "< (Neuter)", cat: "Comparison" },
    { label: "समः अस्ति (M)", insert: "समः अस्ति ", sql: "= (Masculine)", cat: "Comparison" },
    { label: "समा अस्ति (F)", insert: "समा अस्ति ", sql: "= (Feminine)", cat: "Comparison" },
    { label: "समम् अस्ति (N)", insert: "समम् अस्ति ", sql: "= (Neuter)", cat: "Comparison" },
    { label: "रिक्तः अस्ति (M)", insert: "रिक्तः अस्ति ", sql: "IS NULL (Masculine)", cat: "Comparison" },
    { label: "रिक्ता अस्ति (F)", insert: "रिक्ता अस्ति ", sql: "IS NULL (Feminine)", cat: "Comparison" },
    { label: "रिक्तम् अस्ति (N)", insert: "रिक्तम् अस्ति ", sql: "IS NULL (Neuter)", cat: "Comparison" },
    { label: "रिक्तः नास्ति (M)", insert: "रिक्तः नास्ति ", sql: "IS NOT NULL (Masculine)", cat: "Comparison" },
    { label: "रिक्ता नास्ति (F)", insert: "रिक्ता नास्ति ", sql: "IS NOT NULL (Feminine)", cat: "Comparison" },
    { label: "रिक्तम् नास्ति (N)", insert: "रिक्तम् नास्ति ", sql: "IS NOT NULL (Neuter)", cat: "Comparison" },
    { label: "इत्यस्मात् (M/N)", insert: "इत्यस्मात् ", sql: "Than (Ablative M/N)", cat: "Pronoun" },
    { label: "इत्यस्याः (F)", insert: "इत्यस्याः ", sql: "Than / Of (Feminine)", cat: "Pronoun" },
    { label: "इत्यनेन (M/N)", insert: "इत्यनेन ", sql: "With / Equal to (Instrumental M/N)", cat: "Pronoun" },
    { label: "इत्यनया (F)", insert: "इत्यनया ", sql: "With / Equal to (Instrumental F)", cat: "Pronoun" },
    { label: "इत्यस्य", insert: "इत्यस्य ", sql: "Of (Genitive Singular)", cat: "Pronoun" },
    { label: "इत्यनयोः", insert: "इत्यनयोः ", sql: "Of Both (Genitive Dual)", cat: "Pronoun" },
  ],
  aggregates: [
    { label: "योगः", insert: "योगः ", sql: "SUM(x) / (a + b) [Masculine]", cat: "Aggregate" },
    { label: "माध्यम्", insert: "माध्यम् ", sql: "AVG(x)", cat: "Aggregate" },
    { label: "गणना", insert: "गणना ", sql: "COUNT(x) [Feminine]", cat: "Aggregate" },
    { label: "महत्तमम्", insert: "महत्तमम् ", sql: "MAX(x) [Neuter]", cat: "Aggregate" },
    { label: "लघुतमम्", insert: "लघुतमम् ", sql: "MIN(x) [Neuter]", cat: "Aggregate" },
    { label: "गुणनम्", insert: "गुणनम् ", sql: "Multiplied by (*)", cat: "Math" },
    { label: "विभज्य लब्धम्", insert: "विभज्य लब्धम् ", sql: "Divided by (/)", cat: "Math" },
    { label: "विशोध्य अवशिष्टम्", insert: "विशोध्य अवशिष्टम् ", sql: "Minus (-)", cat: "Math" },
    { label: "विशोध्य अवशिष्टम्", insert: "विशोध्य अवशिष्टम् ", sql: "Subtracting (-)", cat: "Math" },
    { label: "विशिष्टस्य (M/N)", insert: "विशिष्टस्य ", sql: "COUNT(DISTINCT x) [M/N]", cat: "Aggregate" },
    { label: "विशिष्टायाः (F)", insert: "विशिष्टायाः ", sql: "COUNT(DISTINCT x) [F]", cat: "Aggregate" },
  ],
  windows_udfs: [
    { label: "दीयते", insert: "दीयते ", sql: "UDF parameter (1; दीयेते 2, दीयन्ते 3+)", cat: "UDF" },
    { label: "प्रदाय", insert: "प्रदाय ", sql: "Supplying UDF arguments", cat: "UDF" },
    { label: "इति शुद्धम् कार्यम्", insert: "इति शुद्धम् कार्यम् रच्यताम् यत्र ", sql: "CREATE PURE FUNCTION", cat: "UDF" },
    { label: "इति उत्तरम् प्रतिदीयताम्", insert: "इति उत्तरम् प्रतिदीयताम् ", sql: "RETURN <result>", cat: "UDF" },
    { label: "इति कार्यम् ।", insert: "इति कार्यम् ।", sql: "END FUNCTION", cat: "UDF" },
    { label: "यत् श्रेणी यत्र", insert: "यत् श्रेणी यत्र ", sql: "RANK() OVER (", cat: "Window" },
    { label: "सघनश्रेणी", insert: "सघनश्रेणी ", sql: "DENSE_RANK()", cat: "Window" },
    { label: "पङ्क्तिसङ्ख्या", insert: "पङ्क्तिसङ्ख्या ", sql: "ROW_NUMBER()", cat: "Window" },
    { label: "पूर्वपङ्क्तेः", insert: "पूर्वपङ्क्तेः ", sql: "LAG(col)", cat: "Window" },
    { label: "अनन्तरपङ्क्तेः", insert: "अनन्तरपङ्क्तेः ", sql: "LEAD(col)", cat: "Window" },
    { label: "चतुर्थांशाङ्कः", insert: "चतुर्थांशाङ्कः ", sql: "NTILE(4)", cat: "Window" },
    { label: "भागाङ्कः", insert: "भागेषु भागाङ्कः ", sql: "NTILE(n)", cat: "Window" },
    { label: "आधारेण विभाजनम्", insert: "आधारेण विभाजनम् ", sql: "PARTITION BY ...", cat: "Window" },
    { label: "अनुसारम् आरोहक्रमेण", insert: "अनुसारम् आरोहक्रमेण ", sql: "ORDER BY ... ASC (Window)", cat: "Window" },
    { label: "अनुसारम् अवरोहक्रमेण", insert: "अनुसारम् अवरोहक्रमेण ", sql: "ORDER BY ... DESC (Window)", cat: "Window" },
    { label: "इति सञ्ज्ञा", insert: "इति सञ्ज्ञा ", sql: "WITH <cte> AS", cat: "CTE" },
    { label: "आवर्तिका", insert: "आवर्तिका ", sql: "WITH RECURSIVE", cat: "CTE" },
  ],
  ddl_types: [
    { label: "इति पटलम्", insert: "इति पटलम् ", sql: "TABLE", cat: "DDL" },
    { label: "इति दृश्यम्", insert: "इति दृश्यम् ", sql: "VIEW", cat: "DDL" },
    { label: "इति सूची", insert: "इति सूची ", sql: "INDEX", cat: "DDL" },
    { label: "इति आख्या", insert: "इति आख्या ", sql: "Physical SQL Name / Stem", cat: "DDL" },
    { label: "पूर्णाङ्कः", insert: "पूर्णाङ्कः ", sql: "INTEGER", cat: "Type" },
    { label: "वास्तविकः", insert: "वास्तविकः ", sql: "REAL / NUMERIC", cat: "Type" },
    { label: "वाक्यम्", insert: "वाक्यम् ", sql: "TEXT / VARCHAR", cat: "Type" },
    { label: "दिनाङ्कः", insert: "दिनाङ्कः ", sql: "DATE", cat: "Type" },
    { label: "पुंलिङ्गम्", insert: "पुंलिङ्गम् ", sql: "Masculine Gender", cat: "Gender" },
    { label: "स्त्रीलिङ्गम्", insert: "स्त्रीलिङ्गम् ", sql: "Feminine Gender", cat: "Gender" },
    { label: "नपुंसकलिङ्गम्", insert: "नपुंसकलिङ्गम् ", sql: "Neuter Gender", cat: "Gender" },
    { label: "एकवचनम्", insert: "एकवचनम् ", sql: "Singular Number", cat: "Number" },
    { label: "द्विवचनम्", insert: "द्विवचनम् ", sql: "Dual Number", cat: "Number" },
    { label: "बहुवचनम्", insert: "बहुवचनम् ", sql: "Plural Number", cat: "Number" },
    { label: "प्राथमिकी", insert: "प्राथमिकी ", sql: "PRIMARY KEY (F)", cat: "Constraint" },
    { label: "क्रमिका", insert: "क्रमिका ", sql: "AUTOINCREMENT (F)", cat: "Constraint" },
    { label: "अनिवार्यम्", insert: "अनिवार्यम् ", sql: "NOT NULL (N)", cat: "Constraint" },
    { label: "अद्वितीयम्", insert: "अद्वितीयम् ", sql: "UNIQUE (N)", cat: "Constraint" },
    { label: "बद्धा", insert: "बद्धा ", sql: "FOREIGN KEY (F)", cat: "Constraint" },
  ],
};

// Build a fast lookup map of all SSQL keywords for hover help & syntax highlighting
const KEYWORD_LOOKUP = new Map();
for (const items of Object.values(CHIP_CATEGORIES)) {
  for (const item of items) {
    const firstWord = item.insert.trim().split(/\s+/)[0];
    if (firstWord && !KEYWORD_LOOKUP.has(firstWord)) {
      KEYWORD_LOOKUP.set(firstWord, item);
    }
    KEYWORD_LOOKUP.set(item.insert.trim(), item);
  }
}

const VERB_SET = new Set([
  "आह्रियताम्", "आह्रियेताम्", "आह्रियन्ताम्", "रच्यताम्", "निवेश्यताम्",
  "निवेश्येताम्", "निवेश्यन्ताम्", "परिवर्त्यताम्", "परिवर्त्येताम्", "परिवर्त्यन्ताम्", "निष्कास्यताम्", "लुप्यताम्", "रच्यताम्", "प्रतिदीयते",
  "प्रतिदीयताम्", "अभिधीयताम्", "दीयते", "दीयेते", "दीयन्ते", "आहरणम्", "योज्यताम्"
]);

const CLAUSE_SET = new Set([
  "इति", "यत्र", "यत्", "च", "वा", "चेत्", "तर्हि", "अन्यथा", "नियमेन",
  "समूहेन", "समूहनियमेन", "क्रमेण", "सह", "मेलनेन", "वाममेलनेन",
  "दक्षिणमेलनेन", "पूर्णमेलनेन", "सर्वमेलनम्", "आरोहः", "अवरोहः",
  "आरोहक्रमेण", "अवरोहक्रमेण", "सीमा", "मित्या", "अपसारेण", "विशिष्टम्",
  "विशिष्टस्य", "विशिष्टायाः", "विशिष्टस्य", "विशिष्टायाः", "सर्वम्",
  "मध्ये", "सङ्गमः", "सर्वसङ्गमः", "प्रतिच्छेदः", "वर्जनम्",
  "प्रदाय", "आधारेण", "विभाजनम्", "अनुसारम्", "उत्तरम्"
]);

const COMP_SET = new Set([
  "अधिकः", "अधिका", "अधिकम्", "न्यूनः", "न्यूना", "न्यूनम्",
  "समः", "समा", "समम्", "समानः", "समाना", "समानम्",
  "रिक्तः", "रिक्ता", "रिक्तम्", "अरिक्तः", "अरिक्ता", "अरिक्तम्",
  "अस्ति", "नास्ति", "इत्यस्मात्", "इत्यस्याः", "इत्यनेन", "इत्यनया", "इत्यस्य", "इत्यनयोः"
]);

const AGG_SET = new Set([
  "योगः", "माध्यम्", "गणना", "महत्तमम्", "लघुतमम्", "अधिकतमम्", "न्यूनतमम्",
  "गुणितम्", "गुणनम्", "भाजितम्", "विभज्य", "लब्धम्", "विशोध्य", "अवशिष्टम्", "विशोध्य",
  "श्रेणी", "सघनश्रेणी", "पङ्क्तिसङ्ख्या", "पूर्वपङ्क्तेः", "अनन्तरपङ्क्तेः", "चतुर्थांशाङ्कः", "भागाङ्कः"
]);

const TYPE_SET = new Set([
  "पटलम्", "दृश्यम्", "सूची", "शुद्धम्", "कार्यम्", "सञ्ज्ञा", "आवर्तिका", "आख्या",
  "पूर्णाङ्कः", "वास्तविकः", "वाक्यम्", "दिनाङ्कः", "कालमुद्रा",
  "पुंलिङ्गम्", "स्त्रीलिङ्गम्", "नपुंसकलिङ्गम्", "एकवचनम्", "द्विवचनम्", "बहुवचनम्",
  "प्राथमिकः", "प्राथमिकी", "प्राथमिकम्", "क्रमिकः", "क्रमिका", "क्रमिकम्",
  "अनिवार्यः", "अनिवार्या", "अनिवार्यम्", "अद्वितीयः", "अद्वितीया", "अद्वितीयम्",
  "बद्धः", "बद्धा", "बद्धम्"
]);

/* ==========================================================================
   Application State
   ========================================================================== */
let wasmReady = false;
let sqlJsEngine = null;
let sqlDb = null;

let corpusData = null; // Loaded from data/corpus.json

// The featured scenarios, tutorials and exercises run on the University
// Bookstore. Its schema and its setup SQL (compiled from schema.सं and
// seed.सं by build_site.py) come from the corpus, so the playground never
// carries a hand-kept copy of either.
const BOOKSTORE_PROJECT_ID = "university_bookstore/SAMSQL";
function bookstoreProject() {
  return corpusData?.projects?.find((p) => p.id === BOOKSTORE_PROJECT_ID) || null;
}
function bookstoreSchemaSsql() {
  return bookstoreProject()?.schema_ssql || "";
}
function bookstoreSetupSql() {
  return bookstoreProject()?.setup_sql || "";
}
let contentData = null; // Loaded from data/content.json
let activeProjectId = "__featured__";
let activeChipCategory = "clauses";
let lastCompileResult = null;
let cachedSchemaTables = [];
let debounceTimer = null;
let activeTutorialId = "track-a-01";
let confirmedUnfilteredDmlQuery = "";
let pendingDeepLinkWarning = "";
let isResettingDb = false;
const completedExercises = new Set();
const projectDbSnapshots = new Map();
const projectSchemaDrafts = new Map();
const projectQueryDrafts = new Map();

try {
  const savedExercises = JSON.parse(localStorage.getItem("ssql-completed-exercises") || "[]");
  if (Array.isArray(savedExercises)) {
    savedExercises.forEach((id) => completedExercises.add(id));
  }
  const savedTutorial =
    sessionStorage.getItem("ssql-active-tutorial") ||
    localStorage.getItem("ssql-active-tutorial");
  if (savedTutorial) {
    activeTutorialId = savedTutorial;
  }
} catch (_) {}

function saveCompletedExercises() {
  try {
    localStorage.setItem(
      "ssql-completed-exercises",
      JSON.stringify(Array.from(completedExercises))
    );
  } catch (_) {}
}

function saveWorkspaceStateToStorage() {
  try {
    sessionStorage.setItem("ssql-active-project", activeProjectId);
    if (exampleSelect?.value) {
      sessionStorage.setItem("ssql-active-query", exampleSelect.value);
    }
    if (ssqlEditor) {
      sessionStorage.setItem("ssql-draft-query", ssqlEditor.value);
      sessionStorage.setItem(
        "ssql-draft-query-key",
        `${activeProjectId}::${exampleSelect?.value || ""}`
      );
    }
    if (schemaEditor) {
      sessionStorage.setItem("ssql-draft-schema", schemaEditor.value);
      sessionStorage.setItem("ssql-draft-schema-project", activeProjectId);
    }
    if (dialectSelect?.value) {
      sessionStorage.setItem("ssql-dialect", dialectSelect.value);
      localStorage.setItem("ssql-dialect", dialectSelect.value);
    }
    sessionStorage.setItem("ssql-active-tutorial", activeTutorialId);
    localStorage.setItem("ssql-active-tutorial", activeTutorialId);
  } catch (_) {}
}

function syncUrlQueryParams() {
  try {
    const url = new URL(window.location.href);
    if (activeProjectId && activeProjectId !== "__featured__") {
      url.searchParams.set("project", activeProjectId);
    } else {
      url.searchParams.delete("project");
    }
    const qVal = exampleSelect?.value;
    if (qVal && qVal !== "__custom_snippet__" && !(activeProjectId === "__featured__" && qVal === "filter")) {
      url.searchParams.set("query", qVal);
    } else {
      url.searchParams.delete("query");
    }
    window.history.replaceState({}, "", url.toString());
  } catch (_) {}
}

let acItems = [];
let acSelectedIndex = 0;

/* ==========================================================================
   DOM References
   ========================================================================== */
const runtimeStatus = document.getElementById("runtime-status");
const projectSelect = document.getElementById("project-select");
const exampleSelect = document.getElementById("example-select");
const dialectSelect = document.getElementById("dialect-select");
const activeProjectBadge = document.getElementById("active-project-badge");

const ssqlEditor = document.getElementById("ssql-editor");
const ssqlHighlight = document.getElementById("ssql-highlight");
const ssqlGutter = document.getElementById("ssql-gutter");

const schemaEditor = document.getElementById("schema-editor");
const schemaHighlight = document.getElementById("schema-highlight");
const schemaGutter = document.getElementById("schema-gutter");

const sqlInputEditor = document.getElementById("sql-input-editor");
const sqlInputHighlight = document.getElementById("sql-input-highlight");
const sqlInputGutter = document.getElementById("sql-input-gutter");

const exerciseEditor = document.getElementById("exercise-editor");
const exerciseHighlight = document.getElementById("exercise-highlight");
const exerciseGutter = document.getElementById("exercise-gutter");

const scenarioDescription = document.getElementById("scenario-description");
const chipPaletteItems = document.getElementById("chip-palette-items");

const hoverBarContent = document.getElementById("hover-bar-content");
const autocompletePopup = document.getElementById("editor-autocomplete");
const hoverTooltip = document.getElementById("editor-hover-tooltip");

const compileErrorBanner = document.getElementById("compile-error-banner");
const generatedSqlCode = document.getElementById("generated-sql-code");
const generatedSqlGutter = document.getElementById("generated-sql-gutter");
const sqlDialectLabel = document.getElementById("sql-dialect-label");
const execStatusLabel = document.getElementById("exec-status-label");
const execRowCount = document.getElementById("exec-row-count");
const resultsThead = document.getElementById("results-thead");
const resultsTbody = document.getElementById("results-tbody");
const adhyetaTbody = document.getElementById("adhyeta-tbody");
const diagnosticsList = document.getElementById("diagnostics-list");
const schemaCardsContainer = document.getElementById("schema-cards-container");
const badgeTokens = document.getElementById("badge-tokens");
const badgeDiags = document.getElementById("badge-diags");

/* ==========================================================================
   Theme, Font Pairing, and Font Size Scaling (`Aa` Appearance Control)
   ========================================================================== */
function setupThemeToggle() {
  // Conditionally load Invoker Commands & Popover polyfills if native support is missing
  if (typeof HTMLButtonElement !== "undefined" && !("commandForElement" in HTMLButtonElement.prototype)) {
    import("https://esm.run/invokers-polyfill@1.0.4").catch(() => {});
  }
  if (typeof HTMLElement !== "undefined" && !("popover" in HTMLElement.prototype)) {
    import("https://unpkg.com/@oddbird/popover-polyfill@0.7.3/dist/popover.min.js").catch(() => {});
  }

  const root = document.documentElement;
  const meta = document.querySelector('meta[name="color-scheme"]');
  const themeBtn = document.getElementById("theme-toggle");
  const scaleBadge = document.getElementById("font-scale-badge");
  const appearanceBtn = document.getElementById("btn-appearance");
  const appearancePopover = document.getElementById("appearance-popover");

  // Fallback click handler for browsers where commandfor="appearance-popover" is not native yet
  if (appearanceBtn && appearancePopover && !("commandForElement" in HTMLButtonElement.prototype)) {
    appearanceBtn.addEventListener("click", () => {
      if (typeof appearancePopover.togglePopover === "function") {
        appearancePopover.togglePopover();
      }
    });
  }

  const THEME_SCHEMES = {
    bhojpatra: "light",
    sphatika: "light",
    tamrapatta: "light",
    nisha: "dark",
    tamala: "dark",
  };

  function applyScale(scaleNum) {
    const clamped = Math.max(85, Math.min(155, Math.round(scaleNum)));
    root.style.setProperty("--ui-scale", `${clamped}%`);
    if (scaleBadge) scaleBadge.textContent = `${clamped}%`;
    document.querySelectorAll(".scale-preset-btn").forEach((b) => {
      b.classList.toggle("active", Number(b.dataset.scale) === clamped);
    });
    try {
      localStorage.setItem("ssql-scale", String(clamped));
    } catch (_) {}
  }

  function applyTheme(themeId) {
    const scheme = THEME_SCHEMES[themeId] || "light";
    root.dataset.theme = themeId;
    if (meta) meta.content = scheme;
    document.querySelectorAll(".theme-swatch-btn").forEach((b) => {
      b.classList.toggle("active", b.dataset.themeChoice === themeId);
    });
    try {
      localStorage.setItem("ssql-theme", themeId);
      localStorage.setItem("color-scheme", scheme);
      if (scheme === "light") localStorage.setItem("ssql-last-light", themeId);
      if (scheme === "dark") localStorage.setItem("ssql-last-dark", themeId);
    } catch (_) {}
  }

  function applyFont(fontId) {
    root.dataset.font = fontId;
    document.querySelectorAll(".font-choice-btn").forEach((b) => {
      b.classList.toggle("active", b.dataset.fontChoice === fontId);
    });
    try {
      localStorage.setItem("ssql-font", fontId);
    } catch (_) {}
  }

  // Hydrate initial active states
  const currentScale = Number(localStorage.getItem("ssql-scale") || 105);
  const currentTheme = localStorage.getItem("ssql-theme") || "bhojpatra";
  const currentFont = localStorage.getItem("ssql-font") || "sans";
  applyScale(currentScale);
  document.querySelectorAll(".theme-swatch-btn").forEach((b) => {
    b.classList.toggle("active", b.dataset.themeChoice === currentTheme);
  });
  document.querySelectorAll(".font-choice-btn").forEach((b) => {
    b.classList.toggle("active", b.dataset.fontChoice === currentFont);
  });

  // Wire up font size buttons to step through all preset scale levels
  const presetScales = Array.from(document.querySelectorAll(".scale-preset-btn"))
    .map((b) => Number(b.dataset.scale))
    .filter((n) => !Number.isNaN(n));
  const SCALE_STEPS = Array.from(new Set([85, 90, ...presetScales, 155])).sort(
    (a, b) => a - b
  );
  document.getElementById("btn-font-dec")?.addEventListener("click", () => {
    const cur = Math.round(parseFloat(root.style.getPropertyValue("--ui-scale")) || 105);
    const prevSteps = SCALE_STEPS.filter((s) => s < cur);
    const nextVal = prevSteps.length > 0 ? prevSteps[prevSteps.length - 1] : SCALE_STEPS[0];
    applyScale(nextVal);
  });
  document.getElementById("btn-font-inc")?.addEventListener("click", () => {
    const cur = Math.round(parseFloat(root.style.getPropertyValue("--ui-scale")) || 105);
    const nextSteps = SCALE_STEPS.filter((s) => s > cur);
    const nextVal =
      nextSteps.length > 0 ? nextSteps[0] : SCALE_STEPS[SCALE_STEPS.length - 1];
    applyScale(nextVal);
  });
  document.getElementById("btn-font-reset")?.addEventListener("click", () => {
    applyScale(105);
  });
  document.getElementById("btn-font-reset-inline")?.addEventListener("click", () => {
    applyScale(105);
  });
  document.querySelectorAll(".scale-preset-btn").forEach((btn) => {
    btn.addEventListener("click", () => applyScale(Number(btn.dataset.scale)));
  });

  // Wire up theme swatches
  document.querySelectorAll(".theme-swatch-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      if (btn.dataset.themeChoice) applyTheme(btn.dataset.themeChoice);
    });
  });

  // Wire up font pairings
  document.querySelectorAll(".font-choice-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      if (btn.dataset.fontChoice) applyFont(btn.dataset.fontChoice);
    });
  });

  // Quick 1-click light/dark toggle button
  if (themeBtn && meta) {
    themeBtn.addEventListener("click", () => {
      const isCurrentlyDark =
        meta.content === "dark" ||
        (meta.content === "light dark" &&
          window.matchMedia("(prefers-color-scheme: dark)").matches);
      if (isCurrentlyDark) {
        const nextLight = localStorage.getItem("ssql-last-light") || "bhojpatra";
        applyTheme(nextLight);
      } else {
        const nextDark = localStorage.getItem("ssql-last-dark") || "nisha";
        applyTheme(nextDark);
      }
    });
  }
}

/* ==========================================================================
   Scrollspy Navigation Indicator (scroll-target-group + IntersectionObserver)
   ========================================================================== */
function setupScrollSpy() {
  const navLinks = document.querySelectorAll('.site-nav a[href^="#"]');
  const sections = document.querySelectorAll("main section[id]");
  if (!navLinks.length || !sections.length) return;

  const supportsScrollTargetGroup =
    typeof CSS !== "undefined" && CSS.supports("scroll-target-group: auto");

  const setActiveByHref = (targetHref) => {
    navLinks.forEach((link) => {
      const isActive = link.getAttribute("href") === targetHref;
      if (!supportsScrollTargetGroup) {
        link.classList.toggle(":target-current", isActive);
      } else {
        link.classList.remove(":target-current");
      }
      link.setAttribute("aria-current", isActive ? "true" : "false");
    });
  };

  navLinks.forEach((link) => {
    link.addEventListener("click", () => {
      setActiveByHref(link.getAttribute("href"));
    });
  });

  if (supportsScrollTargetGroup) {
    const syncAriaCurrent = () => {
      const currentLink = document.querySelector(".site-nav a:target-current");
      navLinks.forEach((link) => {
        link.classList.remove(":target-current");
        link.setAttribute("aria-current", link === currentLink ? "true" : "false");
      });
    };
    const scheduleSync = () => {
      syncAriaCurrent();
      requestAnimationFrame(() => {
        syncAriaCurrent();
        requestAnimationFrame(syncAriaCurrent);
      });
      setTimeout(syncAriaCurrent, 60);
    };
    scheduleSync();
    document.addEventListener("scroll", scheduleSync, { passive: true });
    document.addEventListener("scrollend", scheduleSync, { passive: true });
  } else if ("IntersectionObserver" in window) {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            const id = entry.target.getAttribute("id");
            setActiveByHref(`#${id}`);
          }
        });
      },
      { rootMargin: "-50% 0px -50% 0px", threshold: 0 }
    );
    sections.forEach((section) => observer.observe(section));
  }
}

/* ==========================================================================
   Tabs Setup
   ========================================================================== */
function setupTabs(tabSelector, panelSelector) {
  const tabs = document.querySelectorAll(tabSelector);
  const panels = document.querySelectorAll(panelSelector);
  tabs.forEach((tab) => {
    tab.addEventListener("click", (e) => {
      e.preventDefault();
      const savedScrollY = window.scrollY;
      tabs.forEach((t) => {
        t.classList.remove("active");
        t.setAttribute("aria-selected", "false");
      });
      panels.forEach((p) => p.setAttribute("hidden", ""));
      tab.classList.add("active");
      tab.setAttribute("aria-selected", "true");
      const targetId = tab.getAttribute("aria-controls");
      const targetPanel = document.getElementById(targetId);
      if (targetPanel) targetPanel.removeAttribute("hidden");
      if (window.scrollY !== savedScrollY) {
        window.scrollTo(window.scrollX, savedScrollY);
      }
    });
  });
}

function activateOutputTab(tabId) {
  const tab = document.getElementById(tabId);
  if (tab) tab.click();
}

/* ==========================================================================
   Database Seeding & Execution (sql.js + Local Python Server Fallback)
   ========================================================================== */
function registerSqlJsUdfs(db) {
  if (!db || typeof db.create_function !== "function") return;
  try {
    db.create_function("POWER", (a, b) =>
      a != null && b != null ? Math.pow(Number(a), Number(b)) : null
    );
    db.create_function("SQRT", (a) =>
      a != null && Number(a) >= 0 ? Math.sqrt(Number(a)) : null
    );
    db.create_function("MOD", (a, b) =>
      a != null && b != null && Number(b) !== 0 ? Number(a) % Number(b) : null
    );
    db.create_function("REGEXP", (pat, val) => {
      if (pat == null || val == null) return 0;
      try {
        return new RegExp(String(pat)).test(String(val)) ? 1 : 0;
      } catch (_) {
        return 0;
      }
    });
  } catch (_) {}
}

async function initSqliteRuntime() {
  if (typeof window.initSqlJs === "function") {
    try {
      sqlJsEngine = await window.initSqlJs({
        locateFile: (file) =>
          `https://cdnjs.cloudflare.com/ajax/libs/sql.js/1.10.3/${file}`,
      });
      await resetDatabaseForCurrentProject();
      return true;
    } catch (_) {
      // Fall back to local server `/api/execute`
    }
  }
  return false;
}

function getActiveProjectSetupSql() {
  if (activeProjectId === "__featured__") {
    return bookstoreSetupSql();
  }
  if (corpusData && corpusData.projects) {
    const proj = corpusData.projects.find((p) => p.id === activeProjectId);
    if (proj && proj.setup_sql) {
      return proj.setup_sql;
    }
  }
  return bookstoreSetupSql();
}

function getActiveProjectDefaultSchemaSsql() {
  if (activeProjectId === "__featured__") {
    return bookstoreSchemaSsql();
  }
  if (corpusData && corpusData.projects) {
    const proj = corpusData.projects.find((p) => p.id === activeProjectId);
    if (proj && proj.schema_ssql) {
      return proj.schema_ssql;
    }
  }
  return bookstoreSchemaSsql();
}

function makeDdlIdempotent(ddlSql) {
  return String(ddlSql || "")
    .replace(
      /\bCREATE\s+TABLE\s+(?!IF\s+NOT\s+EXISTS\b)/gi,
      "CREATE TABLE IF NOT EXISTS "
    )
    .replace(
      /\bCREATE\s+(UNIQUE\s+)?INDEX\s+(?!IF\s+NOT\s+EXISTS\b)/gi,
      "CREATE $1INDEX IF NOT EXISTS "
    );
}

async function applySchemaDdlNonDestructively(schemaSsql) {
  if (!wasmReady || !schemaSsql || !schemaSsql.trim()) return { ok: true };
  const res = JSON.parse(compile_ssql_wasm(schemaSsql, schemaSsql, "sqlite"));
  if (!res.ok) {
    return { ok: false, errors: res.errors || ["Schema compilation failed."] };
  }
  const rawDdl = stripGeneratedSqlHeader(res.sql || "");
  if (!rawDdl.trim()) return { ok: true };
  const idempotentDdl = makeDdlIdempotent(rawDdl);

  if (sqlDb) {
    try {
      sqlDb.run(idempotentDdl);
    } catch (_) {}

    // Also check if any existing table gained new columns in schema.सं and add them non-destructively
    const createTableRegex =
      /CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(["`]?[A-Za-z0-9_\u0900-\u097F]+["`]?)\s*\(([\s\S]*?)\)\s*;/gi;
    let m;
    while ((m = createTableRegex.exec(idempotentDdl)) !== null) {
      const tblName = m[1].replace(/^["`]|["`]$/g, "");
      const colsBlock = m[2];
      try {
        const infoRes = sqlDb.exec(`PRAGMA table_info("${tblName}");`);
        const existingCols = new Set(
          (infoRes[0]?.values || []).map((row) => String(row[1]).toLowerCase())
        );
        const colLines = colsBlock
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean);
        for (const colDef of colLines) {
          const upper = colDef.toUpperCase();
          if (
            upper.startsWith("PRIMARY ") ||
            upper.startsWith("FOREIGN ") ||
            upper.startsWith("UNIQUE ") ||
            upper.startsWith("CHECK ") ||
            upper.startsWith("CONSTRAINT ")
          ) {
            continue;
          }
          const colMatch = colDef.match(/^(["`]?[A-Za-z0-9_\u0900-\u097F]+["`]?)\s+(.*)$/);
          if (colMatch) {
            const colName = colMatch[1].replace(/^["`]|["`]$/g, "");
            if (!existingCols.has(colName.toLowerCase())) {
              const safeDef = colDef.replace(/\bPRIMARY\s+KEY\b/gi, "").replace(/\bAUTOINCREMENT\b/gi, "");
              try {
                sqlDb.run(`ALTER TABLE "${tblName}" ADD COLUMN ${safeDef};`);
              } catch (_) {}
            }
          }
        }
      } catch (_) {}
    }

    if (typeof sqlDb.export === "function") {
      try {
        projectDbSnapshots.set(activeProjectId, sqlDb.export());
        registerSqlJsUdfs(sqlDb);
      } catch (_) {}
    }
  } else {
    try {
      const ctrl = new AbortController();
      const tid = setTimeout(() => ctrl.abort(), 1500);
      await fetch("/api/execute", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ sql: idempotentDdl }),
        signal: ctrl.signal,
      });
      clearTimeout(tid);
    } catch (_) {}
  }

  return { ok: true, sql: idempotentDdl };
}

async function resetDatabaseForCurrentProject(customSetupSql = null, syncEditorSchema = true) {
  if (isResettingDb) return;
  isResettingDb = true;
  try {
    const setupSql = customSetupSql || getActiveProjectSetupSql();
    if (sqlJsEngine) {
      if (sqlDb) {
        try {
          sqlDb.close();
        } catch (_) {}
      }
      sqlDb = new sqlJsEngine.Database();
      registerSqlJsUdfs(sqlDb);
      try {
        sqlDb.run(setupSql);
      } catch (_) {}
    } else {
      try {
        const ctrl = new AbortController();
        const tid = setTimeout(() => ctrl.abort(), 1500);
        await fetch("/api/reset", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            project_id: activeProjectId,
            setup_sql: customSetupSql || undefined,
          }),
          signal: ctrl.signal,
        });
        clearTimeout(tid);
      } catch (_) {}
    }

    // Ensure any user-defined tables in schemaEditor stay synchronized with the database
    if (syncEditorSchema && wasmReady && schemaEditor && schemaEditor.value.trim()) {
      await applySchemaDdlNonDestructively(schemaEditor.value);
    } else if (sqlDb && typeof sqlDb.export === "function") {
      try {
        projectDbSnapshots.set(activeProjectId, sqlDb.export());
        registerSqlJsUdfs(sqlDb);
      } catch (_) {}
    }
  } finally {
    isResettingDb = false;
  }
}

function splitExecutableSqlStatements(sql) {
  const stmts = [];
  let current = "";
  let inSingle = false;
  let inDouble = false;
  let inLineComment = false;
  let inBlockComment = false;

  for (let i = 0; i < sql.length; i++) {
    const ch = sql[i];
    const next = sql[i + 1] || "";

    if (inLineComment) {
      if (ch === "\n") {
        inLineComment = false;
        current += ch;
      }
      continue;
    }
    if (inBlockComment) {
      if (ch === "*" && next === "/") {
        inBlockComment = false;
        i++;
      }
      continue;
    }
    if (!inSingle && !inDouble) {
      if (ch === "-" && next === "-") {
        inLineComment = true;
        i++;
        continue;
      }
      if (ch === "/" && next === "*") {
        inBlockComment = true;
        i++;
        continue;
      }
    }
    if (ch === "'" && !inDouble) {
      if (inSingle && next === "'") {
        current += "''";
        i++;
        continue;
      }
      inSingle = !inSingle;
      current += ch;
      continue;
    }
    if (ch === '"' && !inSingle) {
      inDouble = !inDouble;
      current += ch;
      continue;
    }
    if (ch === ";" && !inSingle && !inDouble) {
      if (current.trim()) {
        stmts.push(current.trim());
      }
      current = "";
      continue;
    }
    current += ch;
  }
  if (current.trim()) {
    stmts.push(current.trim());
  }
  return stmts;
}

function cleanDisplayColumnName(col) {
  return String(col || "").replace(/\bt\d+\./g, "");
}

async function executeSqliteQuery(sqlString) {
  let cleanedSql = sqlString
    .split("\n")
    .filter((line) => !line.trim().startsWith("--"))
    .join("\n")
    .trim();

  if (!cleanedSql) {
    return { columns: [], rows: [] };
  }

  // Substitute default sample bind values if the query contains `?` placeholders
  if (cleanedSql.includes("?")) {
    const defaultBind = /isbn/i.test(cleanedSql) ? "'978-1234567890'" : "20";
    cleanedSql = cleanedSql.replace(/\?/g, defaultBind);
  }

  // Make CREATE VIEW idempotent in interactive playground sessions
  const createViewMatch = cleanedSql.match(
    /\bCREATE\s+VIEW\s+(?:IF\s+NOT\s+EXISTS\s+)?(["`]?[A-Za-z0-9_]+["`]?)/i
  );
  if (createViewMatch) {
    const viewName = createViewMatch[1];
    cleanedSql = `DROP VIEW IF EXISTS ${viewName};\n${cleanedSql}`;
  }

  cleanedSql = makeDdlIdempotent(cleanedSql);

  if (sqlDb) {
    registerSqlJsUdfs(sqlDb);
    const stmts = splitExecutableSqlStatements(cleanedSql);
    const selectResults = [];
    const dmlEvents = [];

    for (const stmtSql of stmts) {
      let stmt = null;
      try {
        stmt = sqlDb.prepare(stmtSql);
        const rawCols = stmt.getColumnNames();
        const rows = [];
        while (stmt.step()) {
          rows.push(stmt.get());
        }
        const cleanCols = rawCols.map(cleanDisplayColumnName);
        if (cleanCols.length > 0) {
          selectResults.push({ columns: cleanCols, rows });
        } else {
          const modified =
            typeof sqlDb.getRowsModified === "function" ? sqlDb.getRowsModified() : 0;
          const verb = (stmtSql.trim().match(/^[A-Za-z]+/) || ["SQL"])[0].toUpperCase();
          dmlEvents.push({ verb, modified });
        }
      } finally {
        if (stmt) {
          try {
            stmt.free();
          } catch (_) {}
        }
      }
    }

    if (typeof sqlDb.export === "function") {
      try {
        projectDbSnapshots.set(activeProjectId, sqlDb.export());
        registerSqlJsUdfs(sqlDb);
      } catch (_) {}
    }

    if (selectResults.length > 0) {
      const lastSel = selectResults[selectResults.length - 1];
      return {
        columns: lastSel.columns,
        rows: lastSel.rows,
        isSelect: true,
        selectResults,
        dmlEvents,
      };
    }

    const dmlRows = dmlEvents.map((e, idx) => [
      stmts.length > 1 ? `Statement #${idx + 1} (${e.verb})` : e.verb,
      /^(INSERT|UPDATE|DELETE)$/i.test(e.verb)
        ? `${e.modified} row${e.modified === 1 ? "" : "s"} affected`
        : "Statement executed successfully",
    ]);
    const singleDml = dmlEvents.length === 1 ? dmlEvents[0] : null;
    const dmlSummary =
      singleDml && /^(INSERT|UPDATE|DELETE)$/i.test(singleDml.verb)
        ? `${singleDml.modified} row${singleDml.modified === 1 ? "" : "s"} affected`
        : "Statement executed";
    return {
      columns: ["Operation", "Status"],
      rows: dmlRows.length ? dmlRows : [["SQL", "Statement executed successfully"]],
      isDmlOnly: true,
      dmlSummary,
    };
  }

  try {
    const resp = await fetch("/api/execute", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ sql: cleanedSql }),
    });
    if (resp.ok) {
      const payload = await resp.json();
      if (payload.ok) {
        const cols = (payload.columns || []).map(cleanDisplayColumnName);
        if (cols.length === 0) {
          return {
            columns: ["Operation", "Status"],
            rows: [["SQL", "Statement executed successfully"]],
            isDmlOnly: true,
            dmlSummary: "Statement executed",
          };
        }
        return { columns: cols, rows: payload.rows || [], isSelect: true };
      }
      throw new Error(payload.error || "SQLite execution error");
    }
  } catch (err) {
    if (err.message && !err.message.includes("Failed to fetch")) {
      throw err;
    }
  }

  return {
    columns: ["Info"],
    rows: [
      [
        "Generated SQL verified. Start `uv run --frozen python website/server.py` for local SQLite execution.",
      ],
    ],
  };
}

/* ==========================================================================
   Syntax Highlighting, Gutter & Inline Diagnostic Squigglies
   ========================================================================== */
function escapeHtml(str) {
  return String(str)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

const SQL_VERBS = new Set([
  "SELECT",
  "INSERT",
  "UPDATE",
  "DELETE",
  "CREATE",
  "DROP",
  "ALTER",
  "EXPLAIN",
  "WITH",
  "RECURSIVE",
  "UNION",
  "INTERSECT",
  "EXCEPT",
  "VALUES",
  "RETURNS",
]);

const SQL_CLAUSES = new Set([
  "FROM",
  "WHERE",
  "JOIN",
  "INNER",
  "LEFT",
  "RIGHT",
  "FULL",
  "OUTER",
  "CROSS",
  "ON",
  "GROUP",
  "BY",
  "HAVING",
  "ORDER",
  "LIMIT",
  "OFFSET",
  "INTO",
  "SET",
  "TABLE",
  "VIEW",
  "INDEX",
  "FUNCTION",
  "AS",
  "DISTINCT",
  "ALL",
  "OVER",
  "PARTITION",
  "ROWS",
  "RANGE",
  "UNBOUNDED",
  "PRECEDING",
  "CURRENT",
  "ROW",
  "FOLLOWING",
  "LANGUAGE",
  "SQL",
  "IMMUTABLE",
  "CYCLE",
  "USING",
]);

const SQL_COMPS = new Set([
  "AND",
  "OR",
  "NOT",
  "IN",
  "IS",
  "NULL",
  "BETWEEN",
  "LIKE",
  "EXISTS",
  "ASC",
  "DESC",
  "CASE",
  "WHEN",
  "THEN",
  "ELSE",
  "END",
]);

const SQL_AGGS = new Set([
  "COUNT",
  "SUM",
  "AVG",
  "MIN",
  "MAX",
  "ROUND",
  "ABS",
  "COALESCE",
  "NULLIF",
  "UPPER",
  "LOWER",
  "LENGTH",
  "SUBSTR",
  "RANK",
  "DENSE_RANK",
  "ROW_NUMBER",
  "LAG",
  "LEAD",
  "FIRST_VALUE",
  "LAST_VALUE",
  "NTILE",
  "STRFTIME",
  "CAST",
  "POWER",
  "SQRT",
  "MOD",
]);

const SQL_TYPES = new Set([
  "INTEGER",
  "INT",
  "REAL",
  "FLOAT",
  "DOUBLE",
  "NUMERIC",
  "DECIMAL",
  "TEXT",
  "VARCHAR",
  "CHAR",
  "BOOLEAN",
  "BOOL",
  "DATE",
  "TIMESTAMP",
  "PRIMARY",
  "KEY",
  "FOREIGN",
  "REFERENCES",
  "UNIQUE",
  "CHECK",
  "DEFAULT",
  "AUTOINCREMENT",
]);

function highlightSqlLine(lineText, sqlTableSet) {
  const commentIdx = lineText.indexOf("--");
  const codePart = commentIdx >= 0 ? lineText.slice(0, commentIdx) : lineText;
  const commentPart = commentIdx >= 0 ? lineText.slice(commentIdx) : "";

  const tokenRegex = /('(?:''|[^'])*'|"[^"]*"|\s+|<=|>=|<>|!=|[(),.;=<>+*\/\-]|[^\s(),.;=<>+*\/'"\-]+)/g;
  let html = "";
  let match;
  while ((match = tokenRegex.exec(codePart)) !== null) {
    const seg = match[0];
    if (/^\s+$/.test(seg)) {
      html += seg;
      continue;
    }
    let cls = "";
    if (seg.startsWith("'") || seg.startsWith('"')) {
      cls = "hl-str";
    } else if (/^[0-9]+(\.[0-9]+)?$/.test(seg)) {
      cls = "hl-num";
    } else if (/^(<=|>=|<>|!=|=|<|>)$/.test(seg)) {
      cls = "hl-comp";
    } else if (seg === ";") {
      cls = "hl-danda";
    } else {
      const upper = seg.toUpperCase();
      if (SQL_VERBS.has(upper)) cls = "hl-verb";
      else if (SQL_CLAUSES.has(upper)) cls = "hl-clause";
      else if (SQL_COMPS.has(upper)) cls = "hl-comp";
      else if (SQL_AGGS.has(upper)) cls = "hl-agg";
      else if (SQL_TYPES.has(upper)) cls = "hl-type";
      else if (sqlTableSet && sqlTableSet.has(seg.toLowerCase())) cls = "hl-table";
      else if (VERB_SET.has(seg) || CLAUSE_SET.has(seg)) cls = "hl-clause";
    }

    if (cls) {
      html += `<span class="${cls}">${escapeHtml(seg)}</span>`;
    } else {
      html += escapeHtml(seg);
    }
  }

  if (commentPart) {
    html += `<span class="hl-comment">${escapeHtml(commentPart)}</span>`;
  }

  return html || " ";
}

function getActiveSqlTableSet() {
  const sqlTableSet = new Set();
  const tables =
    lastCompileResult?.tables?.length ? lastCompileResult.tables : cachedSchemaTables;
  if (Array.isArray(tables)) {
    for (const t of tables) {
      if (t.sql_name) sqlTableSet.add(t.sql_name.toLowerCase());
    }
  }
  return sqlTableSet;
}

function highlightSqlSource(sqlText) {
  const sqlTableSet = getActiveSqlTableSet();
  return String(sqlText || "")
    .split("\n")
    .map((line) => highlightSqlLine(line, sqlTableSet))
    .join("\n");
}

function classifyWord(word, tableFormsSet) {
  if (word === "।") return "hl-danda";
  if (/^[०-९0-9]+(\.[०-९0-9]+)?$/.test(word)) return "hl-num";
  if (VERB_SET.has(word)) return "hl-verb";
  if (CLAUSE_SET.has(word)) return "hl-clause";
  if (COMP_SET.has(word)) return "hl-comp";
  if (AGG_SET.has(word)) return "hl-agg";
  if (TYPE_SET.has(word)) return "hl-type";
  if (tableFormsSet && tableFormsSet.has(word)) return "hl-table";
  return "";
}

function highlightLineWithDiagnostics(lineText, lineNumber, lineDiags, tableFormsSet) {
  // Handle single-line comments `// ...`
  const commentIdx = lineText.indexOf("//");
  let codePart = commentIdx >= 0 ? lineText.slice(0, commentIdx) : lineText;
  const commentPart = commentIdx >= 0 ? lineText.slice(commentIdx) : "";

  // Tokenize `codePart` into runs of strings, whitespace, punctuation, or words
  const tokenRegex = /("[^"]*"|\s+|।|[^\s"।]+)/g;
  let html = "";
  let match;
  while ((match = tokenRegex.exec(codePart)) !== null) {
    const seg = match[0];
    const startCol = match.index + 1;
    const endCol = startCol + seg.length;

    let cls = "";
    if (seg.startsWith('"')) {
      cls = "hl-str";
    } else if (!/^\s+$/.test(seg)) {
      cls = classifyWord(seg, tableFormsSet);
    }

    // Check if any diagnostic overlaps [startCol, endCol)
    let diagCls = "";
    let diagTitle = "";
    if (lineDiags && lineDiags.length > 0 && !/^\s+$/.test(seg)) {
      for (const d of lineDiags) {
        if (endCol > d.start_col && startCol < d.end_col) {
          diagCls = d.severity === "error" ? "diag-err-span" : "diag-warn-span";
          diagTitle = d.message;
          if (d.severity === "error") break;
        }
      }
    }

    const combinedCls = [cls, diagCls].filter(Boolean).join(" ");
    if (combinedCls) {
      const titleAttr = diagTitle ? ` title="${escapeHtml(diagTitle).replace(/"/g, "&quot;")}"` : "";
      html += `<span class="${combinedCls}"${titleAttr}>${escapeHtml(seg)}</span>`;
    } else {
      html += escapeHtml(seg);
    }
  }

  if (commentPart) {
    html += `<span class="hl-comment">${escapeHtml(commentPart)}</span>`;
  }

  return html || " ";
}

function renderEditorHighlightAndGutter(textarea, highlightPre, gutterEl, targetName) {
  if (!textarea || !highlightPre || !gutterEl) return;
  const text = textarea.value;
  const lines = text.split("\n");

  const tableFormsSet = new Set();
  const tables =
    lastCompileResult?.tables?.length ? lastCompileResult.tables : cachedSchemaTables;
  if (Array.isArray(tables)) {
    for (const t of tables) {
      [t.sanskrit_stem, t.prathama_bahu, t.panchami_bahu, t.tritiya_bahu, t.saptami_bahu, t.shashthi_eka]
        .filter(Boolean)
        .forEach((w) => tableFormsSet.add(w));
    }
  }

  const diagsByLine = new Map();
  if (lastCompileResult && Array.isArray(lastCompileResult.diagnostics)) {
    for (const d of lastCompileResult.diagnostics) {
      if (d.target === targetName) {
        if (!diagsByLine.has(d.line)) diagsByLine.set(d.line, []);
        diagsByLine.get(d.line).push(d);
      }
    }
  }

  const highlightedLines = [];
  let gutterHtml = "";
  const sqlTableSet = targetName === "sql" ? getActiveSqlTableSet() : null;

  for (let i = 0; i < lines.length; i++) {
    const lineNum = i + 1;
    const lineDiags = diagsByLine.get(lineNum) || [];
    if (targetName === "sql") {
      highlightedLines.push(highlightSqlLine(lines[i], sqlTableSet));
    } else {
      highlightedLines.push(
        highlightLineWithDiagnostics(lines[i], lineNum, lineDiags, tableFormsSet)
      );
    }

    const hasErr = lineDiags.some((d) => d.severity === "error");
    const hasWarn = !hasErr && lineDiags.length > 0;
    const cls = hasErr ? "gutter-line has-err" : hasWarn ? "gutter-line has-warn" : "gutter-line";
    const icon = hasErr ? "❌" : hasWarn ? "⚠️" : "";
    const tooltip = lineDiags.map((d) => `[${d.code}] ${d.message}`).join(" | ");
    const titleAttr = tooltip ? ` title="${escapeHtml(tooltip).replace(/"/g, "&quot;")}"` : "";
    gutterHtml += `<div class="${cls}" data-line="${lineNum}"${titleAttr}><span>${icon}</span><span>${lineNum}</span></div>`;
  }

  const codeEl = highlightPre.querySelector("code");
  if (codeEl) {
    codeEl.innerHTML = highlightedLines.join("\n") + (text.endsWith("\n") ? "\n " : "");
  }
  gutterEl.innerHTML = gutterHtml;
}

function syncEditorScroll(textarea, highlightPre, gutterEl) {
  if (!textarea || !highlightPre || !gutterEl) return;
  highlightPre.scrollTop = textarea.scrollTop;
  highlightPre.scrollLeft = textarea.scrollLeft;
  gutterEl.scrollTop = textarea.scrollTop;
}

function stripGeneratedSqlHeader(sqlText) {
  if (!sqlText) return "";
  return sqlText.replace(/^(?:--\s*(?:Generated SQL|Dialect:)[^\n]*\n)+\s*/i, "").trimEnd();
}

function renderGeneratedSqlWithGutter(sqlText) {
  if (!generatedSqlCode) return;
  const cleaned = stripGeneratedSqlHeader(sqlText);
  const codeEl = generatedSqlCode.querySelector("code");
  if (codeEl) {
    codeEl.innerHTML = highlightSqlSource(cleaned);
  }
  if (generatedSqlGutter) {
    const lines = cleaned.split("\n");
    let gutterHtml = "";
    for (let i = 0; i < lines.length; i++) {
      const lineNum = i + 1;
      gutterHtml += `<div class="gutter-line" data-line="${lineNum}"><span></span><span>${lineNum}</span></div>`;
    }
    generatedSqlGutter.innerHTML = gutterHtml;
    generatedSqlCode.scrollTop = 0;
    generatedSqlGutter.scrollTop = 0;
  }
}

/* ==========================================================================
   Live Hover Help / Cursor Token Inspector & Quick Fixes
   ========================================================================== */
function getWordAndLineColAtOffset(text, offset) {
  const clamped = Math.max(0, Math.min(offset, text.length));
  const prefix = text.slice(0, clamped);
  const linesBefore = prefix.split("\n");
  const line = linesBefore.length;
  const col = linesBefore[linesBefore.length - 1].length + 1;

  const isWordChar = (ch) => /[\u0900-\u097F\w।]/.test(ch);
  let start = clamped;
  let end = clamped;
  while (start > 0 && isWordChar(text[start - 1])) start--;
  while (end < text.length && isWordChar(text[end])) end++;
  const word = text.slice(start, end);
  return { word, line, col, startOffset: start, endOffset: end };
}

function updateCursorHoverHelp() {
  if (!ssqlEditor || !hoverBarContent) return;
  const info = getWordAndLineColAtOffset(ssqlEditor.value, ssqlEditor.selectionStart);

  // Check if there is an active diagnostic on this line
  const lineDiags = (lastCompileResult?.diagnostics || []).filter(
    (d) => d.target === "query" && d.line === info.line
  );

  // Check if `ssql-wasm` returned a `TokenBreakdown` at (line, col) or matching surface
  let tokMatch = null;
  if (lastCompileResult && Array.isArray(lastCompileResult.tokens)) {
    tokMatch =
      lastCompileResult.tokens.find(
        (t) => t.line === info.line && info.col >= t.start_col && info.col <= t.end_col
      ) ||
      lastCompileResult.tokens.find((t) => t.surface === info.word);
  }

  let html = "";
  if (tokMatch) {
    const isVerbTok =
      VERB_SET.has(tokMatch.surface) ||
      (tokMatch.category && tokMatch.category.startsWith("Verb")) ||
      (tokMatch.karaka_role && tokMatch.karaka_role.startsWith("Verb"));
    const vibhaktiLabel = isVerbTok ? "लकारः" : "विभक्तिः";
    const lingaLabel = isVerbTok ? "प्रयोगः" : "लिङ्गम्";
    html += `<span class="hover-pill">${escapeHtml(tokMatch.surface)}</span>`;
    html += `<strong>प्रातिपदिकम्:</strong> <code>${escapeHtml(tokMatch.pratipadika)}</code> • `;
    html += `<strong>${vibhaktiLabel}:</strong> ${escapeHtml(tokMatch.vibhakti)} (${escapeHtml(tokMatch.vachana)}) • `;
    html += `<strong>${lingaLabel}:</strong> ${escapeHtml(tokMatch.linga)} • `;
    html += `<strong>कारकम्:</strong> ${escapeHtml(tokMatch.karaka_role)}`;
    if (tokMatch.sql_mapping && tokMatch.sql_mapping !== "—") {
      html += ` • <strong>SQL:</strong> <code>${escapeHtml(tokMatch.sql_mapping)}</code>`;
    }
  } else if (info.word && KEYWORD_LOOKUP.has(info.word)) {
    const kw = KEYWORD_LOOKUP.get(info.word);
    html += `<span class="hover-pill">${escapeHtml(info.word)}</span>`;
    html += `<strong>${escapeHtml(kw.cat)}:</strong> <code>${escapeHtml(kw.sql)}</code>`;
  } else {
    html += `Line ${info.line}, Col ${info.col} • Click or hover over any Sanskrit token to inspect its Pāṇinian Kāraka &amp; Vibhakti.`;
  }

  if (lineDiags.length > 0) {
    const d = lineDiags[0];
    html += ` <span class="hover-pill err">${d.severity === "error" ? "❌" : "⚠️"} ${escapeHtml(d.code)}: ${escapeHtml(d.message)}</span>`;
  }

  hoverBarContent.innerHTML = html;
}

/* ==========================================================================
   Interactive Autocomplete (`Ctrl+Space` & Live Devanagari Prefix)
   ========================================================================== */
function buildAutocompleteCandidates(prefix) {
  const candidates = [];
  const seen = new Set();
  const isDevanagariPrefix = Boolean(prefix && /[\u0900-\u097F]/.test(prefix));

  const addCandidate = (token, sqlDesc, cat) => {
    const clean = token.trim();
    if (!clean || seen.has(clean)) return;
    if (prefix) {
      if (isDevanagariPrefix) {
        if (!clean.startsWith(prefix)) return;
      } else if (!clean.startsWith(prefix) && !sqlDesc.toLowerCase().includes(prefix.toLowerCase())) {
        return;
      }
    }
    seen.add(clean);
    candidates.push({ token: clean, insert: token, sql: sqlDesc, cat });
  };

  // 1. Declined Active Schema Tables & Columns (highest priority!)
  const tables =
    lastCompileResult?.tables?.length ? lastCompileResult.tables : cachedSchemaTables;
  if (Array.isArray(tables)) {
    for (const t of tables) {
      addCandidate(`${t.panchami_bahu} `, `FROM ${t.sql_name} (पञ्चमी बहुवचनम्)`, "Table");
      addCandidate(`${t.tritiya_bahu} सह `, `JOIN ${t.sql_name} (तृतीया बहुवचनम्)`, "Table");
      addCandidate(`${t.shashthi_eka} `, `${t.sql_name}. (षष्ठी एकवचनम्)`, "Table");
      addCandidate(`${t.saptami_bahu} `, `INTO/UPDATE ${t.sql_name} (सप्तमी बहुवचनम्)`, "Table");
      for (const c of t.columns) {
        addCandidate(`${c.prathama_eka} `, `${t.sql_name}.${c.sql_name} (${c.gender.split(" ")[0]})`, "Column");
        addCandidate(`${c.shashthi_eka} `, `of ${t.sql_name}.${c.sql_name} (षष्ठी)`, "Column");
        addCandidate(`${c.tritiya_eka} `, `by ${t.sql_name}.${c.sql_name} (तृतीया)`, "Column");
      }
    }
  }

  // 2. All SSQL Keywords across all categories
  for (const items of Object.values(CHIP_CATEGORIES)) {
    for (const item of items) {
      addCandidate(item.insert, item.sql, item.cat);
    }
  }

  return candidates.slice(0, 18);
}

function showAutocomplete(textarea, force = false) {
  if (!autocompletePopup || !textarea) return;
  const info = getWordAndLineColAtOffset(textarea.value, textarea.selectionStart);
  const prefix = info.word.replace(/।/g, "").trim();

  if (!force && prefix.length < 1) {
    hideAutocomplete();
    return;
  }

  acItems = buildAutocompleteCandidates(prefix);
  if (acItems.length === 0) {
    hideAutocomplete();
    return;
  }

  acSelectedIndex = 0;
  renderAutocompleteList(textarea);
  autocompletePopup.removeAttribute("hidden");
}

function renderAutocompleteList(textarea) {
  if (!autocompletePopup) return;
  autocompletePopup.innerHTML = "";
  acItems.forEach((item, idx) => {
    const row = document.createElement("div");
    row.className = idx === acSelectedIndex ? "ac-item active" : "ac-item";
    row.setAttribute("role", "option");
    row.innerHTML = `
      <span class="ac-token">${escapeHtml(item.token)}</span>
      <span class="ac-meta">${escapeHtml(item.sql)} • ${escapeHtml(item.cat)}</span>
    `;
    row.addEventListener("mousedown", (e) => {
      e.preventDefault();
      applyAutocompleteItem(textarea, item);
    });
    autocompletePopup.appendChild(row);
  });
}

function applyAutocompleteItem(textarea, item) {
  if (!textarea || !item) return;
  const info = getWordAndLineColAtOffset(textarea.value, textarea.selectionStart);
  const before = textarea.value.slice(0, info.startOffset);
  const after = textarea.value.slice(info.endOffset);
  textarea.value = before + item.insert + after;
  const nextPos = info.startOffset + item.insert.length;
  textarea.selectionStart = nextPos;
  textarea.selectionEnd = nextPos;
  hideAutocomplete();
  textarea.focus();
  compileAndRender(false);
}

function hideAutocomplete() {
  if (autocompletePopup) {
    autocompletePopup.setAttribute("hidden", "");
  }
  acItems = [];
}

/* ==========================================================================
   7-Category Chip Palette Rendering
   ========================================================================== */
function renderChipPalette() {
  if (!chipPaletteItems) return;
  chipPaletteItems.innerHTML = "";

  let items = [];
  if (activeChipCategory === "schema_stems") {
    // Dynamic declined table & column stems from `lastCompileResult.tables` or `cachedSchemaTables` + Devanagari numerals
    const tables =
      lastCompileResult?.tables?.length ? lastCompileResult.tables : cachedSchemaTables;
    if (Array.isArray(tables)) {
      for (const t of tables) {
        items.push({
          label: `${t.panchami_bahu}`,
          sub: `FROM ${t.sql_name}`,
          insert: `${t.panchami_bahu} `,
          sql: `FROM ${t.sql_name} (पञ्चमी बहुवचनम्)`,
        });
        items.push({
          label: `${t.tritiya_bahu} सह`,
          sub: `JOIN ${t.sql_name}`,
          insert: `${t.tritiya_bahu} सह `,
          sql: `JOIN ${t.sql_name} (तृतीया बहुवचनम्)`,
        });
        items.push({
          label: `${t.shashthi_eka}`,
          sub: `${t.sql_name}.`,
          insert: `${t.shashthi_eka} `,
          sql: `${t.sql_name}. (षष्ठी एकवचनम्)`,
        });
        for (const c of t.columns.slice(0, 6)) {
          items.push({
            label: c.prathama_eka,
            sub: c.sql_name,
            insert: `${c.prathama_eka} `,
            sql: `${t.sql_name}.${c.sql_name} (${c.data_type}, ${c.gender})`,
          });
        }
      }
    }
    for (const d of ["०", "१", "२", "३", "४", "५", "६", "७", "८", "९"]) {
      items.push({ label: d, sub: "", insert: d, sql: `Devanagari numeral ${d}` });
    }
  } else {
    items = CHIP_CATEGORIES[activeChipCategory] || [];
  }

  for (const item of items) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "chip-btn";
    btn.title = item.sql || "";
    btn.innerHTML = item.sub
      ? `${escapeHtml(item.label)}<span class="chip-sub">${escapeHtml(item.sub)}</span>`
      : escapeHtml(item.label);
    btn.addEventListener("click", () => {
      const targetEditor =
        !document.getElementById("panel-schema").hasAttribute("hidden")
          ? schemaEditor
          : ssqlEditor;
      const start = targetEditor.selectionStart ?? targetEditor.value.length;
      const end = targetEditor.selectionEnd ?? targetEditor.value.length;
      const text = targetEditor.value;
      targetEditor.value = text.slice(0, start) + item.insert + text.slice(end);
      targetEditor.focus();
      targetEditor.selectionStart = targetEditor.selectionEnd = start + item.insert.length;
      compileAndRender(false);
    });
    chipPaletteItems.appendChild(btn);
  }
}

/* ==========================================================================
   Render Tables, Adhyeta, Diagnostics & Schema Cards
   ========================================================================== */
function formatCellValue(val) {
  if (val === null || val === undefined) {
    return "NULL (रिक्तम्)";
  }
  if (typeof val === "number" && !Number.isInteger(val)) {
    const str = String(val);
    const dotIdx = str.indexOf(".");
    return dotIdx >= 0 && str.length - dotIdx - 1 > 4
      ? String(Number(val.toFixed(4)))
      : str;
  }
  return String(val);
}

function appendRowsToTbody(tbody, columns, rows) {
  if (rows.length === 0) {
    const emptyTr = document.createElement("tr");
    const emptyTd = document.createElement("td");
    emptyTd.colSpan = Math.max(1, columns.length);
    emptyTd.className = "empty-result-cell";
    emptyTd.textContent = "0 rows returned (query matched no records)";
    emptyTr.appendChild(emptyTd);
    tbody.appendChild(emptyTr);
    return;
  }
  rows.forEach((rowVals) => {
    const tr = document.createElement("tr");
    rowVals.forEach((val, idx) => {
      const cell = document.createElement(idx === 0 ? "th" : "td");
      if (idx === 0) cell.setAttribute("scope", "row");
      cell.textContent = formatCellValue(val);
      tr.appendChild(cell);
    });
    tbody.appendChild(tr);
  });
}

function renderDataTable(columns, rows, meta = {}) {
  resultsThead.innerHTML = "";
  resultsTbody.innerHTML = "";

  const tableEl = document.getElementById("results-data-table");
  if (tableEl) {
    const labelVars = columns
      .map((col, idx) => `--label-${idx + 1}: '${String(col).replace(/'/g, "")}'`)
      .join("; ");
    tableEl.setAttribute("style", labelVars);
  }

  if (columns.length === 0) {
    execRowCount.textContent = meta.compileFailed
      ? "Not executed (compile error)"
      : "0 rows";
    return;
  }

  const selectResults = Array.isArray(meta.selectResults) ? meta.selectResults : null;
  if (selectResults && selectResults.length > 1) {
    // Render all result sets in multi-statement batches
    const firstSel = selectResults[0];
    const headRow = document.createElement("tr");
    firstSel.columns.forEach((col) => {
      const th = document.createElement("th");
      th.textContent = col;
      headRow.appendChild(th);
    });
    resultsThead.appendChild(headRow);

    let totalRows = 0;
    selectResults.forEach((sel, sIdx) => {
      if (sIdx > 0) {
        const sepTr = document.createElement("tr");
        sel.columns.forEach((col) => {
          const th = document.createElement("th");
          th.style.backgroundColor = "var(--bg-elevated)";
          th.style.fontWeight = "700";
          th.textContent = `Result #${sIdx + 1}: ${col}`;
          sepTr.appendChild(th);
        });
        resultsTbody.appendChild(sepTr);
      }
      appendRowsToTbody(resultsTbody, sel.columns, sel.rows);
      totalRows += sel.rows.length;
    });

    execRowCount.textContent = `${totalRows} row${totalRows === 1 ? "" : "s"} (${selectResults.length} queries)`;
    updateResultsScrollAffordance();
    return;
  }

  const headRow = document.createElement("tr");
  columns.forEach((col) => {
    const th = document.createElement("th");
    th.textContent = col;
    headRow.appendChild(th);
  });
  resultsThead.appendChild(headRow);

  appendRowsToTbody(resultsTbody, columns, rows);

  if (meta.isDmlOnly) {
    execRowCount.textContent = meta.dmlSummary || "Statement executed";
  } else {
    const baseCount = `${rows.length} row${rows.length === 1 ? "" : "s"}`;
    if (Array.isArray(meta.dmlEvents) && meta.dmlEvents.length > 0) {
      execRowCount.textContent = `${baseCount} (+ ${meta.dmlEvents.length} DML)`;
    } else {
      execRowCount.textContent = baseCount;
    }
  }
  updateResultsScrollAffordance();
}

function updateResultsScrollAffordance() {
  const scroller = document.getElementById("results-table-container");
  const hint = document.getElementById("results-scroll-hint");
  if (!scroller) return;
  requestAnimationFrame(() => {
    const canScrollHoriz = scroller.scrollWidth > scroller.clientWidth + 4;
    scroller.classList.toggle(
      "can-scroll-right",
      canScrollHoriz && scroller.scrollLeft + scroller.clientWidth < scroller.scrollWidth - 4
    );
    scroller.classList.toggle("scrolled-right", scroller.scrollLeft > 4);
    if (hint) {
      if (canScrollHoriz) {
        hint.removeAttribute("hidden");
      } else {
        hint.setAttribute("hidden", "");
      }
    }
  });
}

function renderAdhyetaTable(tokens) {
  adhyetaTbody.innerHTML = "";
  badgeTokens.textContent = String(tokens.length);

  const tableEl = document.getElementById("adhyeta-table");
  if (tableEl) {
    tableEl.setAttribute(
      "style",
      "--label-1: 'Surface'; --label-2: 'Stem'; --label-3: 'Vibhakti'; --label-4: 'Vacana'; --label-5: 'Liṅga'; --label-6: 'Kāraka Role'; --label-7: 'SQL Mapping';"
    );
  }

  tokens.forEach((tok) => {
    const tr = document.createElement("tr");
    const cells = [
      tok.surface,
      tok.pratipadika,
      tok.vibhakti,
      tok.vachana,
      tok.linga,
      tok.karaka_role,
      tok.sql_mapping,
    ];
    cells.forEach((text, idx) => {
      const el = document.createElement(idx === 0 ? "th" : "td");
      if (idx === 0) el.setAttribute("scope", "row");
      el.textContent = text;
      tr.appendChild(el);
    });
    adhyetaTbody.appendChild(tr);
  });
}

function renderDiagnostics(diagnostics) {
  diagnosticsList.innerHTML = "";
  const list = Array.isArray(diagnostics) ? diagnostics : [];
  badgeDiags.textContent = String(list.length);

  if (list.length === 0) {
    const okDiv = document.createElement("div");
    okDiv.className = "diag-item diag-ok";
    okDiv.textContent =
      "✓ शुद्धम् (No compiler errors or Pāṇinian padapāṭha lint warnings!)";
    diagnosticsList.appendChild(okDiv);
    return;
  }

  list.forEach((d) => {
    const div = document.createElement("div");
    div.className = d.severity === "error" ? "diag-item diag-error" : "diag-item diag-lint";
    div.style.cursor = "pointer";
    div.innerHTML = `
      <div class="diag-header">
        <span>${d.severity === "error" ? "❌" : "⚠️"} ${escapeHtml(d.code)} (${escapeHtml(d.target)}.सं)</span>
        <span>Line ${d.line}, Col ${d.start_col}</span>
      </div>
      <div>${escapeHtml(d.message)}</div>
      ${d.suggestion ? `<div class="diag-suggestion">💡 Suggestion: ${escapeHtml(d.suggestion)}</div>` : ""}
    `;
    div.addEventListener("click", () => {
      const targetTab = d.target === "schema" ? "tab-schema" : "tab-ssql";
      document.getElementById(targetTab)?.click();
      const editor = d.target === "schema" ? schemaEditor : ssqlEditor;
      editor?.focus();
    });
    diagnosticsList.appendChild(div);
  });
}

function renderSchemaCards(tables) {
  schemaCardsContainer.innerHTML = "";
  tables.forEach((t) => {
    const card = document.createElement("div");
    card.className = "schema-card";
    const colsHtml = t.columns
      .map(
        (c) => `
        <li>
          <span>
            <button type="button" class="chip-btn" data-insert-col="${escapeHtml(c.prathama_eka)} " title="Insert ${escapeHtml(c.prathama_eka)}">${escapeHtml(c.prathama_eka)}</button>
            <code>(${escapeHtml(c.sql_name)})</code>
          </span>
          <span>${escapeHtml(c.data_type)} • ${escapeHtml(c.gender.split(" ")[0])}</span>
        </li>`
      )
      .join("");
    card.innerHTML = `
      <div class="schema-card-header">
        <span>${escapeHtml(t.sanskrit_stem)} → <code>${escapeHtml(t.sql_name)}</code></span>
        <span>${escapeHtml(t.gender.split(" ")[0])}</span>
      </div>
      <div class="quick-insert-bar">
        <button type="button" class="chip-btn" data-insert-col="${escapeHtml(t.panchami_bahu)} " title="FROM ${escapeHtml(t.sql_name)}">${escapeHtml(t.panchami_bahu)} (FROM)</button>
        <button type="button" class="chip-btn" data-insert-col="${escapeHtml(t.tritiya_bahu)} सह " title="JOIN ${escapeHtml(t.sql_name)}">${escapeHtml(t.tritiya_bahu)} सह (JOIN)</button>
        <button type="button" class="chip-btn" data-insert-col="${escapeHtml(t.shashthi_eka)} " title="${escapeHtml(t.sql_name)}.">${escapeHtml(t.shashthi_eka)} (.)</button>
      </div>
      <ul class="schema-col-list">${colsHtml}</ul>
    `;
    card.querySelectorAll("[data-insert-col]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const ins = btn.getAttribute("data-insert-col") || "";
        const pos = ssqlEditor.selectionStart ?? ssqlEditor.value.length;
        ssqlEditor.value =
          ssqlEditor.value.slice(0, pos) + ins + ssqlEditor.value.slice(pos);
        ssqlEditor.focus();
        compileAndRender(false);
      });
    });
    schemaCardsContainer.appendChild(card);
  });
}

/* ==========================================================================
   Core Compile & Execute Workflow
   ========================================================================== */
function isMutatingSsqlOrSql(ssqlText, sqlText = "") {
  const ssql = String(ssqlText || "");
  if (
    /(निवेश्य|परिवर्त्य|निष्कास्यताम्|लुप्यताम्|रच्यताम्|योज्यताम्|अभिधीयताम्)/.test(
      ssql
    )
  ) {
    return true;
  }
  const sql = stripGeneratedSqlHeader(sqlText || "");
  return /\b(INSERT|UPDATE|DELETE|DROP|ALTER|CREATE)\b/i.test(sql);
}

async function compileAndRender(executeSql = true, options = {}) {
  if (!wasmReady) return;

  const ssqlSource = ssqlEditor.value;
  const schemaSource = schemaEditor.value;
  const dialect = dialectSelect.value;

  sqlDialectLabel.textContent = `Generated SQL (${
    dialect === "postgres" ? "PostgreSQL" : "SQLite"
  })`;
  if (execStatusLabel) {
    execStatusLabel.textContent =
      dialect === "postgres"
        ? "Live Query Results (Simulated via In-Browser SQLite)"
        : "Live Query Results (SQLite DB)";
  }

  const rawJson = compile_ssql_wasm(ssqlSource, schemaSource, dialect);
  const result = JSON.parse(rawJson);
  lastCompileResult = result;
  if (Array.isArray(result.tables) && result.tables.length > 0) {
    cachedSchemaTables = result.tables;
  }

  renderAdhyetaTable(result.tokens || []);
  renderDiagnostics(result.diagnostics || []);
  renderSchemaCards(result.tables?.length ? result.tables : cachedSchemaTables);
  renderEditorHighlightAndGutter(ssqlEditor, ssqlHighlight, ssqlGutter, "query");
  renderEditorHighlightAndGutter(schemaEditor, schemaHighlight, schemaGutter, "schema");
  renderEditorHighlightAndGutter(sqlInputEditor, sqlInputHighlight, sqlInputGutter, "sql");
  renderEditorHighlightAndGutter(exerciseEditor, exerciseHighlight, exerciseGutter, "exercise");
  updateCursorHoverHelp();
  if (activeChipCategory === "schema_stems") {
    renderChipPalette();
  }

  if (!result.ok) {
    renderDataTable([], [], { compileFailed: true });
    compileErrorBanner.removeAttribute("hidden");
    compileErrorBanner.textContent = (result.errors || []).join("\n\n");
    renderGeneratedSqlWithGutter(
      "-- Compilation halted due to grammatical or semantic errors above."
    );
    return;
  }

  if (pendingDeepLinkWarning) {
    compileErrorBanner.removeAttribute("hidden");
    compileErrorBanner.textContent = pendingDeepLinkWarning;
    pendingDeepLinkWarning = "";
  } else {
    compileErrorBanner.setAttribute("hidden", "");
  }
  renderGeneratedSqlWithGutter(result.sql);

  if (executeSql) {
    const unfilteredDmlWarn = (result.lint_warnings || []).find(
      (w) => w.code === "SSQL-LINT-DML"
    );
    if (
      unfilteredDmlWarn &&
      !options.forceUnfilteredDml &&
      confirmedUnfilteredDmlQuery !== ssqlSource.trim()
    ) {
      compileErrorBanner.removeAttribute("hidden");
      compileErrorBanner.innerHTML = "";
      const msgSpan = document.createElement("span");
      msgSpan.textContent = `⚠️ Safety Guard (${unfilteredDmlWarn.code}): ${unfilteredDmlWarn.message} `;
      const confirmBtn = document.createElement("button");
      confirmBtn.type = "button";
      confirmBtn.id = "btn-confirm-unfiltered-dml";
      confirmBtn.className = "btn btn-primary btn-xs";
      confirmBtn.style.marginInlineStart = "0.5rem";
      confirmBtn.textContent = "Confirm & Execute All Rows";
      confirmBtn.addEventListener("click", () => {
        confirmedUnfilteredDmlQuery = ssqlSource.trim();
        compileAndRender(true, { forceUnfilteredDml: true });
      });
      compileErrorBanner.appendChild(msgSpan);
      compileErrorBanner.appendChild(confirmBtn);
      renderDataTable(
        ["Operation", "Status"],
        [
          [
            "Unfiltered DML Guard",
            "Execution paused: statement has no WHERE filter (इति नियमेन). Click 'Confirm & Execute All Rows' above to proceed.",
          ],
        ],
        { isDmlOnly: true, dmlSummary: "Confirmation required" }
      );
      return;
    }

    try {
      let sqlForSqliteEngine = result.sql;
      if (dialect === "postgres") {
        const sqliteRes = JSON.parse(compile_ssql_wasm(ssqlSource, schemaSource, "sqlite"));
        if (sqliteRes.ok && sqliteRes.sql) {
          sqlForSqliteEngine = sqliteRes.sql;
        }
      }
      const execOut = await executeSqliteQuery(sqlForSqliteEngine);
      renderDataTable(execOut.columns, execOut.rows, execOut);
    } catch (sqlErr) {
      renderDataTable([], [], { compileFailed: true });
      compileErrorBanner.removeAttribute("hidden");
      compileErrorBanner.textContent = `SQLite Execution Note: ${sqlErr.message}`;
    }
  }
}

/* ==========================================================================
   Corpus Explorer (All Example Projects & Queries)
   ========================================================================== */
function renderScenarioContextCard({
  projectTitle,
  domain,
  difficulty,
  fileName,
  summary,
  objective,
}) {
  if (!scenarioDescription) return;
  const dbLabel = projectTitle || "University Bookstore";
  const projPills = [
    `<span id="active-project-badge" class="ctx-pill ctx-pill-proj">DB: ${escapeHtml(dbLabel)}</span>`,
    domain ? `<span class="ctx-pill">${escapeHtml(domain)}</span>` : "",
    difficulty ? `<span class="ctx-pill">${escapeHtml(difficulty)}</span>` : "",
  ]
    .filter(Boolean)
    .join("");

  const queryPills = [
    fileName ? `<span class="ctx-pill ctx-pill-file"><code>${escapeHtml(fileName)}</code></span>` : "",
  ]
    .filter(Boolean)
    .join("");

  const summaryHtml = summary
    ? `<div class="ctx-summary"><strong>Project Context:</strong> ${escapeHtml(summary)}</div>`
    : "";
  const objectiveHtml = objective
    ? `<div class="ctx-objective"><strong>Query Objective:</strong> ${escapeHtml(objective)}</div>`
    : "";

  scenarioDescription.innerHTML = `
    <div class="context-subcard context-subcard-project" id="project-context-box">
      <div class="ctx-meta-row">${projPills}</div>
      ${summaryHtml}
    </div>
    <div class="context-subcard context-subcard-objective" id="query-objective-box">
      <div class="ctx-meta-row">${queryPills}</div>
      ${objectiveHtml}
    </div>
  `;
}

function populateProjectSelector() {
  if (!projectSelect) return;
  projectSelect.innerHTML = `
    <option value="__featured__">★ Featured Quick Start Scenarios • University Bookstore</option>
  `;
  if (corpusData && Array.isArray(corpusData.projects)) {
    const metricProj = document.getElementById("metric-projects");
    const metricQueries = document.getElementById("metric-queries");
    if (metricProj) metricProj.textContent = String(corpusData.project_count);
    if (metricQueries) metricQueries.textContent = `${corpusData.total_queries}+`;

    corpusData.projects.forEach((p, idx) => {
      const opt = document.createElement("option");
      opt.value = p.id;
      const qWord = p.query_count === 1 ? "query" : "queries";
      opt.textContent = `${idx + 1}. ${p.title} • ${p.query_count} ${qWord}`;
      projectSelect.appendChild(opt);
    });
  }
}

function populateQuerySelectorForProject(projectId, autoLoad = true) {
  if (!exampleSelect) return;
  exampleSelect.innerHTML = "";

  if (projectId === "__featured__") {
    for (const [key, sc] of Object.entries(FEATURED_SCENARIOS)) {
      const opt = document.createElement("option");
      opt.value = key;
      opt.textContent = sc.title;
      exampleSelect.appendChild(opt);
    }
    schemaEditor.value = projectSchemaDrafts.get("__featured__") || bookstoreSchemaSsql();
    if (activeProjectBadge) activeProjectBadge.textContent = "DB: University Bookstore";
    if (autoLoad) loadSelectedQuery();
    return;
  }

  const proj = corpusData?.projects?.find((p) => p.id === projectId);
  if (!proj) return;

  schemaEditor.value =
    projectSchemaDrafts.get(projectId) || proj.schema_ssql || bookstoreSchemaSsql();
  if (activeProjectBadge) activeProjectBadge.textContent = `DB: ${proj.title}`;

  proj.queries.forEach((q) => {
    const opt = document.createElement("option");
    opt.value = q.id;
    opt.textContent = q.title;
    exampleSelect.appendChild(opt);
  });

  if (autoLoad) loadSelectedQuery();
}

function loadSelectedQuery(triggerCompile = true) {
  const qId = exampleSelect.value;
  if (activeProjectId === "__featured__") {
    const sc = FEATURED_SCENARIOS[qId] || FEATURED_SCENARIOS.filter;
    ssqlEditor.value = sc.ssql;
    if (sqlInputEditor) {
      sqlInputEditor.value =
        sc.sample_sql ||
        "SELECT title, price\nFROM books\nWHERE price > 25\nORDER BY price DESC\nLIMIT 5;";
    }
    renderScenarioContextCard({
      projectTitle: "University Bookstore",
      domain: "Retail • Bookstore",
      difficulty: "★ Featured",
      fileName: `${qId || "filter"}.सं`,
      summary:
        "University bookstore network managing books, authors, publishers, inventory, customers, sales, sales representatives, and book reviews.",
      objective: sc.description,
    });
    saveWorkspaceStateToStorage();
    syncUrlQueryParams();
    if (triggerCompile) compileAndRender(true);
    return;
  }

  const proj = corpusData?.projects?.find((p) => p.id === activeProjectId);
  if (!proj) return;
  const q = proj.queries.find((item) => item.id === qId) || proj.queries[0];
  if (q) {
    ssqlEditor.value = q.ssql;
    if (sqlInputEditor) {
      sqlInputEditor.value =
        q.sample_sql ||
        proj.sample_sql ||
        "SELECT title, price\nFROM books\nWHERE price > 25\nORDER BY price DESC\nLIMIT 5;";
    }
    renderScenarioContextCard({
      projectTitle: proj.title,
      domain: proj.domain,
      difficulty: proj.difficulty,
      fileName: q.filename,
      summary: proj.summary,
      objective: q.objective,
    });
    saveWorkspaceStateToStorage();
    syncUrlQueryParams();
    if (triggerCompile) compileAndRender(true);
  }
}

/* ==========================================================================
   Tutorials, Markdown Renderer & Runnable Chapter Exercises
   ========================================================================== */
function renderCodeBlockCard(cb) {
  if (!cb) return "";
  const isSsql =
    cb.lang === "ssql" ||
    cb.lang === "सं" ||
    (!cb.lang && /[\u0900-\u097F]/.test(cb.code));
  const isSql = cb.lang === "sql" || cb.lang === "sqlite" || cb.lang === "postgres";
  const loadBtn = isSsql
    ? `<button type="button" class="btn btn-primary btn-xs btn-load-snippet" data-snippet="${encodeURIComponent(cb.code)}">▶ Load in Playground</button>`
    : "";
  const highlightedCode = isSsql
    ? cb.code
        .split("\n")
        .map((l, idx) => highlightLineWithDiagnostics(l, idx + 1, [], new Set()))
        .join("\n")
    : isSql
    ? highlightSqlSource(cb.code)
    : escapeHtml(cb.code);
  return `
    <div class="tutorial-code-wrap">
      <div class="tutorial-code-header">
        <span>${escapeHtml(cb.lang.toUpperCase() || "CODE")}</span>
        ${loadBtn}
      </div>
      <pre><code>${highlightedCode}</code></pre>
    </div>
  `;
}

function renderListLinesToHtml(lines) {
  const isOrdered = /^\d+\.\s+/.test(lines[0].trim());
  const tag = isOrdered ? "ol" : "ul";
  const items = [];
  for (const rawLine of lines) {
    const l = rawLine.trim();
    if (!l) continue;
    if (/^(\*|-|\d+\.)\s+/.test(l)) {
      items.push(l.replace(/^(\*|-|\d+\.)\s+/, ""));
    } else if (items.length > 0) {
      items[items.length - 1] += " " + l;
    } else {
      items.push(l);
    }
  }
  return `<${tag}>${items.map((item) => `<li>${formatInlineMarkdown(item)}</li>`).join("")}</${tag}>`;
}

function renderMarkdownToHtml(md) {
  if (!md) return "";
  const codeBlocks = [];
  const withPlaceholders = md.replace(/```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g, (_, lang, code) => {
    const idx = codeBlocks.length;
    codeBlocks.push({ lang: (lang || "ssql").toLowerCase(), code: code.trim() });
    return `\n\n@@CODE_BLOCK_${idx}@@\n\n`;
  });

  const blocks = withPlaceholders.split(/\n\n+/);
  const htmlParts = blocks.map((block) => {
    const trimmed = block.trim();
    if (!trimmed) return "";
    const codeMatch = trimmed.match(/^@@CODE_BLOCK_(\d+)@@$/);
    if (codeMatch) {
      return renderCodeBlockCard(codeBlocks[Number(codeMatch[1])]);
    }
    if (trimmed.startsWith("### ")) {
      return `<h3>${formatInlineMarkdown(trimmed.slice(4))}</h3>`;
    }
    if (trimmed.startsWith("## ")) {
      return `<h2>${formatInlineMarkdown(trimmed.slice(3))}</h2>`;
    }
    if (trimmed.startsWith("# ")) {
      return `<h3 class="tutorial-chapter-heading">${formatInlineMarkdown(trimmed.slice(2))}</h3>`;
    }
    if (trimmed.startsWith(">")) {
      const quoteLines = trimmed
        .split("\n")
        .map((l) => l.trim().replace(/^>\s?/, ""));
      const alertMatch = quoteLines[0]?.match(/^\[!(NOTE|WARNING|TIP|IMPORTANT|CAUTION)\]\s*(.*)$/i);
      if (alertMatch) {
        const kind = alertMatch[1].toUpperCase();
        const restFirst = alertMatch[2] ? [alertMatch[2]] : [];
        const bodyText = [...restFirst, ...quoteLines.slice(1)].filter(Boolean).join(" ");
        return `<div class="md-alert md-alert-${kind.toLowerCase()}"><div class="md-alert-title">${kind}</div><div>${formatInlineMarkdown(bodyText)}</div></div>`;
      }
      return `<blockquote class="md-blockquote">${formatInlineMarkdown(quoteLines.join(" "))}</blockquote>`;
    }
    if (trimmed.startsWith("|") && trimmed.includes("\n|")) {
      const rows = trimmed.split("\n").filter((r) => r.trim().startsWith("|"));
      if (rows.length >= 2) {
        const parseRow = (r) =>
          r
            .trim()
            .replace(/^\||\|$/g, "")
            .split("|")
            .map((c) => formatInlineMarkdown(c.trim()));
        const headers = parseRow(rows[0]);
        const bodyRows = rows.slice(2).map(parseRow);
        return `
          <div class="table-wrapper" tabindex="0">
            <table>
              <thead><tr>${headers.map((h) => `<th>${h}</th>`).join("")}</tr></thead>
              <tbody>${bodyRows
                .map((r) => `<tr>${r.map((c, i) => (i === 0 ? `<th>${c}</th>` : `<td>${c}</td>`)).join("")}</tr>`)
                .join("")}</tbody>
            </table>
          </div>
        `;
      }
    }
    const blockLines = trimmed.split("\n");
    const firstListIdx = blockLines.findIndex((l) => /^(\*|-|\d+\.)\s+/.test(l.trim()));
    if (firstListIdx === 0) {
      return renderListLinesToHtml(blockLines);
    }
    if (firstListIdx > 0) {
      const introLines = blockLines.slice(0, firstListIdx).join(" ");
      const listLines = blockLines.slice(firstListIdx);
      return `<p>${formatInlineMarkdown(introLines)}</p>\n${renderListLinesToHtml(listLines)}`;
    }
    return `<p>${formatInlineMarkdown(trimmed)}</p>`;
  });

  return htmlParts
    .join("\n")
    .replace(/@@CODE_BLOCK_(\d+)@@/g, (_, idx) => renderCodeBlockCard(codeBlocks[Number(idx)]));
}

function formatInlineMarkdown(text) {
  const inlineCodeSpans = [];
  const withCodePlaceholders = String(text || "").replace(/`([^`]+)`/g, (_, code) => {
    const idx = inlineCodeSpans.length;
    inlineCodeSpans.push(`<code>${escapeHtml(code)}</code>`);
    return `@@INLINE_CODE_${idx}@@`;
  });

  const escaped = escapeHtml(withCodePlaceholders)
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, label, href) => {
      const cleanHref = href.trim();
      if (cleanHref.endsWith(".md")) {
        return `<a href="#tutorials" class="tutorial-chap-link" data-md-href="${cleanHref}">${label}</a>`;
      }
      return `<a href="${cleanHref}" target="_blank" rel="noopener noreferrer">${label}</a>`;
    })
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\*([^*]+)\*/g, "<em>$1</em>");

  return escaped.replace(/@@INLINE_CODE_(\d+)@@/g, (_, idx) => inlineCodeSpans[Number(idx)] || "");
}

function resolveTutorialMdHrefToChapId(mdHref) {
  if (!mdHref) return null;
  const base = mdHref.split("/").pop() || "";
  if (contentData && Array.isArray(contentData.tutorials)) {
    const found = contentData.tutorials.find((c) => c.filename === base);
    if (found) return found.id;
  }
  const map = {
    "01-introduction.md": "track-a-01",
    "01-setup-and-hello-world.md": "track-a-01",
    "02-select-basics.md": "track-a-02",
    "03-where-conditions.md": "track-a-03",
    "03-where-filtering.md": "track-a-03",
    "04-joins.md": "track-a-04",
    "05-aggregates.md": "track-a-05",
    "05-grouping-and-aggregation.md": "track-a-05",
    "06-window-functions-and-ctes.md": "track-a-06",
    "06-dml-mutations.md": "track-a-06",
    "07-pure-functions-and-transpiler.md": "track-a-07",
    "07-advanced-analytics.md": "track-a-07",
    "01-vibhakti-overview.md": "track-b-01",
    "01-vibhakti-foundations.md": "track-b-01",
    "02-prathama-and-vachana-concordance.md": "track-b-02",
    "02-linga-and-vachana-concord.md": "track-b-02",
    "03-linga-and-visheshana-concordance.md": "track-b-03",
    "03-reading-ssql-as-prose.md": "track-b-03",
  };
  return map[base] || null;
}

function selectCustomSnippetInQueryDropdown(labelText) {
  if (!exampleSelect) return;
  let customOpt = exampleSelect.querySelector('option[value="__custom_snippet__"]');
  if (!customOpt) {
    customOpt = document.createElement("option");
    customOpt.value = "__custom_snippet__";
    exampleSelect.prepend(customOpt);
  }
  customOpt.textContent = labelText;
  exampleSelect.value = "__custom_snippet__";
}

function renderTutorialsSection() {
  const navList = document.getElementById("tutorial-nav-list");
  if (!navList || !contentData || !Array.isArray(contentData.tutorials)) return;

  navList.innerHTML = "";
  let currentTrack = "";
  contentData.tutorials.forEach((chap) => {
    if (chap.track_title !== currentTrack) {
      currentTrack = chap.track_title;
      const h = document.createElement("div");
      h.className = "tutorial-track-heading";
      h.textContent = currentTrack;
      navList.appendChild(h);
    }
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = chap.id === activeTutorialId ? "tutorial-nav-btn active" : "tutorial-nav-btn";
    btn.dataset.chapId = chap.id;
    const checkMark = completedExercises.has(chap.id) ? " ✓" : "";
    btn.innerHTML = `<span>${escapeHtml(chap.title)}</span><span>${checkMark}</span>`;
    btn.addEventListener("click", () => {
      activeTutorialId = chap.id;
      saveWorkspaceStateToStorage();
      renderTutorialsSection();
      renderActiveTutorialChapter();
    });
    navList.appendChild(btn);
  });

  renderActiveTutorialChapter();
}

function renderActiveTutorialChapter() {
  if (!contentData || !Array.isArray(contentData.tutorials)) return;
  const chap =
    contentData.tutorials.find((c) => c.id === activeTutorialId) ||
    contentData.tutorials[0];
  if (!chap) return;

  const articleEl = document.getElementById("tutorial-article");
  if (articleEl) {
    articleEl.innerHTML = renderMarkdownToHtml(chap.markdown);
    articleEl.querySelectorAll(".tutorial-chap-link").forEach((link) => {
      link.addEventListener("click", (e) => {
        e.preventDefault();
        const targetId = resolveTutorialMdHrefToChapId(link.getAttribute("data-md-href"));
        if (targetId) {
          activeTutorialId = targetId;
          renderTutorialsSection();
          document.getElementById("tutorials")?.scrollIntoView({ behavior: "smooth" });
        }
      });
    });
    articleEl.querySelectorAll(".btn-load-snippet").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const code = decodeURIComponent(btn.dataset.snippet || "");
        activeProjectId = "__featured__";
        if (projectSelect) projectSelect.value = "__featured__";
        populateQuerySelectorForProject("__featured__", false);
        selectCustomSnippetInQueryDropdown(`★ Tutorial Snippet: ${chap.title}`);
        schemaEditor.value = bookstoreSchemaSsql();
        if (activeProjectBadge) activeProjectBadge.textContent = "DB: University Bookstore";
        await resetDatabaseForCurrentProject(bookstoreSetupSql(), false);
        ssqlEditor.value = code;
        saveWorkspaceStateToStorage();
        syncUrlQueryParams();
        renderScenarioContextCard({
          projectTitle: "University Bookstore",
          domain: "Tutorial Example",
          difficulty: chap.track_title || "Interactive Tutorial",
          fileName: `${chap.id}.सं`,
          summary:
            "University bookstore network managing books, authors, publishers, inventory, customers, sales, sales representatives, and book reviews.",
          objective: `Interactive snippet loaded from chapter: ${chap.title}`,
        });
        document.getElementById("tab-ssql")?.click();
        document.getElementById("playground")?.scrollIntoView({ behavior: "smooth" });
        await compileAndRender(true);
      });
    });
  }

  const ex = chap.exercise;
  if (!ex) return;
  document.getElementById("exercise-title").textContent = ex.title;
  document.getElementById("exercise-prompt").innerHTML = formatInlineMarkdown(ex.prompt);
  const exEditor = document.getElementById("exercise-editor");
  if (exEditor && exEditor.dataset.chapId !== chap.id) {
    exEditor.value = ex.starter_ssql;
    exEditor.dataset.chapId = chap.id;
  }
  renderEditorHighlightAndGutter(exerciseEditor, exerciseHighlight, exerciseGutter, "exercise");

  const statusBadge = document.getElementById("exercise-status-badge");
  if (statusBadge) {
    if (completedExercises.has(chap.id)) {
      statusBadge.className = "exercise-badge passed";
      statusBadge.textContent = "✓ Completed";
    } else {
      statusBadge.className = "exercise-badge";
      statusBadge.textContent = "Ready to Try";
    }
  }

  document.getElementById("exercise-hint-box")?.setAttribute("hidden", "");
  document.getElementById("exercise-feedback-box")?.setAttribute("hidden", "");
}

async function runAndVerifyExercise() {
  if (!wasmReady || !contentData) return;
  const chap = contentData.tutorials.find((c) => c.id === activeTutorialId);
  if (!chap || !chap.exercise) return;
  const ex = chap.exercise;
  const exEditor = document.getElementById("exercise-editor");
  const feedbackBox = document.getElementById("exercise-feedback-box");
  if (!exEditor || !feedbackBox) return;

  const answerSsql = exEditor.value.trim();
  const resJson = compile_ssql_wasm(answerSsql, bookstoreSchemaSsql(), "sqlite");
  const res = JSON.parse(resJson);

  feedbackBox.removeAttribute("hidden");
  if (!res.ok) {
    feedbackBox.className = "exercise-feedback-box err";
    feedbackBox.textContent = `❌ Compilation Error: ${(res.errors || []).join(" | ")}`;
    return;
  }

  if (ex.expected_sql_contains) {
    if (res.sql.includes(ex.expected_sql_contains)) {
      completedExercises.add(chap.id);
      saveCompletedExercises();
      renderTutorialsSection();
      feedbackBox.removeAttribute("hidden");
      feedbackBox.className = "exercise-feedback-box ok";
      feedbackBox.innerHTML = `<strong>✓ साधु! Exercise Passed!</strong><br>Generated SQL:<pre class="sql-pre"><code>${highlightSqlSource(stripGeneratedSqlHeader(res.sql))}</code></pre>`;
    } else {
      feedbackBox.className = "exercise-feedback-box err";
      feedbackBox.textContent = `Compiled to SQL, but expected "${ex.expected_sql_contains}" in output.`;
    }
    return;
  }

  const savedSnapshot =
    sqlDb && typeof sqlDb.export === "function" ? sqlDb.export() : null;
  try {
    await resetDatabaseForCurrentProject(bookstoreSetupSql(), false);
    const execOut = await executeSqliteQuery(res.sql);
    const expectedCols = ex.expected_columns || [];
    const colsMatch =
      expectedCols.length === 0 ||
      (execOut.columns.length === expectedCols.length &&
        expectedCols.every((c, idx) => execOut.columns[idx] === c));
    const rowsOk = execOut.rows.length >= (ex.expected_min_rows || 1);

    if (colsMatch && rowsOk) {
      completedExercises.add(chap.id);
      saveCompletedExercises();
      renderTutorialsSection();
      feedbackBox.removeAttribute("hidden");
      feedbackBox.className = "exercise-feedback-box ok";
      feedbackBox.innerHTML = `<strong>✓ साधु! Exercise Passed (${execOut.rows.length} rows returned across [${execOut.columns.join(", ")}])!</strong>`;
    } else {
      feedbackBox.className = "exercise-feedback-box err";
      feedbackBox.textContent = `Query compiled and returned ${execOut.rows.length} rows with columns [${execOut.columns.join(", ")}], but expected columns [${expectedCols.join(", ")}].`;
    }
  } catch (err) {
    feedbackBox.className = "exercise-feedback-box err";
    feedbackBox.textContent = `SQLite Execution Error: ${err.message}`;
  } finally {
    if (savedSnapshot && sqlJsEngine) {
      try {
        if (sqlDb) sqlDb.close();
        sqlDb = new sqlJsEngine.Database(savedSnapshot);
        registerSqlJsUdfs(sqlDb);
        projectDbSnapshots.set(activeProjectId, savedSnapshot);
      } catch (_) {}
    }
  }
}

/* ==========================================================================
   Downloads & Constitution Rendering
   ========================================================================== */
function renderDownloadsSection() {
  const grid = document.getElementById("downloads-grid");
  if (!grid || !contentData || !Array.isArray(contentData.downloads)) return;

  grid.innerHTML = "";
  contentData.downloads.forEach((item) => {
    const card = document.createElement("article");
    card.className = "download-card";
    const fullSha = item.sha256_full || item.sha256 || "";
    const sourceCmdFull = item.source_cmd
      ? `git clone https://github.com/performance/SSQL.git && cd SSQL && ${item.source_cmd}`
      : "";
    const platformNoteHtml = item.platform_note
      ? `<div class="download-platform-note">${escapeHtml(item.platform_note)}</div>`
      : "";
    const sourceRowHtml = sourceCmdFull
      ? `<div class="download-cmd-row" style="margin-block-start: 0.35rem;" title="Build from source (Linux, Windows, Intel macOS)">
           <pre class="download-cmd"><code>${escapeHtml(sourceCmdFull)}</code></pre>
           <button type="button" class="btn btn-ghost btn-xs btn-copy-dl" data-copy-text="${escapeHtml(sourceCmdFull)}">Copy Source</button>
         </div>`
      : "";
    card.innerHTML = `
      <div>
        <span class="card-tag">${escapeHtml(item.badge)}</span>
        <h3>${escapeHtml(item.title)}</h3>
        <p>${escapeHtml(item.subtitle)}</p>
        ${platformNoteHtml}
      </div>
      <div>
        <div class="download-meta">
          <span>Size: <strong>${escapeHtml(item.size)}</strong></span>
        </div>
        <div class="download-sha-row">
          <code class="download-sha-code" title="${escapeHtml(fullSha)}">SHA256: ${escapeHtml(fullSha)}</code>
          <button type="button" class="btn btn-ghost btn-xs btn-copy-dl" data-copy-text="${escapeHtml(fullSha)}">Copy SHA</button>
        </div>
        <div class="download-cmd-row">
          <pre class="download-cmd"><code>${escapeHtml(item.install_cmd)}</code></pre>
          <button type="button" class="btn btn-ghost btn-xs btn-copy-dl" data-copy-text="${escapeHtml(item.install_cmd)}">Copy</button>
        </div>
        ${sourceRowHtml}
        <a href="${escapeHtml(item.url.replace(/^\//, "./"))}" download="${escapeHtml(item.filename)}" class="btn btn-primary btn-sm" style="inline-size: 100%; justify-content: center; margin-block-start: 0.55rem;">
          ⬇ Download ${escapeHtml(item.filename)}
        </a>
      </div>
    `;
    card.querySelectorAll(".btn-copy-dl").forEach((btn) => {
      btn.addEventListener("click", async () => {
        const textToCopy = btn.getAttribute("data-copy-text") || "";
        const origLabel = btn.textContent;
        try {
          await navigator.clipboard.writeText(textToCopy);
          btn.textContent = "Copied!";
          setTimeout(() => {
            btn.textContent = origLabel;
          }, 1200);
        } catch (_) {}
      });
    });
    grid.appendChild(card);
  });
}

function renderConstitutionSection() {
  const container = document.getElementById("constitution-articles");
  const closingEl = document.getElementById("constitution-closing");
  const headingEl = document.getElementById("constitution-heading");
  if (!container || !contentData || !contentData.constitution) return;

  if (headingEl && contentData.constitution.title) {
    headingEl.textContent = contentData.constitution.title;
  }

  container.innerHTML = "";
  const articles = contentData.constitution.articles || [];
  articles.forEach((art) => {
    const card = document.createElement("article");
    card.className = "constitution-card";
    const sub = art.subtitle ? `: ${escapeHtml(art.subtitle)}` : "";
    card.innerHTML = `
      <h3>${escapeHtml(art.number)}. <code>${escapeHtml(art.sanskrit)}</code> (${escapeHtml(art.english)})${sub}</h3>
      <div class="constitution-body">${renderMarkdownToHtml(art.body)}</div>
    `;
    container.appendChild(card);
  });

  if (closingEl && contentData.constitution.closing) {
    closingEl.textContent = contentData.constitution.closing;
  }
}

/* ==========================================================================
   Event Wiring & Initialization
   ========================================================================== */
async function bootstrap() {
  setupThemeToggle();
  setupScrollSpy();
  setupTabs(".editor-pane .pane-tab", ".editor-panel");
  setupTabs(".output-pane .pane-tab", ".output-panel");

  ssqlEditor.value = FEATURED_SCENARIOS.filter.ssql;
  scenarioDescription.textContent = FEATURED_SCENARIOS.filter.description;

  // Chip category tab switching
  document.querySelectorAll(".chip-cat-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      document.querySelectorAll(".chip-cat-btn").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      activeChipCategory = btn.dataset.chipCat || "clauses";
      renderChipPalette();
    });
  });
  renderChipPalette();

  // Load corpus.json and content.json
  try {
    const [corpusResp, contentResp] = await Promise.all([
      fetch("./data/corpus.json"),
      fetch("./data/content.json"),
    ]);
    if (corpusResp.ok) {
      corpusData = await corpusResp.json();
    }
    if (contentResp.ok) {
      contentData = await contentResp.json();
    }
  } catch (_) {}
  schemaEditor.value = bookstoreSchemaSsql();

  populateProjectSelector();

  // Restore dialect choice from sessionStorage / localStorage
  try {
    const savedDialect =
      sessionStorage.getItem("ssql-dialect") || localStorage.getItem("ssql-dialect");
    if (savedDialect === "postgres" || savedDialect === "sqlite") {
      if (dialectSelect) dialectSelect.value = savedDialect;
      document.querySelectorAll('input[name="sql-dialect"]').forEach((radio) => {
        radio.checked = radio.value === savedDialect;
      });
    }
  } catch (_) {}

  // Restore project & query state from URL params (`?project=...&query=...`) or per-tab sessionStorage
  try {
    const params = new URLSearchParams(window.location.search);
    const urlProj = params.get("project");
    const urlQuery = params.get("query");
    const hasUrlDeepLink = Boolean(urlProj || urlQuery);
    const savedProj = sessionStorage.getItem("ssql-active-project");
    const savedQuery = sessionStorage.getItem("ssql-active-query");
    const savedDraftQuery = sessionStorage.getItem("ssql-draft-query");
    const savedDraftQueryKey = sessionStorage.getItem("ssql-draft-query-key");
    const savedDraftSchema = sessionStorage.getItem("ssql-draft-schema");
    const savedDraftSchemaProject = sessionStorage.getItem("ssql-draft-schema-project");

    const isValidProject = (pid) =>
      pid === "__featured__" ||
      Boolean(corpusData?.projects?.some((p) => p.id === pid));

    const invalidUrlParts = [];
    if (urlProj && !isValidProject(urlProj)) {
      invalidUrlParts.push(`project "${urlProj}"`);
    }

    if (urlProj && isValidProject(urlProj)) {
      activeProjectId = urlProj;
    } else if (!hasUrlDeepLink && savedProj && isValidProject(savedProj)) {
      activeProjectId = savedProj;
    } else {
      activeProjectId = "__featured__";
    }

    if (projectSelect) {
      projectSelect.value = activeProjectId;
    }
    populateQuerySelectorForProject(activeProjectId, false);

    if (urlQuery) {
      if (Array.from(exampleSelect?.options || []).some((o) => o.value === urlQuery)) {
        exampleSelect.value = urlQuery;
        loadSelectedQuery(false);
      } else {
        invalidUrlParts.push(`query "${urlQuery}"`);
        loadSelectedQuery(false);
      }
    } else if (!hasUrlDeepLink && savedQuery) {
      if (savedQuery === "__custom_snippet__") {
        selectCustomSnippetInQueryDropdown("★ Custom / Restored Session Query");
      } else if (Array.from(exampleSelect?.options || []).some((o) => o.value === savedQuery)) {
        exampleSelect.value = savedQuery;
        loadSelectedQuery(false);
      } else {
        loadSelectedQuery(false);
      }
      const expectedKey = `${activeProjectId}::${exampleSelect?.value || ""}`;
      if (
        savedDraftQuery &&
        savedDraftQuery.trim() &&
        (!savedDraftQueryKey || savedDraftQueryKey === expectedKey)
      ) {
        ssqlEditor.value = savedDraftQuery;
      }
      if (
        savedDraftSchema &&
        savedDraftSchema.trim() &&
        (!savedDraftSchemaProject || savedDraftSchemaProject === activeProjectId)
      ) {
        schemaEditor.value = savedDraftSchema;
        projectSchemaDrafts.set(activeProjectId, savedDraftSchema);
      }
    } else {
      loadSelectedQuery(false);
    }

    if (invalidUrlParts.length > 0) {
      pendingDeepLinkWarning = `⚠️ Shared Link Notice: Unknown ${invalidUrlParts.join(" / ")} in URL; loaded default query instead.`;
    }
  } catch (_) {
    populateQuerySelectorForProject("__featured__", true);
  }

  renderTutorialsSection();
  renderDownloadsSection();
  renderConstitutionSection();

  // Initialize WASM Compiler + SQLite Engine
  try {
    await Promise.all([initWasm(), initSqliteRuntime()]);
    wasmReady = true;
    runtimeStatus.textContent = "● WASM Ready";
    runtimeStatus.title = "WASM Compiler + SQLite Engine Ready";
    runtimeStatus.classList.remove("status-loading");
    runtimeStatus.classList.add("status-ready");
    if (schemaEditor && schemaEditor.value.trim()) {
      await applySchemaDdlNonDestructively(schemaEditor.value);
    }
    await compileAndRender(true);
  } catch (err) {
    runtimeStatus.textContent = "WASM Error";
    runtimeStatus.title = `WASM Init Error: ${err.message}`;
  }

  // Project & Query selector events
  projectSelect?.addEventListener("change", async () => {
    const prevProjectId = activeProjectId;
    if (schemaEditor) {
      projectSchemaDrafts.set(prevProjectId, schemaEditor.value);
    }
    if (ssqlEditor) {
      projectQueryDrafts.set(prevProjectId, {
        queryId: exampleSelect?.value || "",
        ssql: ssqlEditor.value,
      });
    }
    if (sqlDb && typeof sqlDb.export === "function") {
      try {
        projectDbSnapshots.set(prevProjectId, sqlDb.export());
        registerSqlJsUdfs(sqlDb);
      } catch (_) {}
    }

    activeProjectId = projectSelect.value;
    populateQuerySelectorForProject(activeProjectId, false);

    if (sqlJsEngine && projectDbSnapshots.has(activeProjectId)) {
      try {
        if (sqlDb) sqlDb.close();
        sqlDb = new sqlJsEngine.Database(projectDbSnapshots.get(activeProjectId));
        registerSqlJsUdfs(sqlDb);
      } catch (_) {
        await resetDatabaseForCurrentProject();
      }
    } else {
      await resetDatabaseForCurrentProject();
    }

    const savedDraft = projectQueryDrafts.get(activeProjectId);
    if (savedDraft && savedDraft.ssql) {
      if (
        savedDraft.queryId &&
        Array.from(exampleSelect?.options || []).some((o) => o.value === savedDraft.queryId)
      ) {
        exampleSelect.value = savedDraft.queryId;
        loadSelectedQuery(false);
      } else if (savedDraft.queryId === "__custom_snippet__") {
        selectCustomSnippetInQueryDropdown("★ Custom / Session Query");
      } else {
        loadSelectedQuery(false);
      }
      ssqlEditor.value = savedDraft.ssql;
      saveWorkspaceStateToStorage();
      syncUrlQueryParams();
      await compileAndRender(true);
    } else {
      loadSelectedQuery(true);
    }
  });

  exampleSelect?.addEventListener("change", () => {
    loadSelectedQuery();
  });

  dialectSelect?.addEventListener("change", () => {
    const val = dialectSelect.value;
    document.querySelectorAll('input[name="sql-dialect"]').forEach((radio) => {
      radio.checked = radio.value === val;
    });
    saveWorkspaceStateToStorage();
    compileAndRender(true);
  });

  document.querySelectorAll('input[name="sql-dialect"]').forEach((radio) => {
    radio.addEventListener("change", () => {
      if (radio.checked && dialectSelect) {
        dialectSelect.value = radio.value;
        saveWorkspaceStateToStorage();
        compileAndRender(true);
      }
    });
  });

  // Workspace Split vs Stacked Full-Width Toggle
  const workspaceEl = document.getElementById("playground-workspace");
  document.querySelectorAll(".layout-mode-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const mode = btn.dataset.layoutMode;
      document.querySelectorAll(".layout-mode-btn").forEach((b) =>
        b.classList.toggle("active", b === btn)
      );
      if (workspaceEl) {
        workspaceEl.classList.toggle("stacked-mode", mode === "stacked");
      }
      updateResultsScrollAffordance();
    });
  });

  const resultsScroller = document.getElementById("results-table-container");
  resultsScroller?.addEventListener("scroll", updateResultsScrollAffordance, { passive: true });
  window.addEventListener("resize", updateResultsScrollAffordance, { passive: true });

  if (typeof CSS !== "undefined" && !CSS.supports("container-type", "scroll-state") && resultsScroller && "IntersectionObserver" in window) {
    const leftSentinel = resultsScroller.querySelector(".sentinel-left");
    const rightSentinel = resultsScroller.querySelector(".sentinel-right");
    if (leftSentinel && rightSentinel) {
      const scrollObs = new IntersectionObserver(
        (entries) => {
          entries.forEach((entry) => {
            if (entry.target === leftSentinel) {
              resultsScroller.classList.toggle("scrolled-right", !entry.isIntersecting);
            }
            if (entry.target === rightSentinel) {
              resultsScroller.classList.toggle("can-scroll-right", !entry.isIntersecting);
            }
          });
        },
        { root: resultsScroller }
      );
      scrollObs.observe(leftSentinel);
      scrollObs.observe(rightSentinel);
    }
  }

  // Toolbar buttons
  document.getElementById("btn-run")?.addEventListener("click", () => {
    clearTimeout(debounceTimer);
    hideAutocomplete();
    activateOutputTab("tab-out-results");
    compileAndRender(true);
  });

  document.getElementById("btn-format")?.addEventListener("click", () => {
    if (!wasmReady) return;
    ssqlEditor.value = format_ssql_wasm(ssqlEditor.value);
    compileAndRender(false);
    const fmtBtn = document.getElementById("btn-format");
    if (fmtBtn) {
      const orig = fmtBtn.textContent;
      fmtBtn.textContent = "✓ Formatted";
      setTimeout(() => {
        fmtBtn.textContent = orig;
      }, 1200);
    }
  });

  document.getElementById("btn-lint")?.addEventListener("click", () => {
    if (!wasmReady) return;
    compileAndRender(false);
    activateOutputTab("tab-out-diagnostics");
  });

  document.getElementById("btn-explain")?.addEventListener("click", () => {
    compileAndRender(false);
    activateOutputTab("tab-out-adhyeta");
  });

  document.getElementById("btn-trigger-autocomplete")?.addEventListener("click", (e) => {
    e.stopPropagation();
    ssqlEditor.focus();
    showAutocomplete(ssqlEditor, true);
  });

  document.getElementById("btn-reset-db")?.addEventListener("click", async () => {
    await resetDatabaseForCurrentProject(null, true);
    const resetBtn = document.getElementById("btn-reset-db");
    if (resetBtn) {
      const orig = resetBtn.textContent;
      resetBtn.textContent = "✓ DB Reset";
      setTimeout(() => {
        resetBtn.textContent = orig;
      }, 1200);
    }
    if (isMutatingSsqlOrSql(ssqlEditor?.value, lastCompileResult?.sql)) {
      await compileAndRender(false);
      renderDataTable(
        ["Operation", "Status"],
        [
          [
            "Reset Project DB",
            "Default project tables and sample rows restored (mutating statement in editor was not re-run).",
          ],
        ],
        { isDmlOnly: true, dmlSummary: "Database restored" }
      );
    } else {
      await compileAndRender(true);
    }
  });

  document.getElementById("btn-apply-schema")?.addEventListener("click", async () => {
    if (!wasmReady) return;
    const schemaDdl = schemaEditor.value;
    const applied = await applySchemaDdlNonDestructively(schemaDdl);
    if (!applied.ok) {
      renderDataTable([], [], { compileFailed: true });
      compileErrorBanner.removeAttribute("hidden");
      compileErrorBanner.textContent = (applied.errors || []).join("\n\n");
      activateOutputTab("tab-out-results");
      return;
    }
    projectSchemaDrafts.set(activeProjectId, schemaDdl);
    saveWorkspaceStateToStorage();
    const applyBtn = document.getElementById("btn-apply-schema");
    if (applyBtn) {
      const orig = applyBtn.textContent;
      applyBtn.textContent = "✓ Schema Applied (Data Preserved)";
      setTimeout(() => {
        applyBtn.textContent = orig;
      }, 1500);
    }
    compileAndRender(true);
  });

  document.getElementById("btn-reset-schema")?.addEventListener("click", async () => {
    const defaultSchema = getActiveProjectDefaultSchemaSsql();
    const resetSchemaBtn = document.getElementById("btn-reset-schema");
    const hasCustomSchema =
      schemaEditor && schemaEditor.value.trim() !== defaultSchema.trim();

    if (hasCustomSchema && resetSchemaBtn?.dataset.confirmPending !== "true") {
      if (resetSchemaBtn) {
        resetSchemaBtn.dataset.confirmPending = "true";
        resetSchemaBtn.textContent = "⚠️ Click Again to Confirm Reset";
        setTimeout(() => {
          if (resetSchemaBtn.dataset.confirmPending === "true") {
            delete resetSchemaBtn.dataset.confirmPending;
            resetSchemaBtn.textContent = "↺ Reset Schema";
          }
        }, 3500);
      }
      return;
    }

    if (resetSchemaBtn) {
      delete resetSchemaBtn.dataset.confirmPending;
    }
    projectSchemaDrafts.delete(activeProjectId);
    schemaEditor.value = defaultSchema;
    saveWorkspaceStateToStorage();
    await resetDatabaseForCurrentProject(null, false);
    if (resetSchemaBtn) {
      resetSchemaBtn.textContent = "✓ Default Schema Restored";
      setTimeout(() => {
        resetSchemaBtn.textContent = "↺ Reset Schema";
      }, 1500);
    }
    compileAndRender(true);
  });

  document.getElementById("btn-transpile-sql")?.addEventListener("click", () => {
    if (!wasmReady) return;
    const sqlText = sqlInputEditor.value;
    const raw = transpile_sql_to_ssql_wasm(sqlText, schemaEditor.value);
    const parsed = JSON.parse(raw);
    if (parsed.ok) {
      ssqlEditor.value = parsed.ssql;
      const ddlBlocks =
        parsed.ssql.match(/[^\n।]*?इति\s+पटलम्\s+"([^"]+)"[\s\S]*?इति\s+पटलम्\s*।/g) || [];
      for (const block of ddlBlocks) {
        const nameMatch = block.match(/इति\s+पटलम्\s+"([^"]+)"/);
        const sqlTableName = nameMatch ? nameMatch[1] : "";
        if (sqlTableName && !schemaEditor.value.includes(`"${sqlTableName}"`)) {
          schemaEditor.value = `${schemaEditor.value.trim()}\n\n${block.trim()}\n`;
          projectSchemaDrafts.set(activeProjectId, schemaEditor.value);
        }
      }
      saveWorkspaceStateToStorage();
      document.getElementById("tab-ssql")?.click();
      activateOutputTab("tab-out-results");
      compileAndRender(true);
    } else {
      renderDataTable([], [], { compileFailed: true });
      compileErrorBanner.removeAttribute("hidden");
      compileErrorBanner.textContent = `SQL → SSQL Transpile Error: ${parsed.error}`;
      activateOutputTab("tab-out-results");
    }
  });

  document.getElementById("btn-copy-sql")?.addEventListener("click", async () => {
    const code = generatedSqlCode.querySelector("code").textContent;
    try {
      await navigator.clipboard.writeText(code);
      const copyBtn = document.getElementById("btn-copy-sql");
      copyBtn.textContent = "Copied!";
      setTimeout(() => {
        copyBtn.textContent = "Copy SQL";
      }, 1400);
    } catch (_) {}
  });

  // Editor live compilation, scroll sync, hover help & autocomplete
  ssqlEditor?.addEventListener("input", () => {
    saveWorkspaceStateToStorage();
    renderEditorHighlightAndGutter(ssqlEditor, ssqlHighlight, ssqlGutter, "query");
    showAutocomplete(ssqlEditor, false);
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => compileAndRender(false), 180);
  });

  schemaEditor?.addEventListener("input", () => {
    projectSchemaDrafts.set(activeProjectId, schemaEditor.value);
    saveWorkspaceStateToStorage();
    renderEditorHighlightAndGutter(schemaEditor, schemaHighlight, schemaGutter, "schema");
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => compileAndRender(false), 220);
  });

  sqlInputEditor?.addEventListener("input", () => {
    renderEditorHighlightAndGutter(sqlInputEditor, sqlInputHighlight, sqlInputGutter, "sql");
  });

  exerciseEditor?.addEventListener("input", () => {
    renderEditorHighlightAndGutter(exerciseEditor, exerciseHighlight, exerciseGutter, "exercise");
  });

  ssqlEditor?.addEventListener("scroll", () =>
    syncEditorScroll(ssqlEditor, ssqlHighlight, ssqlGutter)
  );
  schemaEditor?.addEventListener("scroll", () =>
    syncEditorScroll(schemaEditor, schemaHighlight, schemaGutter)
  );
  sqlInputEditor?.addEventListener("scroll", () =>
    syncEditorScroll(sqlInputEditor, sqlInputHighlight, sqlInputGutter)
  );
  exerciseEditor?.addEventListener("scroll", () =>
    syncEditorScroll(exerciseEditor, exerciseHighlight, exerciseGutter)
  );
  generatedSqlCode?.addEventListener("scroll", () => {
    if (generatedSqlGutter) {
      generatedSqlGutter.scrollTop = generatedSqlCode.scrollTop;
    }
  });

  ["click", "keyup", "select"].forEach((evtName) => {
    ssqlEditor?.addEventListener(evtName, () => updateCursorHoverHelp());
  });

  ssqlEditor?.addEventListener("keydown", (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      hideAutocomplete();
      compileAndRender(true);
      return;
    }
    if ((e.ctrlKey || e.altKey) && e.code === "Space") {
      e.preventDefault();
      showAutocomplete(ssqlEditor, true);
      return;
    }
    if (!autocompletePopup.hasAttribute("hidden") && acItems.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        acSelectedIndex = (acSelectedIndex + 1) % acItems.length;
        renderAutocompleteList(ssqlEditor);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        acSelectedIndex = (acSelectedIndex - 1 + acItems.length) % acItems.length;
        renderAutocompleteList(ssqlEditor);
        return;
      }
      if (e.key === "Tab" || e.key === "Enter") {
        e.preventDefault();
        applyAutocompleteItem(ssqlEditor, acItems[acSelectedIndex]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        hideAutocomplete();
        return;
      }
    }
  });

  document.addEventListener("click", (e) => {
    const acBtn = document.getElementById("btn-trigger-autocomplete");
    if (
      autocompletePopup &&
      !autocompletePopup.contains(e.target) &&
      !(acBtn && acBtn.contains(e.target))
    ) {
      hideAutocomplete();
    }
  });

  // Tutorial Exercise Buttons
  document.getElementById("btn-exercise-check")?.addEventListener("click", () => {
    runAndVerifyExercise();
  });

  document.getElementById("btn-exercise-hint")?.addEventListener("click", () => {
    const chap = contentData?.tutorials?.find((c) => c.id === activeTutorialId);
    const hintBox = document.getElementById("exercise-hint-box");
    if (chap?.exercise && hintBox) {
      hintBox.removeAttribute("hidden");
      hintBox.innerHTML = `<strong>💡 Hint:</strong> ${formatInlineMarkdown(chap.exercise.hint)}`;
    }
  });

  document.getElementById("btn-exercise-solution")?.addEventListener("click", () => {
    const chap = contentData?.tutorials?.find((c) => c.id === activeTutorialId);
    const exEditor = document.getElementById("exercise-editor");
    if (chap?.exercise && exEditor) {
      exEditor.value = chap.exercise.solution_ssql;
      renderEditorHighlightAndGutter(exerciseEditor, exerciseHighlight, exerciseGutter, "exercise");
    }
  });

  document.getElementById("btn-exercise-open-playground")?.addEventListener("click", async () => {
    const exEditor = document.getElementById("exercise-editor");
    if (exEditor) {
      const chap = contentData?.tutorials?.find((c) => c.id === activeTutorialId);
      activeProjectId = "__featured__";
      if (projectSelect) projectSelect.value = "__featured__";
      populateQuerySelectorForProject("__featured__", false);
      selectCustomSnippetInQueryDropdown(`★ Exercise: ${chap?.exercise?.title || chap?.title || "Tutorial"}`);
      schemaEditor.value = bookstoreSchemaSsql();
      await resetDatabaseForCurrentProject(bookstoreSetupSql(), false);
      ssqlEditor.value = exEditor.value;
      saveWorkspaceStateToStorage();
      syncUrlQueryParams();
      renderScenarioContextCard({
        projectTitle: "University Bookstore",
        domain: "Tutorial Exercise",
        difficulty: chap?.track_title || "Interactive Exercise",
        fileName: `${activeTutorialId}_exercise.सं`,
        summary:
          "University bookstore network managing books, authors, publishers, inventory, customers, sales, sales representatives, and book reviews.",
        objective: chap?.exercise?.title || "Tutorial exercise loaded into the main playground.",
      });
      document.getElementById("tab-ssql")?.click();
      document.getElementById("playground")?.scrollIntoView({ behavior: "smooth" });
      await compileAndRender(true);
    }
  });

  document.getElementById("btn-reset-tutorial-progress")?.addEventListener("click", () => {
    const resetTutBtn = document.getElementById("btn-reset-tutorial-progress");
    if (completedExercises.size > 0 && resetTutBtn?.dataset.confirmPending !== "true") {
      resetTutBtn.dataset.confirmPending = "true";
      resetTutBtn.textContent = "⚠️ Click to Confirm";
      setTimeout(() => {
        if (resetTutBtn.dataset.confirmPending === "true") {
          delete resetTutBtn.dataset.confirmPending;
          resetTutBtn.textContent = "↺ Reset Progress";
        }
      }, 3000);
      return;
    }
    if (resetTutBtn) {
      delete resetTutBtn.dataset.confirmPending;
    }
    completedExercises.clear();
    saveCompletedExercises();
    renderTutorialsSection();
    if (resetTutBtn) {
      resetTutBtn.textContent = "✓ Progress Reset";
      setTimeout(() => {
        resetTutBtn.textContent = "↺ Reset Progress";
      }, 1500);
    }
  });

  // Ensure brand link (#top) scrolls cleanly to top of page
  document.querySelectorAll('a[href="#top"]').forEach((link) => {
    link.addEventListener("click", (e) => {
      e.preventDefault();
      window.history.pushState(null, "", "#top");
      window.scrollTo({ top: 0, behavior: "instant" });
    });
  });

  // Re-align scroll position if page was opened with a URL hash (e.g. /#tutorials on mobile)
  const alignHashScroll = () => {
    const hash = window.location.hash;
    if (!hash) return;
    if (hash === "#top") {
      window.scrollTo({ top: 0, behavior: "instant" });
      return;
    }
    try {
      const targetEl = document.querySelector(hash);
      if (targetEl) {
        targetEl.scrollIntoView({ behavior: "instant", block: "start" });
      }
    } catch (_) {}
  };
  alignHashScroll();
  requestAnimationFrame(() => {
    alignHashScroll();
  });
  window.addEventListener("hashchange", alignHashScroll);
}

bootstrap();
