# Chapter 2: SELECT Basics

## Selecting Columns

In SQL, you list columns after SELECT. In SSQL, columns appear in **प्रथमा विभक्ति** (nominative case).

**SQL:**
```sql
SELECT title, price FROM books;
```

**SSQL:**
```ssql
पुस्तकेभ्यः शीर्षकम् च मूल्यम् च आह्रियेताम् ।
```

### Multiple Columns with च (and)

Use `च` after each column to list multiple columns, matching the verb number (`आह्रियताम्` for 1 column, `आह्रियेताम्` for 2 columns, `आह्रियन्ताम्` for 3 or more):

```ssql
पुस्तकेभ्यः 
    शीर्षकम् च 
    मूल्यम् च 
    भाषा च 
आह्रियन्ताम् ।
```

Compiles to:
```sql
SELECT title, price, language FROM books;
```

## Column Aliases

Use `इति` to create aliases:

**SQL:**
```sql
SELECT title AS book_title FROM books;
```

**SSQL:**
```ssql
पुस्तकेभ्यः शीर्षकम् पुस्तकशीर्षकम् इति आख्या आह्रियताम् ।
```

The pattern is: `<expression> <alias> इति आख्या`

## Expressions

### Arithmetic

Use Sanskrit mathematical constructs with `च` (using dual `इत्यनयोः` for 2 operands or plural `इत्येषाम्` for 3+ operands), Ablative (`पञ्चमी`, `-आत्`) + `विशोध्य अवशिष्टम्` for subtraction, and Instrumental (`तृतीया`) + `विभज्य लब्धम्` for division:

| Operation | Sanskrit Pattern | Example |
|-----------|------------------|---------|
| Addition (`+`) | `<A> च <B> च इत्यनयोः योगः` | `मूल्यम् च १० च इत्यनयोः योगः` |
| Subtraction (`-`) | `<MinuendAblative> <Subtrahend> विशोध्य अवशिष्टम्` | `मूल्यात् ५ विशोध्य अवशिष्टम्` |
| Multiplication (`*`) | `<A> च <B> च इत्यनयोः गुणनम्` | `मूल्यम् च २ च इत्यनयोः गुणनम्` |
| Division (`/`) | `<Dividend> <DivisorInstrumental> विभज्य लब्धम्` | `सकलमूल्यम् परिमाणेन विभज्य लब्धम्` |

**Example (Multiple Arithmetic Projections):**
```ssql
पुस्तकेभ्यः
    शीर्षकम् च
    मूल्यम् च २ च इत्यनयोः गुणनम् द्विगुणमूल्यम् इति आख्या च
    मूल्यात् ५ विशोध्य अवशिष्टम् न्यूनमूल्यम् इति आख्या च
आह्रियन्ताम् ।
```

Compiles to:
```sql
SELECT
    t1.title,
    t1.price * 2 AS द्विगुणमूल्यम्,
    t1.price - 5 AS न्यूनमूल्यम्
FROM
    books t1;
```

## String Constants

Use quoted strings (Devanagari or Latin):

**SSQL:**
```ssql
पुस्तकेभ्यः 
    भाषा "Sanskrit" इत्यनया समा अस्ति इति नियमेन 
    शीर्षकम् 
आह्रियताम् ।
```

Compiles to:
```sql
SELECT title FROM books WHERE language = 'Sanskrit';
```

## SELECT DISTINCT

Write `पुनरुक्तिम् विना` ("without repetition") just before the select list.
It removes repeated rows, whatever the columns and their genders:

```ssql
पुस्तकेभ्यः
    पुनरुक्तिम् विना भाषा
आह्रियताम् ।
```

Compiles to:
```sql
SELECT DISTINCT language FROM books;
```

## Next Steps

- [Chapter 3: WHERE and Conditions](03-where-conditions.md)

