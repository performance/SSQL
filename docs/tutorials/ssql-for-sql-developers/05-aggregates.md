# Chapter 5: Aggregates and GROUP BY

## Aggregate Functions

SSQL supports standard SQL aggregate functions using Sanskrit nouns. When comparing an aggregate in a `HAVING` (`इति समूहनियमेन`) clause, the predicate adjective (`अधिकः`/`अधिका`/`अधिकम्`, `न्यूनः`/`न्यूना`/`न्यूनम्`, `समः`/`समा`/`समम्`) agrees with the **grammatical gender of the aggregate noun itself** (not the inner column):

| Function | Sanskrit | Gender (`लिङ्गम्`) | Predicate Agreement (`HAVING`) | Pattern |
|----------|----------|-------------------|--------------------------------|---------|
| SUM | `योगः` | पुंलिङ्गम् (Masculine) | `अधिकः` / `न्यूनः` / `समः` | `मूल्यस्य योगः` (an alias: `<alias> इत्यस्य योगः`) |
| AVG | `माध्यम्` | नपुंसकलिङ्गम् (Neuter) | `अधिकम्` / `न्यूनम्` / `समम्` | `मूल्यस्य माध्यम्` (an alias: `<alias> इत्यस्य माध्यम्`) |
| COUNT | `गणना` | स्त्रीलिङ्गम् (Feminine) | `अधिका` / `न्यूना` / `समा` | `सङ्ख्यायाः गणना` (or `पङ्क्तीनाम् गणना`, "the count of rows", for `COUNT(*)`) |
| MAX | `महत्तमम्` | नपुंसकलिङ्गम् (Neuter) | `अधिकम्` / `न्यूनम्` / `समम्` | `मूल्यस्य महत्तमम्` (an alias: `<alias> इत्यस्य महत्तमम्`) |
| MIN | `लघुतमम्` | नपुंसकलिङ्गम् (Neuter) | `अधिकम्` / `न्यूनम्` / `समम्` | `मूल्यस्य लघुतमम्` (an alias: `<alias> इत्यस्य लघुतमम्`) |

## Basic Aggregation

**SQL:**
```sql
SELECT SUM(total_amount) FROM sales;
```

**SSQL:**
```ssql
विक्रयणेभ्यः सकलमूल्यस्य योगः आह्रियताम् ।
```

## GROUP BY (समूहेन)

Use `समूहेन` (by group) to mark GROUP BY columns:

**SQL:**
```sql
SELECT language, AVG(price) 
FROM books 
GROUP BY language;
```

**SSQL:**
```ssql
पुस्तकेभ्यः 
    भाषा इति समूहेन 
    भाषा च मूल्यस्य माध्यम् माध्यमूल्यम् इति आख्या च 
आह्रियेताम् ।
```

## HAVING (Group Conditions)

HAVING conditions come after GROUP BY, using the `इति समूहनियमेन` marker:

**SQL:**
```sql
SELECT language, COUNT(*) as book_count
FROM books 
GROUP BY language
HAVING COUNT(*) > 2;
```

**SSQL:**
```ssql
पुस्तकेभ्यः
    भाषा इति समूहेन
    सङ्ख्यायाः गणना २ इत्यस्याः अधिका अस्ति इति समूहनियमेन
    भाषा च सङ्ख्यायाः गणना पुस्तकगणना इति आख्या च
आह्रियेताम् ।
```

## ORDER BY (क्रमेण)

**SQL:**
```sql
SELECT name, SUM(total_amount) as revenue
FROM sales_reps 
JOIN sales ON sales_reps.id = sales.sales_rep_id
GROUP BY name
ORDER BY revenue DESC NULLS LAST;
```

**SSQL:**
```ssql
विक्रयाधिकारिभ्यः
    विक्रयणैः सह विक्रयाधिकारिणः सङ्ख्या विक्रयणस्य विक्रयाधिकारिसङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    विक्रयाधिकारिणः नाम इति समूहेन
    विक्रयणस्य सकलमूल्यस्य योगः अवरोहः इति क्रमेण
    विक्रयाधिकारिणः नाम च विक्रयणस्य सकलमूल्यस्य योगः राजस्वम् इति आख्या च
आह्रियेताम् ।
```

## Type Safety

The compiler checks aggregate type compatibility:

- ✅ `SUM(price)` — numeric column, valid
- ❌ `SUM(name)` — text column, compile error (`योगः` and `माध्यम्` are of numbers; `गणना`, `महत्तमम्` and `लघुतमम्` take any column)

This is configured via each column's declared type in the schema DDL:

```ssql
पुस्तकानि इति पटलम् "books" इति आख्या पुस्तक इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
    मूल्यम् इति वास्तविकः "price" इति आख्या मूल्य इति नपुंसकलिङ्गम् एकवचनम् च  // SUM OK
    शीर्षकम् इति वाक्यम् "title" इति आख्या शीर्षक इति नपुंसकलिङ्गम् एकवचनम् च     // SUM ERROR
  इति पटलम् ।
```

## Complete Example

**Calculate commission for sales reps:**

```ssql
विक्रयाधिकारिभ्यः
    विक्रयणैः सह विक्रयाधिकारिणः सङ्ख्या विक्रयणस्य विक्रयाधिकारिसङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    विक्रयाधिकारिणः नाम इति समूहेन
    विक्रयणस्य सकलमूल्यस्य योगः अवरोहः इति क्रमेण
    विक्रयाधिकारिणः नाम च 
    विक्रयणस्य सकलमूल्यस्य योगः सकलराजस्वम् इति आख्या च
    यत् विक्रयणस्य सकलमूल्यस्य योगः इति च ०.१ च इत्यनयोः गुणनम् लाभांशः इति आख्या च
आह्रियन्ताम् ।
```

**Generated SQL:**
```sql
SELECT
    t1.name,
    SUM(t2.total_amount) AS सकलराजस्वम्,
    SUM(t2.total_amount) * 0.1 AS लाभांशः
FROM
    sales_reps t1
JOIN sales t2 ON t1.id = t2.sales_rep_id
GROUP BY t1.name
ORDER BY SUM(t2.total_amount) DESC NULLS LAST;
```

## Next Steps

- [Chapter 6: Window Functions and CTEs](06-window-functions-and-ctes.md)

