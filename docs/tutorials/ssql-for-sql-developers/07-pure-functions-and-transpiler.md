# Chapter 7: Pure User-Defined Functions & Tooling

## Pure User-Defined Functions (`शुद्धम् कार्यम्`)

SSQL supports native, side-effect-free User-Defined Functions (`शुद्धम् कार्यम्`) inspired by the *Sopānam* Sanskrit programming specification. Pure UDFs are expanded at compile time into standard SQL expressions across both SQLite and PostgreSQL.

### Declaring a Pure UDF

The function is created with `रच्यताम्`, like every other object. Its return type is stated with `प्रतिदीयते` ("is returned"), its parameters with the passive `दीयते` ("is given"), which agrees in number (`वचन`) with the parameter count, and its result with `इति उत्तरम् प्रतिदीयताम्` ("let the answer be returned"):
- **1 parameter (Ekavachana):** `दीयते` ("is given")
- **2 parameters (Dvivachana):** `दीयेते` ("both are given")
- **3+ parameters (Bahuvachana):** `दीयन्ते` ("are given")

At the call site the arguments are supplied with `प्रदाय` ("having supplied").

```ssql
द्विगुणितम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
    मूल्यम् इति वास्तविकः दीयते
    मूल्यम् च २ च इत्यनयोः गुणनम् इति उत्तरम् प्रतिदीयताम्
इति कार्यम् ।

न्यूनीकृतम् इति शुद्धम् कार्यम् रच्यताम् यत्र वास्तविकः प्रतिदीयते च
    मूल्यम् इति वास्तविकः च मूल्यह्रासः इति वास्तविकः च दीयेते
    मूल्यात् मूल्यह्रासम् विशोध्य अवशिष्टम् इति उत्तरम् प्रतिदीयताम्
इति कार्यम् ।

पुस्तकेभ्यः
    शीर्षकम् च
    मूल्यस्य द्विगुणितम् द्विगुणितमूल्यम् इति आख्या च
    मूल्यम् च १० च प्रदाय न्यूनीकृतम् न्यूनीकृतमूल्यम् इति आख्या च
आह्रियन्ताम् ।
```

### Calling Conventions

1. **Single-Argument Genitive (`षष्ठी`):**
   - `मूल्यस्य द्विगुणितम्` ("the doubled value of price")
   - `मूल्यम् प्रदाय द्विगुणितम्` ("having supplied price, doubled")
2. **Multi-Argument Positional (`प्रदाय`):**
   - `मूल्यम् च १० च प्रदाय न्यूनीकृतम्`
3. **Multi-Argument Named (`प्रदाय <param> इत्यस्य मूल्यम् <val> इति`):**
   - Allows arguments in any order with full Kāraka clarity.

## Developer Tooling (`--from-sql`, `--fmt`, `--lint`)

- **Reverse Transpiler (`--from-sql`):** Automatically translates existing `.sql` schemas and queries into strict-mode, Kāraka-concordant `.सं` files:
  ```sql
  -- ssql_compiler --from-sql legacy_queries.sql --schema schema.सं -o queries.सं
  SELECT title, price FROM books WHERE price > 25 ORDER BY price DESC LIMIT 10;
  ```
- **Canonical Formatter (`--fmt`):** Formats `.सं` files in place with canonical DDL, UDF, and DQL indentation (`ssql_compiler --fmt query.सं`).
- **Linguistic Purity Linter (`--lint`):** Flags ASCII numerals (`LINT-001`), legacy parentheses (`LINT-002`), word-final anusvāra instead of explicit halanta `म्` (`LINT-003`), and a missing daṇḍa `।` (`LINT-004`).

## Next Steps

- [Track B, Chapter 1: विभक्तिपरिचयः (Vibhakti Overview)](../samskrutam-through-ssql/01-vibhakti-overview.md)
