# Chapter 1: Introduction to SSQL

## What is SSQL?

**SSQL is a subset of Samskrutam (Sanskrit)** designed for expressing database queries. The design goal is that valid SSQL code reads as natural Sanskrit prose, with only a few additional graphemes (numerals, operators) beyond standard Devanagari.

SSQL compiles to standard SQL dialects (SQLite, PostgreSQL, etc.).

## Why Sanskrit?

Sanskrit has a highly systematic grammar with **विभक्ति** (Vibhakti - case endings) that naturally map to SQL semantics:

| Vibhakti | Sanskrit Name | SQL Role |
|----------|---------------|----------|
| प्रथमा | Nominative | SELECT columns, subjects |
| द्वितीया | Accusative | Direct objects |
| तृतीया | Instrumental | WHERE, GROUP BY, ORDER BY |
| चतुर्थी | Dative | GRANT TO recipients (reserved; not yet parsed) |
| पञ्चमी | Ablative | FROM source tables |
| षष्ठी | Genitive | Property access (date's month) |
| सप्तमी | Locative | INSERT INTO, UPDATE IN |

## Your First Query

**SQL:**
```sql
SELECT name, email FROM customers;
```

**SSQL:**
```ssql
ग्राहकेभ्यः नाम च विद्युत्पत्रम् च आह्रियेताम् ।
```

Let's break this down:

| SSQL | Role | Explanation |
|------|------|-------------|
| `ग्राहकेभ्यः` | FROM | ग्राहक (customer) + एभ्यः (पञ्चमी बहुवचन - ablative plural) |
| `नाम` | SELECT | name (प्रथमा - nominative) |
| `च` | AND | conjunction |
| `विद्युत्पत्रम्` | SELECT | email (प्रथमा - nominative) |
| `आह्रियेताम्` | FETCH | 3rd person dual passive imperative ("let the two be fetched") |

## The Schema: Native DDL

Schemas are declared directly in SSQL, using `इति आख्या ... रच्यताम् यत्र ... इति पटलम् ।` (`CREATE TABLE`) syntax — there is no separate mapping file:

```ssql
ग्राहकाः इति पटलम् "customers" इति आख्या ग्राहक इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    सङ्ख्या इति प्राथमिकी क्रमिका पूर्णाङ्कः "id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    नाम इति अनिवार्यम् वाक्यम् "name" इति आख्या नामन् इति नपुंसकलिङ्गम् एकवचनम् च
    विद्युत्पत्रम् इति अद्वितीयम् वाक्यम् "email" इति आख्या विद्युत्पत्र इति नपुंसकलिङ्गम् एकवचनम् च
  इति पटलम् ।
```

### Column Data Types & Constraints

Every column declaration follows the pattern `<ColumnNominative> इति [Constraints...] <DataType> "<sql_name>" इति आख्या <Stem> इति <Gender> <Number>`, separated by `च`:

| SSQL Keyword | SQL Equivalent | Notes |
|--------------|----------------|-------|
| `पूर्णाङ्कः` | `INTEGER` | Whole numbers |
| `वास्तविकः` | `REAL` / `NUMERIC` | Floating-point / decimal numbers |
| `वाक्यम्` | `TEXT` / `VARCHAR` | String / text columns |
| `दिनाङ्कः` | `DATE` | Calendar date (`"२०२५-०१-१५"`) |
| `प्राथमिकः` / `प्राथमिकी` / `प्राथमिकम्` | `PRIMARY KEY` | Primary key constraint |
| `क्रमिकः` / `क्रमिका` / `क्रमिकम्` | `AUTOINCREMENT` | Auto-increment constraint (`प्राथमिकी क्रमिका सङ्ख्या`) |
| `अनिवार्यः` / `अनिवार्या` / `अनिवार्यम्` | `NOT NULL` | Required column (inflected to agree with the column noun's gender) |
| `वैकल्पिकः` / `वैकल्पिकी` / `वैकल्पिकम्` | `NULL` | Optional column |
| `अद्वितीयः` / `अद्वितीया` / `अद्वितीयम्` | `UNIQUE` | Unique constraint |
| `<table, locative plural> बद्धः` / `बद्धा` / `बद्धम्` ("tied to") | `REFERENCES <table>` | Foreign key constraint (e.g., `लेखकसङ्ख्या इति पूर्णाङ्कः लेखकेषु बद्धा "author_id" इति आख्या ...`) |

> [!NOTE]
> **Constraint Placement & Gender Agreement:** Column constraints may be placed either before the column name (`प्राथमिकी क्रमिका सङ्ख्या इति पूर्णाङ्कः`), after `इति` (`सङ्ख्या इति प्राथमिकी क्रमिका पूर्णाङ्कः`), or after the data type (`लेखकसङ्ख्या इति पूर्णाङ्कः लेखकेषु बद्धा`). Each agrees with the column noun in gender and number: `सङ्ख्या` is feminine, so `प्राथमिकी क्रमिका`; the compiler checks it.

> [!WARNING]
> **Reserved Keywords Cannot Be Used as Table or Column Stems**
> Built-in SSQL grammar and window/analytical keywords are reserved and cannot be used as table, column, view, or CTE stems (`प्रातिपदिकम्`). Common words to avoid as column stems include:
> - `विभाग` (reserved for `NTILE` / window partitioning) — use **`शाखा`** (`शाखाः` / `शाखा`) for "department/branch" instead.
> - `बाह्या` (reserved for `बाह्या सञ्ज्ञा` external parameters)
> - `क्रम`, `श्रेणी`, `विभाजन` (reserved for window `ORDER BY`, frames, and `PARTITION BY`)
> - `सङ्कुल`, `कार्य`, `क्रिया` (reserved for modules and UDF declarations)

## Data Manipulation Language (DML): `INSERT`, `UPDATE`, `DELETE`

SSQL uses the **Locative Plural (`सप्तमी बहुवचनम्`, `-एषु` / `-आसु`)** for `INSERT INTO` and `UPDATE`, and the **Ablative Plural (`पञ्चमी बहुवचनम्`, `-एभ्यः` / `-आभ्यः`)** for `DELETE FROM`:

### 1. `INSERT INTO` (`निवेश्यताम्` / `निवेश्येताम्` / `निवेश्यन्ताम्`)
Place the target table in the **Locative Plural** (`पुस्तकेषु` = "into books"), list each `<ColumnNominative> <Value>` pair followed by `च`, and end with the passive verb. The columns are the verb's subject, so - exactly as with `आह्रियताम्` in a `SELECT` - its number agrees with how many there are: `निवेश्यताम्` for one column, `निवेश्येताम्` for two, `निवेश्यन्ताम्` for three or more:

```ssql
पुस्तकेषु सङ्ख्या १०१ च शीर्षकम् "अष्टाध्यायी" च अङ्कः "978-0000000101" च भाषा "Sanskrit" च मूल्यम् ५० च निवेश्यन्ताम् ।
```

### 2. `UPDATE` (`इति परिवर्त्यताम्`)
Place the target table in the **Locative Plural** (`पुस्तकेषु`), filter with `इति नियमेन` (`WHERE`), give each new value as `<ColumnNominative> <NewValue> इति` (`इति` marks the value as the column's new content), and end with `परिवर्त्यताम् ।` ("let it be changed"):

```ssql
पुस्तकेषु सङ्ख्या १०१ इत्यनया समा अस्ति इति नियमेन मूल्यम् ६० इति परिवर्त्यताम् ।
```

The verb agrees in number with the columns it changes. Several columns form a `च`-list, each with its own `च`, and take the dual or plural, `परिवर्त्येताम्` / `परिवर्त्यन्ताम्`:

```ssql
पुस्तकेषु सङ्ख्या १०१ इत्यनया समा अस्ति इति नियमेन मूल्यम् ६० इति च भाषा "पालि" इति च परिवर्त्येताम् ।
```

An `UPDATE` always says which rows it changes. To change every row, say so with `सर्वत्र` ("everywhere"); an `UPDATE` with neither a filter nor `सर्वत्र` is an error:

```ssql
पुस्तकेषु सर्वत्र मूल्यम् ६० इति परिवर्त्यताम् ।
```

### 3. `DELETE FROM` (`निष्कास्यताम्`)
Place the source table in the **Ablative Plural** (`पुस्तकेभ्यः` = "from books"), filter with `इति नियमेन`, and end with `निष्कास्यताम् ।`:

```ssql
पुस्तकेभ्यः सङ्ख्या १०१ इत्यनया समा अस्ति इति नियमेन निष्कास्यताम् ।
```

A `DELETE` always says what it removes. To empty a table, say so explicitly with `सर्वम्` ("let everything be removed from the books"); a `DELETE` with neither a filter nor `सर्वम्` is an error, so a forgotten filter can never empty a table:

```ssql
पुस्तकेभ्यः सर्वम् निष्कास्यताम् ।
```

## Key Principles

1. **Graphemic Purity**: Only Devanagari script in .सं files
2. **Grammar-Driven**: Vibhakti determines SQL semantics
3. **Compilation**: SSQL → SQL at build time, not runtime
4. **Type Safety**: Column types declared in the schema DDL enable compile-time checks

## Next Steps

- [Chapter 2: SELECT Basics](02-select-basics.md) - Columns, aliases, and expressions
