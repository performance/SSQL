# Chapter 6: Window Functions and CTEs

## Window Functions (`श्रेणी`)

SSQL expresses SQL `OVER (PARTITION BY ... ORDER BY ...)` with a `यत्र` clause after the window function: `<keys> आधारेण विभाजनम्` (PARTITION BY) and `<keys> अनुसारम् आरोहक्रमेण / अवरोहक्रमेण` (ORDER BY), in either order, then an optional frame.

| SQL Window Function | SSQL Form | Meaning |
|---------------------|-----------|---------|
| `RANK() OVER (...)` | `श्रेणी यत्र ...` | Rank within the window |
| `DENSE_RANK() OVER (...)` | `सघनश्रेणी यत्र ...` | Rank without gaps |
| `ROW_NUMBER() OVER (...)` | `पङ्क्तिसङ्ख्या यत्र ...` | Row number |
| `NTILE(4) OVER (...)` | `चतुर्थांशाङ्कः यत्र ...` | Quartile number |
| `NTILE(n) OVER (...)` | `<n> भागेषु भागाङ्कः यत्र ...` (two parts: `२ भागयोः भागाङ्कः`) | Part number among n parts |
| `LAG(col) OVER (...)` | `पूर्वपङ्क्तेः <col> यत्र ...` | The previous row's value |
| `LEAD(col) OVER (...)` | `अनन्तरपङ्क्तेः <col> यत्र ...` | The next row's value |
| `AVG(col) OVER (...)` | `<col-genitive> माध्यम् यत्र ...` | Windowed average (likewise `योगः`, `गणना`, `महत्तमम्`, `लघुतमम्`) |

> [!TIP]
> **Keys are in the genitive.** `आधारेण` ("on the basis of") and `अनुसारम्` ("according to") govern the genitive: `भाषायाः आधारेण विभाजनम्`, `मूल्यस्य अनुसारम् अवरोहक्रमेण`. An aggregate key declines its own noun (`मूल्यस्य योगस्य अनुसारम्`, `… इत्यस्य गणनायाः अनुसारम्`); only a name with no declared stem (an alias produced by an earlier CTE) takes the genitive pronoun: `समीक्षाश्रेणीमाध्यम् इत्यस्य अनुसारम्`.
>
> **Every ordering states its direction** (`आरोहक्रमेण` or `अवरोहक्रमेण`). Each part of the window therefore ends in its own marker word, so the alias follows directly: `श्रेणी यत्र … अवरोहक्रमेण मूल्यश्रेणी इति आख्या`.

### Example: Ranking Books by Price Within Each Language

**SQL:**
```sql
SELECT
    title,
    language,
    price,
    RANK() OVER (PARTITION BY language ORDER BY price DESC NULLS LAST) AS price_rank
FROM books;
```

**SSQL:**
```ssql
पुस्तकेभ्यः
    शीर्षकम् च
    भाषा च
    मूल्यम् च
    श्रेणी यत्र भाषायाः आधारेण विभाजनम् मूल्यस्य अनुसारम् अवरोहक्रमेण मूल्यश्रेणी इति आख्या च
आह्रियन्ताम् ।
```

### Example: Windowed Aggregate (`AVG(...) OVER (PARTITION BY ...)`)

Each book is shown beside the average price of its genre. A window with no ordering covers its whole partition; an ordering would make it a running average, up to the current row.

**SQL:**
```sql
SELECT
    title,
    genre,
    price,
    AVG(price) OVER (PARTITION BY genre) AS विधामाध्यम्
FROM books;
```

**SSQL:**
```ssql
पुस्तकेभ्यः
    शीर्षकम् च
    विधा च
    मूल्यम् च
    मूल्यस्य माध्यम् यत्र विधायाः आधारेण विभाजनम् विधामाध्यम् इति आख्या च
आह्रियन्ताम् ।
```

### Example: Comparing Consecutive Sales Using `पूर्वपङ्क्तेः` (`LAG`)

**SQL:**
```sql
SELECT
    sale_date,
    total_amount,
    LAG(total_amount) OVER (PARTITION BY store_id ORDER BY sale_date ASC) AS prev_amount
FROM sales;
```

**SSQL:**
```ssql
विक्रयणेभ्यः
    विक्रयणदिनाङ्कः च
    सकलमूल्यम् च
    पूर्वपङ्क्तेः सकलमूल्यम् यत्र आपणसङ्ख्यायाः आधारेण विभाजनम् विक्रयणदिनाङ्कस्य अनुसारम् आरोहक्रमेण पूर्वमूल्यम् इति आख्या च
आह्रियन्ताम् ।
```

## Common Table Expressions (`इति आख्या ...`)

Temporary named result sets (`WITH cte AS (...)`) are declared by binding the CTE's nominative name, stem, gender, and number with `इति आख्या <stem> इति <gender> <vachana>` before its query body (either directly or wrapped in `सञ्ज्ञा यत् <query> इति ।`).

When querying from a CTE declared as a singular summary noun (`एकवचनम्`), it is read in the **Ablative Singular (`पञ्चमी एकवचनम्`, `-आत्`)**, agreeing with its declared number:

**SQL:**
```sql
WITH पुस्तकविक्रयणसार AS (
    SELECT
        SUM(quantity) AS सकलविक्रयणम्,
        book_id
    FROM sale_items
    GROUP BY book_id
)
SELECT book_id, सकलविक्रयणम्
FROM पुस्तकविक्रयणसार;
```

**SSQL:**
```ssql
पुस्तकविक्रयणसारः इति आख्या पुस्तकविक्रयणसार इति पुंलिङ्गम् एकवचनम् विक्रयणवस्तुभ्यः
    पुस्तकसङ्ख्या इति समूहेन
    परिमाणस्य योगः सकलविक्रयणम् इति आख्या च
    पुस्तकसङ्ख्या च
आह्रियेताम् ।

पुस्तकविक्रयणसारात्
    पुस्तकसङ्ख्या च
    सकलविक्रयणम् च
आह्रियेताम् ।
```

## Next Steps

- [Chapter 7: Pure User-Defined Functions & Tooling](07-pure-functions-and-transpiler.md)
