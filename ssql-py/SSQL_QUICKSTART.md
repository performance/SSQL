# SSQL Language Tour

Samskrtam SQL (SSQL) is a query language written as Sanskrit sentences. The
grammatical case of each word (its vibhakti) says what role it plays in the
query. Every example here runs against the University Bookstore schema
(`examples/university_bookstore/SAMSQL/schema.सं`).

## 1. The schema (native DDL)

A schema is declared in SSQL itself. Each table and column states its name,
its SQL name (`"…" इति आख्या`), and its stem, gender and number; the compiler
declines the stem into every case form a query may use. The Python bindings'
`Compiler(schema_ddl, dialect)` takes this text as a string, and the CLI finds
a sibling `schema.सं` automatically.

```ssql
पुस्तकानि इति पटलम् "books" इति आख्या पुस्तक इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
    प्राथमिकी क्रमिका सङ्ख्या इति पूर्णाङ्कः "id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    अनिवार्यम् शीर्षकम् इति वाक्यम् "title" इति आख्या शीर्षक इति नपुंसकलिङ्गम् एकवचनम् च
    अनिवार्यम् मूल्यम् इति वास्तविकः "price" इति आख्या मूल्य इति नपुंसकलिङ्गम् एकवचनम् च
    लेखकसङ्ख्या इति पूर्णाङ्कः "author_id" इति आख्या लेखकसङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
  इति पटलम् ।
```

## 2. A query

A query is one sentence. The source table is in the ablative plural
(`पुस्तकेभ्यः`, "from the books"), the selected columns are in the nominative,
and the passive verb agrees with how many columns are selected: `आह्रियताम्`
for one, `आह्रियेताम्` for two, `आह्रियन्ताम्` for three or more. The sentence
ends with `।`.

```ssql
पुस्तकेभ्यः शीर्षकम् च मूल्यम् च आह्रियेताम् ।
```

`च` ("and") follows every item of a list, the last one too. A column of a
named table is written with the table in the genitive singular:
`पुस्तकस्य शीर्षकम्`, "the book's title".

## 3. Filtering

A condition is a clause ending in `अस्ति` ("is") or `नास्ति` ("is not"), and
the conditions are applied with `इति नियमेन` ("by this rule"). The comparison
word agrees with the column's gender: `मूल्यम्` is neuter, so `अधिकम्`;
`लेखकसङ्ख्या` is feminine, so `समा`.

**SQL:** `SELECT title FROM books WHERE price > 100 AND author_id = 1`

```ssql
पुस्तकेभ्यः मूल्यम् १०० इत्यस्मात् अधिकम् अस्ति च लेखकसङ्ख्या १ इत्यनया समा अस्ति च इति नियमेन
    शीर्षकम् आह्रियताम् ।
```

`वा` ("or") works like `च`: it follows every condition, the last one too. One
list never mixes `च` and `वा`; group the part you mean with `यत् … इति`.

## 4. Aggregation and grouping

An aggregate takes its argument in the genitive: `मूल्यस्य योगः` (SUM; the `इत्यस्य` form is for an alias). `पङ्क्तीनाम् गणना` is
COUNT(*). GROUP BY is `इति समूहेन`, HAVING is `इति समूहनियमेन`.

**SQL:** `SELECT language, COUNT(*) FROM books GROUP BY language HAVING COUNT(*) > 2`

```ssql
पुस्तकेभ्यः भाषा इति समूहेन पङ्क्तीनाम् गणना २ इत्यस्याः अधिका अस्ति इति समूहनियमेन
    भाषा च पङ्क्तीनाम् गणना च आह्रियेताम् ।
```

## 5. Ordering and limits

ORDER BY (`इति क्रमेण`) and LIMIT (`इति मित्या`) come before the select list,
with the other clause markers. The direction follows the key: `आरोहः`
(ascending) or `अवरोहः` (descending).

**SQL:** `SELECT title, price FROM books ORDER BY price DESC LIMIT 3`

```ssql
पुस्तकेभ्यः मूल्यम् अवरोहः इति क्रमेण ३ इति मित्या शीर्षकम् च मूल्यम् च आह्रियेताम् ।
```

## 6. Joins

A joined table is in the instrumental plural with `सह` ("with"), and the join
condition is applied with `इति मेलनेन`. `इति वाममेलनेन` is a LEFT JOIN.

**SQL:** `SELECT customers.name, sales.total_amount FROM sales JOIN customers ON sales.customer_id = customers.id`

```ssql
विक्रयणेभ्यः
    ग्राहकैः सह विक्रयणस्य ग्राहकसङ्ख्या ग्राहकस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    ग्राहकस्य नाम च विक्रयणस्य सकलमूल्यम् च आह्रियेताम् ।
```

## 7. Window functions

A window function names its partition (`… आधारेण विभाजनम्`) and its order
(`… अनुसारम् अवरोहक्रमेण`) after `यत्र`. `श्रेणी` is RANK.

```ssql
पुस्तकेभ्यः शीर्षकम् च भाषा च मूल्यम् च
    श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण मूल्यश्रेणी इति आख्या च
    आह्रियन्ताम् ।
```

## 8. Parameters

An external value is declared before the query with `इति बाह्या सञ्ज्ञा` and is
bound when the query runs.

```ssql
लक्षितमूल्यम् इति बाह्या सञ्ज्ञा ।

पुस्तकेभ्यः मूल्यम् लक्षितमूल्यम् इत्यनेन समम् अस्ति इति नियमेन शीर्षकम् आह्रियताम् ।
```

## Writing rules

- Only Devanagari is used, except inside quoted string literals; numbers are
  Devanagari digits.
- `।` ends every statement.
- There are no parentheses: grouping that changes the meaning is written
  `यत् … इति`.
- `--` starts a line comment.
