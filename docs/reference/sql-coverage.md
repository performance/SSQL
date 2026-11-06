# SQL → SSQL Feature Coverage Matrix

## Core DQL (Data Query Language)

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| SELECT columns | `SELECT a, b` | ✅ Implemented | ✅ |
| SELECT * | `SELECT *` | ✅ Implemented | ✅ |
| Column aliases | `SELECT a AS b` | ✅ Implemented (`<expr> <alias> इति आख्या`) | ✅ |
| FROM clause | `FROM table` | ✅ Implemented | ✅ |
| Table aliases | `FROM t AS alias` | ✅ Not needed: a column names its table in the genitive singular (`पुस्तकस्य शीर्षकम्`); a second role of one table is a CTE | ✅ |
| WHERE clause | `WHERE x = 1` | ✅ Implemented | ✅ |
| AND/OR | `WHERE a AND b` | ✅ Implemented | ✅ |
| Comparison ops | `=, !=, <, >, <=, >=` | ✅ Implemented | ✅ |
| LIKE | `WHERE name LIKE '%x%'` | ✅ Implemented (`इत्यनेन सदृशम् अस्ति`; `सदृशः` / `सदृशी` / `सदृशम्` agree with the column) | ✅ |
| IS NULL | `WHERE x IS NULL` | ✅ Implemented (`रिक्तम् अस्ति`, agreeing with the column) | ✅ |
| IS NOT NULL | `WHERE x IS NOT NULL` | ✅ Implemented (`रिक्तम् नास्ति`) | ✅ |
| IN list | `WHERE x IN (1,2,3)` | ✅ Implemented (`१ च २ च ३ च इत्येषु अस्ति`; feminine `इत्यासु`, two values `इत्यनयोः`) | ✅ |
| BETWEEN | `WHERE x BETWEEN 1 AND 10` | ✅ Implemented (`१ च १० च इत्यनयोः मध्ये अस्ति`) | ✅ |
| ORDER BY | `ORDER BY x ASC` | ✅ Implemented (`<col> आरोहः इति क्रमेण`, before the select list) | ✅ |
| ORDER BY DESC | `ORDER BY x DESC` | ✅ Implemented (`अवरोहः इति क्रमेण`) | ✅ |
| LIMIT | `LIMIT 10` | ✅ Implemented (`१० इति मित्या`) | ✅ |
| OFFSET | `LIMIT 10 OFFSET 5` | ✅ Implemented (`१० इति मित्या ५ अपसारेण`) | ✅ |
| DISTINCT | `SELECT DISTINCT` | ✅ Implemented (`पुनरुक्तिम् विना` before the select list, "without repetition"; in an aggregate `विशिष्टायाः भाषायाः गणना`) | ✅ |

## JOINs

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| INNER JOIN | `JOIN t ON ...` | ✅ Implemented (`<table> सह … इति मेलनेन`) | ✅ |
| LEFT JOIN | `LEFT JOIN t ON ...` | ✅ Implemented (`इति वाममेलनेन`) | ✅ |
| RIGHT JOIN | `RIGHT JOIN t ON ...` | ✅ Implemented (`इति दक्षिणमेलनेन`) | ✅ |
| FULL OUTER JOIN | `FULL OUTER JOIN` | ✅ Implemented (`इति पूर्णमेलनेन`) | ✅ |
| CROSS JOIN | `CROSS JOIN t` | ✅ Implemented (`<table> सह सर्वमेलनेन`, no condition) | ✅ |
| Self-join | `JOIN t AS t2` | ✅ Implemented (a CTE names the second role) | ✅ |
| Multiple JOINs | `JOIN a ... JOIN b ...` | ✅ Implemented | ✅ |

## Aggregates

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| COUNT | `COUNT(*)` | ✅ Implemented (`पङ्क्तीनाम् गणना`; `COUNT(col)` is `<col genitive> गणना`) | ✅ |
| SUM | `SUM(col)` | ✅ Implemented (`योगः`) | ✅ |
| AVG | `AVG(col)` | ✅ Implemented (`माध्यम्`) | ✅ |
| MAX | `MAX(col)` | ✅ Implemented (`महत्तमम्`) | ✅ |
| MIN | `MIN(col)` | ✅ Implemented (`लघुतमम्`) | ✅ |
| GROUP BY | `GROUP BY col` | ✅ Implemented (`इति समूहेन`) | ✅ |
| HAVING | `HAVING count > 5` | ✅ Implemented (`इति समूहनियमेन`) | ✅ |

## Subqueries

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| Scalar subquery | `WHERE x = (SELECT ...)` | ✅ Implemented (`यत् ... इति`) | ✅ |
| IN subquery | `WHERE x IN (SELECT ...)` | ✅ Implemented (`यत् ... इति इत्येषु अस्ति`; feminine `इत्यासु`) | ✅ |
| EXISTS | `WHERE EXISTS (SELECT ...)` | ✅ Implemented (`यत् ... इति विद्यते`) | ✅ |
| Derived table | `FROM (SELECT ...) AS t` | ✅ Implemented (`यत् ... इति आश्रित्य`) | ✅ |
| Correlated | `WHERE x = (SELECT ... WHERE t.id = ...)` | ✅ Implemented | ✅ |

## CTEs (Common Table Expressions)

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| Simple CTE | `WITH cte AS (...)` | ✅ Implemented (`<name> इति आख्या <stem> इति <gender> <number> <query> ।`) | ✅ |
| Multiple CTEs | `WITH a AS (...), b AS (...)` | ✅ Implemented | ✅ |
| Recursive CTE | `WITH RECURSIVE ...` | ✅ Implemented (`आवर्तिका सञ्ज्ञा`) | ✅ |

## Window Functions

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| RANK() / DENSE_RANK() | `RANK() OVER (...)` | ✅ Implemented (`श्रेणी` / `सघनश्रेणी`) | ✅ |
| ROW_NUMBER() / NTILE() | `ROW_NUMBER() OVER (...)` | ✅ Implemented (`पङ्क्तिसङ्ख्या` / `चतुर्थांशाङ्कः`, `<n> भागेषु भागाङ्कः`) | ✅ |
| PARTITION BY | `OVER (PARTITION BY ...)` | ✅ Implemented (`आधारेण विभाजनम्`) | ✅ |
| ORDER BY in OVER | `OVER (ORDER BY ...)` | ✅ Implemented (`अनुसारम् आरोहक्रमेण`) | ✅ |
| Running total & Frames | `SUM() OVER (ROWS BETWEEN ...)` | ✅ Implemented (`पङ्क्तयः ... आरभ्य ... पर्यन्तम्`) | ✅ |
| LAG/LEAD | `LAG(col) OVER (...)` | ✅ Implemented (`पूर्वपङ्क्तेः` / `अनन्तरपङ्क्तेः`) | ✅ |

## CASE Expressions

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| Simple CASE | `CASE x WHEN 1 THEN 'a' END` | ✅ Written as a searched CASE with equality conditions (`वरणम् यस्मिन् … इत्यनेन समम् अस्ति तस्मिन् …`) | ✅ |
| Searched CASE | `CASE WHEN x > 1 THEN 'a' END` | ✅ Implemented (`वरणम् यस्मिन् ... तस्मिन्`) | ✅ |
| ELSE | `CASE ... ELSE 'default' END` | ✅ Implemented (`अन्यस्मिन्`) | ✅ |

## Set Operations

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| UNION | `SELECT ... UNION SELECT ...` | ✅ Implemented (`… च … च इत्यनयोः सङ्गमः`) | ✅ |
| UNION ALL | `... UNION ALL ...` | ✅ Implemented (`सर्वसङ्गमः`) | ✅ |
| INTERSECT | `... INTERSECT ...` | ✅ Implemented (`प्रतिच्छेदः`) | ✅ |
| EXCEPT | `... EXCEPT ...` | ✅ Implemented (`वर्जनम्`) | ✅ |

## DML (Data Manipulation)

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| INSERT VALUES | `INSERT INTO t VALUES (...)` | ✅ Implemented (`निवेश्यताम्`) | ✅ |
| INSERT SELECT | `INSERT INTO t SELECT ...` | ✅ Implemented | ✅ |
| UPDATE | `UPDATE t SET x = 1` | ✅ Implemented (`परिवर्त्यताम्`) | ✅ |
| DELETE | `DELETE FROM t WHERE ...` | ✅ Implemented (`निष्कास्यताम्`) | ✅ |

## Data Types / Expressions

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| Integer literals | `123` | ✅ Implemented (`१२३`, Devanagari digits) | ✅ |
| Decimal literals | `123.45` | ✅ Implemented (`१२३.४५`, Devanagari digits) | ✅ |
| ASCII strings | `'hello'` | ✅ Implemented | ✅ |
| Devanagari strings | `'कालिदास'` | ✅ Implemented | ✅ |
| Arithmetic | `a + b - c * d / e` | ✅ Implemented (`योगः`, `विशोध्य अवशिष्टम्`, `गुणनम्`, `विभज्य लब्धम्`; compound operands and aggregates by the noun's case, `योगात्` / `योगम्` / `योगेन` / `योगस्य`, a grouped remainder by `इतीदम्`) | ✅ |
| String concat | `a || b` | ✅ Implemented (`संयोजनम्`) | ✅ |
| CAST | `CAST(x AS INTEGER)` | ✅ Implemented (`मूल्यस्य पूर्णाङ्करूपेण परिणामः`; `वाक्यरूपेण`, `वास्तविकरूपेण`, `दिनाङ्करूपेण`, `द्वैतरूपेण`) | ✅ |
| COALESCE | `COALESCE(x, 0)` | ✅ Implemented (`विकल्पः`) | ✅ |
| NULLIF | `NULLIF(x, 0)` | ✅ Implemented (`अनावृत्तम्`) | ✅ |

## Date/Time Functions

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| Extract YEAR | `EXTRACT(YEAR FROM d)` / `strftime('%Y', d)` | ✅ Implemented (`वर्षम्`) | ✅ |
| Extract MONTH | `EXTRACT(MONTH FROM d)` / `strftime('%m', d)` | ✅ Implemented (`मासः`) | ✅ |
| Extract DAY / HOUR | `EXTRACT(DAY/HOUR FROM d)` | ✅ Implemented (`दिनम्` / `होरा`) | ✅ |
| Date comparison | `WHERE date = '2024-01-01'` | ✅ Implemented (incl. Devanagari date normalization) | ✅ |
| Current date / time | `CURRENT_DATE` / `NOW()` | ✅ Implemented (`अद्यतनदिनाङ्कः` / `अद्यतनक्षणः`) | ✅ |
| Date arithmetic | `date - INTERVAL '90 days'` | ❌ Not implemented | ❌ |

## DDL & TCL (Data Definition & Transactions)

| Feature | SQL Example | SSQL Status | Tested |
|---------|-------------|-------------|--------|
| CREATE TABLE / VIEW | `CREATE TABLE t (...)` | ✅ Implemented (`इति पटलम्` / `इति दृश्यम्` … `रच्यताम्`). A view's columns are its query's; declare it in `schema.सं` to read it from other files, as `examples/10_mavenflix_churn` does. | ✅ |
| DROP TABLE / VIEW | `DROP TABLE t` | ✅ Implemented (`लुप्यताम्`) | ✅ |
| CREATE / DROP INDEX | `CREATE INDEX ...` | ✅ Implemented (`सूची ... रच्यताम्`) | ✅ |
| ALTER TABLE | `ALTER TABLE t ADD ...` | ✅ Implemented (`योज्यताम्` ADD COLUMN, `लुप्यताम्` DROP COLUMN, `तस्मात् … अभिधीयताम्` RENAME) | ✅ |
| BEGIN / COMMIT / ROLLBACK | `BEGIN; ... COMMIT;` | ✅ Implemented (`व्यवहारः प्रारभ्यताम्` / `स्थिरीक्रियताम्` / `प्रत्याह्रियताम्`; SAVEPOINT `"<name>" इति सङ्ग्रहबिन्दुः स्थाप्यताम्`) | ✅ |

---

## Summary

| Category | Implemented | Partial | Not Implemented |
|----------|-------------|---------|-----------------|
| Core DQL | 18 | 0 | 0 |
| JOINs | 7 | 0 | 0 |
| Aggregates | 7 | 0 | 0 |
| Subqueries | 5 | 0 | 0 |
| CTEs | 3 | 0 | 0 |
| Window Functions | 6 | 0 | 0 |
| CASE | 3 | 0 | 0 |
| Set Operations | 4 | 0 | 0 |
| DML | 4 | 0 | 0 |
| Expressions | 9 | 0 | 0 |
| Date/Time | 5 | 0 | 1 |
| DDL & TCL | 5 | 0 | 0 |
| **Total** | **76** | **0** | **1** |
