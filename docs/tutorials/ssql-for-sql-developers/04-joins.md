# Chapter 4: JOINs

## Basic JOIN Syntax

JOINs in SSQL use `सह` (with) and तृतीया विभक्ति (instrumental case).

**SQL:**
```sql
SELECT books.title, authors.name 
FROM books 
JOIN authors ON books.author_id = authors.id;
```

**SSQL:**
```ssql
पुस्तकेभ्यः
    लेखकैः सह पुस्तकस्य लेखकसङ्ख्या लेखकस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    पुस्तकस्य शीर्षकम् च लेखकस्य नाम च 
आह्रियेताम् ।
```

> [!TIP]
> **Table Qualification (`षष्ठी विभक्तिः`, singular):**
> A column is qualified by its table stem in the **genitive singular**: `पुस्तकस्य शीर्षकम्`, "the book's title". Each row has one title, so the owner is one book. The plural `पुस्तकानाम् शीर्षकम्` is rejected.

## JOIN Types

### INNER JOIN (default)

```ssql
पुस्तकेभ्यः
    लेखकैः सह पुस्तकस्य लेखकसङ्ख्या लेखकस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    पुस्तकस्य शीर्षकम् च लेखकस्य नाम च
आह्रियेताम् ।
```

### LEFT JOIN (वाममेलनेन)

```ssql
पुस्तकेभ्यः
    लेखकैः सह पुस्तकस्य लेखकसङ्ख्या लेखकस्य सङ्ख्या इत्यनया समा अस्ति इति वाममेलनेन
    पुस्तकस्य शीर्षकम् च लेखकस्य नाम च
आह्रियेताम् ।
```

**SQL:**
```sql
SELECT books.title, authors.name FROM books LEFT JOIN authors ON books.author_id = authors.id;
```

### Self-Joins with CTEs (`आख्या`)

Because SSQL qualifies columns using the genitive case (षष्ठी विभक्तिः) of the table stem rather than dot-notation, self-joins define a named CTE (`आख्या`) for the second role (e.g., `वरिष्ठ`, "senior", for the manager):

**SQL:**
```sql
WITH managers AS (SELECT id, name FROM sales_reps)
SELECT r.name, m.name 
FROM sales_reps r 
JOIN managers m ON r.manager_id = m.id;
```

**SSQL:**
```ssql
वरिष्ठः इति आख्या वरिष्ठ इति पुंलिङ्गम् एकवचनम्
    विक्रयाधिकारिभ्यः सङ्ख्या च नाम च आह्रियेताम् ।

विक्रयाधिकारिभ्यः
    वरिष्ठेन सह विक्रयाधिकारिणः प्रबन्धकसङ्ख्या वरिष्ठस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    विक्रयाधिकारिणः नाम च वरिष्ठस्य नाम च
आह्रियेताम् ।
```

## Multiple JOINs

Chain multiple JOIN clauses:

```ssql
विक्रयणेभ्यः
    ग्राहकैः सह विक्रयणस्य ग्राहकसङ्ख्या ग्राहकस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    आपणैः सह विक्रयणस्य आपणसङ्ख्या आपणस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    ग्राहकस्य नाम च आपणस्य नाम च विक्रयणस्य सकलमूल्यम् च
आह्रियन्ताम् ।
```

**SQL:**
```sql
SELECT c.name, s.name, sales.total_amount
FROM sales
JOIN customers c ON sales.customer_id = c.id
JOIN stores s ON sales.store_id = s.id;
```

## Combining JOIN (`इति मेलनेन`) with WHERE (`इति नियमेन`)

When a query combines a `JOIN` with a `WHERE` filter, place the `JOIN` clause (`... इति मेलनेन`) after the source table (`पञ्चमी`), followed by the `WHERE` clause (`... इति नियमेन`), followed by the projected columns:

```ssql
पुस्तकेभ्यः
    लेखकैः सह पुस्तकस्य लेखकसङ्ख्या लेखकस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    पुस्तकस्य मूल्यम् २५ इत्यस्मात् अधिकम् अस्ति इति नियमेन
    पुस्तकस्य शीर्षकम् च लेखकस्य नाम च पुस्तकस्य मूल्यम् च
आह्रियन्ताम् ।
```

**SQL:**
```sql
SELECT
    t1.title,
    t2.name,
    t1.price
FROM
    books t1
JOIN authors t2 ON t1.author_id = t2.id
WHERE
    t1.price > 25;
```

## Next Steps

- [Chapter 5: Aggregates and GROUP BY](05-aggregates.md)

