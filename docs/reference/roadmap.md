# SSQL Feature Roadmap

## Current Status

### Implemented ✅

| Feature | Grammar | Compiler | Notes |
|---------|---------|----------|-------|
| SELECT with columns | ✅ | ✅ | प्रथमा विभक्ति |
| FROM clause | ✅ | ✅ | पञ्चमी विभक्ति |
| WHERE conditions | ✅ | ✅ | `इति नियमेन` marker |
| Comparison operators | ✅ | ✅ | समः, अधिकः, etc. |
| Logical operators | ✅ | ✅ | च, वा (every item takes its own particle, the last one too) |
| JOINs (INNER, LEFT, RIGHT, FULL, CROSS) | ✅ | ✅ | सह + तृतीया |
| GROUP BY | ✅ | ✅ | समूहेन |
| ORDER BY | ✅ | ✅ | क्रमेण |
| LIMIT | ✅ | ✅ | मित्या |
| Aggregates | ✅ | ✅ | योगः, माध्यम्, etc. |
| Column aliases | ✅ | ✅ | `<expr> <alias> इति आख्या` |
| Arithmetic | ✅ | ✅ | योगः, गुणनम्, etc. |
| NULL checks | ✅ | ✅ | रिक्तम् अस्ति/नास्ति |
| CREATE TABLE / DROP / ALTER | ✅ | ✅ | native DDL, इति आख्या ... रच्यताम् |
| CREATE INDEX / DROP INDEX | ✅ | ✅ | `सूची ... रच्यताम्` / `लुप्यताम्` |
| INSERT | ✅ | ✅ | `निवेश्यताम्` |
| UPDATE | ✅ | ✅ | `परिवर्त्यताम्` |
| DELETE | ✅ | ✅ | `निष्कास्यताम्` |
| Window functions (`RANK`, `LAG`, `LEAD`, `NTILE`, frames) | ✅ | ✅ | `श्रेणी`, `पूर्वपङ्क्तेः`, `अनन्तरपङ्क्तेः`, `चतुर्थांशाङ्कः`, `पङ्क्तयः ... आरभ्य` |
| CTEs & Recursive CTEs | ✅ | ✅ | `<name> इति आख्या ...` / `आवर्तिका सञ्ज्ञा` |
| Self-joins & Views | ✅ | ✅ | a CTE names the second role / `इति दृश्यम्` |
| Subqueries (`WHERE`, `IN`, `EXISTS`, `FROM`) | ✅ | ✅ | `यत् ... इति` |
| CASE expressions | ✅ | ✅ | `वरणम् यस्मिन् ... तस्मिन् ... अन्यस्मिन्` |
| DISTINCT & OFFSET | ✅ | ✅ | `पुनरुक्तिम् विना भाषा` / `विशिष्टायाः भाषायाः गणना`, `<n> अपसारेण` |
| Set operations (`UNION`, `UNION ALL`, `INTERSECT`, `EXCEPT`) | ✅ | ✅ | `सङ्गमः`, `सर्वसङ्गमः`, `प्रतिच्छेदः`, `वर्जनम्` |
| String, Date/Time & Null-handling builtins | ✅ | ✅ | `महावर्णरूपम्`, `खण्डः`, `वर्षम्`, `होरा`, `विकल्पः` |
| Transactions (`BEGIN`, `COMMIT`, `ROLLBACK`, `SAVEPOINT`) | ✅ | ✅ | `व्यवहारः प्रारभ्यताम्`, `व्यवहारः स्थिरीक्रियताम्`, `व्यवहारः प्रत्याह्रियताम्`, `सङ्ग्रहबिन्दुः स्थाप्यताम्` |
| Pure user-defined functions | ✅ | ✅ | `शुद्धम् कार्यम् ... रच्यताम्`, inlined at each call |
| Type & Kāraka Concordance Checking | N/A | ✅ | Strict validator (`src/validator.rs`) |
| Dialect adaptors | N/A | ✅ | SQLite, PostgreSQL |
| Formatter & linter | N/A | ✅ | `ssql_compiler --fmt`, `ssql_compiler --lint` |
| SQL → SSQL transpiler | N/A | ✅ | `ssql_compiler --from-sql <file.sql> --schema schema.सं` |

### Not Yet Implemented 🔄

| Feature | Status | Description |
|---------|--------|-------------|
| **GRANT / REVOKE** | Not started | The verbs `अनुमन्यताम्` (GRANT) and `प्रतिसंह्रियताम्` (REVOKE; `निवर्त्यताम्` is also accepted) are reserved by the lexer; there is no parser rule yet |
| **Date arithmetic** (`date - INTERVAL '90 days'`) | Not started | Dates can be compared and their parts extracted, but not shifted; arithmetic on a date column is a compile error, since the two databases read it differently |

### Known differences between SQLite and PostgreSQL

The compiler aims at one meaning for both dialects, and for these it emits the same results:

- date parts are integers, and `अन्तरम्` is whole days to today in UTC;
- LIKE ignores case and reads a backslash as an ordinary character;
- a recursive CTE's cycle guard is the same exact search of the path;
- stored text compares and sorts by byte value (`COLLATE "C"`);
- integers are 64-bit;
- division, a remainder or a whole quotient by zero, a negative square root, a logarithm of 0 and an arc sine beyond ±1 are NULL, not an error;
- ROUND rounds half away from zero and takes a number of places on any number;
- a cast to a whole number drops the fraction;
- a comparison read as a value is 1 or 0;
- the current moment is UTC;
- date and time functions keep whole seconds, dropping a fraction, and the time of a date is midnight.

These remain different:

- Identifiers are emitted unquoted, so a mixed-case physical alias is folded to lower case on PostgreSQL (a physical name that is an SQL reserved word, or not a plain name, is a compile error).
- `वास्तविकः` is REAL (a binary float) on SQLite and NUMERIC (exact) on PostgreSQL, so `०.१ च ०.२ च इत्यनयोः योगः` is 0.30000000000000004 on one and 0.3 on the other. SUM and AVG are cast to DOUBLE PRECISION on PostgreSQL, so an integer sum is a float there, and ROUND returns a NUMERIC (`32.00` where SQLite shows `32.0`).
- LIKE on a stored text column folds only ASCII letters on both, since the column is `COLLATE "C"`; on a value with no column behind it, such as a literal, PostgreSQL's ILIKE folds every letter.
- `substr` with a negative start differs.
- A RANGE frame with an offset needs a numeric ordering key on PostgreSQL.
- A regular-expression match (REGEXP) needs a `regexp` function registered by the application on SQLite; PostgreSQL has its own (`~`).
- Rows that tie on every ordering key come back in either database's own order.
- A cast of text that is not a number is 0 on SQLite and an error on PostgreSQL.

### Out of Scope (Handled by General-Purpose Sanskrit Language / Sopanam)

| Feature | Rationale | Alternative |
|---------|-----------|-------------|
| Imperative Loops / Mutable Variables | Imperative | Use Sopanam |
| Event-Driven Triggers | Procedural | Use native DB triggers |

## Syntax Notes

### User Defined Functions (Implemented)

A pure function is created like any other object, with `रच्यताम्`:

```text
<नाम> इति शुद्धम् कार्यम् रच्यताम् यत्र <परिणामप्रकारः> प्रतिदीयते च
    <परामितिः> इति <प्रकारः> च … दीयते / दीयेते / दीयन्ते
    <व्यञ्जकम्> इति उत्तरम् प्रतिदीयताम्
इति कार्यम् ।
```

**Example:**
```ssql
करसहितमूल्यम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
    मूल्यम् इति वास्तविकः च करः इति वास्तविकः च दीयेते
    मूल्यम् च करः च इत्यनयोः योगः इति उत्तरम् प्रतिदीयताम्
इति कार्यम् ।

पुस्तकेभ्यः शीर्षकम् च मूल्यम् च ५ च प्रदाय करसहितमूल्यम् सकलमूल्यम् इति आख्या च आह्रियेताम् ।
```

**Key Grammar Points:**
- `रच्यताम्` = "let it be created" (a function is a CREATE like any other)
- `<प्रकारः> प्रतिदीयते` = "is given back" (the return type)
- `यत्र … दीयते / दीयेते / दीयन्ते` = "where … is / are given" (the parameters; the verb agrees with their number)
- `… इति उत्तरम् प्रतिदीयताम्` = "let … be given back as the answer" (the body)
- `इति कार्यम्` = end of the function
- A call takes one argument in the genitive (`मूल्यस्य द्विगुणितम्`) and several with `प्रदाय` ("having supplied"); the body is inlined into the SQL at each call

---

### LSP (Language Server Protocol) — implemented

An SSQL language server (`ssql-lsp`) and a VS Code extension (`editors/vscode/`)
already ship in this repo, providing:
- Syntax highlighting for `.सं` files
- Hover information (symbol kind, resolved column/table, description)
- Basic completion from the active schema

### Subqueries (Implemented)

```ssql
पुस्तकेभ्यः
    मूल्यम् यत् पुस्तकेभ्यः मूल्यस्य माध्यम् आह्रियताम् इति इत्यस्मात् अधिकम् अस्ति इति नियमेन
    शीर्षकम् आह्रियताम् ।
```

**SQL:**
```sql
SELECT title FROM books 
WHERE price > (SELECT AVG(price) FROM books);
```

### Transactions (Implemented)

```ssql
व्यवहारः प्रारभ्यताम् ।
पुस्तकेषु सङ्ख्या १ इत्यनया समा अस्ति इति नियमेन मूल्यम् ६० इति परिवर्त्यताम् ।
"sp1" इति सङ्ग्रहबिन्दुः स्थाप्यताम् ।
पुस्तकेषु सङ्ख्या २ इत्यनया समा अस्ति इति नियमेन मूल्यम् ७० इति परिवर्त्यताम् ।
"sp1" इति सङ्ग्रहबिन्दुम् प्रति व्यवहारः प्रत्याह्रियताम् ।
व्यवहारः स्थिरीक्रियताम् ।
```

This compiles to `BEGIN`, a `SAVEPOINT sp1`, `ROLLBACK TO SAVEPOINT sp1` and
`COMMIT`. A whole transaction is abandoned (ROLLBACK) with:

```ssql
व्यवहारः प्रत्याह्रियताम् ।
```

## Integration Features

### Build System

```bash
# Compile one file to SQL (auto-discovers the sibling schema.सं)
ssql_compiler examples/university_bookstore/SAMSQL/01_books_published_after_2010.सं --dialect postgres -o out.sql

# Compile and check every example project
uv run verify_all.py --dialect sqlite
```

### IDE Support

- Syntax highlighting for .सं files — ✅ shipped (`editors/vscode/`)
- Hover for vibhakti explanations — ✅ shipped
- Inline diagnostics: compile errors (including agreement and type errors) and lint warnings — ✅ shipped
- Go to definition and document formatting — ✅ shipped
- Richer autocomplete from the active schema — still open

### SQL Import

```bash
# Reverse-transpile an existing SQL file into SSQL
ssql_compiler --from-sql legacy.sql --schema schema.सं -o query.सं
```
