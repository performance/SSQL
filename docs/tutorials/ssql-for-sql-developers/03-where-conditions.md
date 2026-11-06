# Chapter 3: WHERE and Conditions

## Basic WHERE Clause

In SSQL, conditions use the pattern: `condition इति नियमेन`

**SQL:**
```sql
SELECT * FROM books WHERE price > 50;
```

**SSQL:**
```ssql
पुस्तकेभ्यः 
    मूल्यम् ५० इत्यस्मात् अधिकम् अस्ति इति नियमेन 
    सर्वम्
आह्रियताम् ।
```

## Comparison Operators

| Operator | Pattern | Example |
|----------|---------|---------|
| = | `X Y इत्यनेन समम् अस्ति` | `भाषा "English" इत्यनया समा अस्ति` |
| != | `X Y इत्यनेन समम् नास्ति` | `भाषा "Hindi" इत्यनया समा नास्ति` |
| > | `X Y इत्यस्मात् अधिकम् अस्ति` | `मूल्यम् १०० इत्यस्मात् अधिकम् अस्ति` |
| < | `X Y इत्यस्मात् न्यूनम् अस्ति` | `मूल्यम् ५० इत्यस्मात् न्यूनम् अस्ति` |
| LIKE | `X Y इत्यनेन सदृशम् अस्ति` | `नाम "%कालि%" इत्यनेन सदृशम् अस्ति` |

## Logical Operators

### AND (च)

```ssql
पुस्तकेभ्यः
    मूल्यम् ५० इत्यस्मात् अधिकम् अस्ति च
    भाषा "Sanskrit" इत्यनया समा अस्ति च
    इति नियमेन
    सर्वम्
आह्रियताम् ।
```

**SQL:**
```sql
SELECT * FROM books WHERE price > 50 AND language = 'Sanskrit';
```

### OR (वा)

```ssql
पुस्तकेभ्यः
    भाषा "English" इत्यनया समा अस्ति वा
    भाषा "Hindi" इत्यनया समा अस्ति वा
    इति नियमेन
    सर्वम्
आह्रियताम् ।
```

**SQL:**
```sql
SELECT * FROM books WHERE language = 'English' OR language = 'Hindi';
```

### Grouping Compound Conditions Without Parentheses (`यत् ... इति`)

Because SSQL avoids ASCII punctuation such as `( )`, nested boolean expressions like `WHERE (A OR B) AND C` are grouped using the classical Sanskrit relative-quotative frame **`यत् ... इति`**:

```ssql
पुस्तकेभ्यः
    यत् भाषा "Sanskrit" इत्यनया समा अस्ति वा भाषा "Hindi" इत्यनया समा अस्ति वा इति च
    मूल्यम् १०० इत्यस्मात् न्यूनम् अस्ति च
    इति नियमेन
    शीर्षकम् च भाषा च मूल्यम् च
आह्रियन्ताम् ।
```

**SQL:**
```sql
SELECT
    t1.title,
    t1.language,
    t1.price
FROM
    books t1
WHERE
    ((t1.language = 'Sanskrit' OR t1.language = 'Hindi') AND t1.price < 100);
```

## NULL Checks

| SQL | SSQL |
|-----|------|
| IS NULL | `रिक्तम् अस्ति` (neuter) / `रिक्ता अस्ति` (feminine) / `रिक्तः अस्ति` (masculine) |
| IS NOT NULL | `रिक्तम् नास्ति` |

**Example:**
```ssql
विक्रयाधिकारिभ्यः
    प्रबन्धकसङ्ख्या रिक्ता अस्ति इति नियमेन
    नाम
आह्रियताम् ।
```

**SQL:**
```sql
SELECT name FROM sales_reps WHERE manager_id IS NULL;
```

## LIKE with Devanagari Strings

SSQL supports Devanagari string constants:

```ssql
लेखकेभ्यः
    नाम "कालि%" इत्यनेन सदृशम् अस्ति इति नियमेन
    नाम
आह्रियताम् ।
```

**SQL:**
```sql
SELECT name FROM authors WHERE name LIKE 'कालि%';
```

`सदृशम्` ignores the case of ASCII letters on both databases: it compiles to `LIKE` on SQLite and `ILIKE` on PostgreSQL.

## Next Steps

- [Chapter 4: JOINs](04-joins.md)
