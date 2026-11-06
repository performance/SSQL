# SSQL Keywords Reference

## Verbs (क्रियापदानि)

| Sanskrit | Translation | SQL Equivalent |
|----------|-------------|----------------|
| `आह्रियताम्` / `आह्रियेताम्` / `आह्रियन्ताम्` | "let it / the two / they be fetched" | SELECT (agrees with the number of columns) |
| `निवेश्यताम्` / `निवेश्येताम्` / `निवेश्यन्ताम्` | "let it / the two / they be inserted" | INSERT (agrees with the number of columns) |
| `परिवर्त्यताम्` / `परिवर्त्येताम्` / `परिवर्त्यन्ताम्` | "let it / the two / they be changed" | UPDATE (agrees with the number of columns changed) |
| `निष्कास्यताम्` | "let it be removed" | DELETE |
| `रच्यताम्` | "let it be created" | CREATE |
| `लुप्यताम्` | "let it be dropped" | DROP TABLE/INDEX, ALTER TABLE DROP COLUMN |
| `योज्यताम्` | "let it be added" | ALTER TABLE ADD COLUMN |
| `अभिधीयताम्` | "let it be called" | ALTER TABLE RENAME COLUMN/TABLE |
| `विशदीक्रियताम्` | "let it be explained" | EXPLAIN |
| `प्रारभ्यताम्` | "let it be begun" | BEGIN (`व्यवहारः प्रारभ्यताम्`) |
| `स्थिरीक्रियताम्` | "let it be made firm" | COMMIT (`व्यवहारः स्थिरीक्रियताम्`) |
| `प्रत्याह्रियताम्` | "let it be withdrawn" | ROLLBACK (`व्यवहारः प्रत्याह्रियताम्`) |
| `स्थाप्यताम्` | "let it be set" | SAVEPOINT (`"<name>" इति सङ्ग्रहबिन्दुः स्थाप्यताम्`) |

*(GRANT/REVOKE keyword tokens exist in the lexer but have no working parser support yet - not usable today; see the roadmap.)*

## Comparison Operators

| Sanskrit | Meaning | SQL |
|----------|---------|-----|
| `इत्यनेन समम् अस्ति` | "is equal to this" | = |
| `इत्यनेन समम् नास्ति` | "is not equal to this" | <> |
| `इत्यस्मात् अधिकम् अस्ति` | "is greater than this" | > |
| `इत्यस्मात् न्यूनम् अस्ति` | "is less than this" | < |
| `इत्यस्मात् न्यूनम् नास्ति` | "is not less than this" | >= |
| `इत्यस्मात् अधिकम् नास्ति` | "is not greater than this" | <= |
| `इत्यनेन सदृशम् अस्ति` / `नास्ति` | "is / is not similar to this" | LIKE / NOT LIKE |
| `… च … च इत्येषु अस्ति` / `नास्ति` | "is / is not among these" | IN / NOT IN |
| `… च … च इत्यनयोः मध्ये अस्ति` / `नास्ति` | "is / is not between these two" | BETWEEN / NOT BETWEEN (both bounds included) |
| `रिक्तम् अस्ति` | "is empty" | IS NULL |
| `रिक्तम् नास्ति` | "is not empty" | IS NOT NULL |

The forms above are neuter. The adjective (`समः` / `समा` / `समम्`, `अधिकः` /
`अधिका` / `अधिकम्`, `सदृशः` / `सदृशी` / `सदृशम्`, `रिक्तः` / `रिक्ता` / `रिक्तम्`)
and the pronoun agree with the column: `मूल्यम् २० इत्यस्मात् अधिकम् अस्ति`,
but `सङ्ख्या ५ इत्यस्याः अधिका अस्ति` and `भाषा "Sanskrit" इत्यनया समा अस्ति`.
A feminine IN list takes `इत्यासु`; a list of two values takes `इत्यनयोः`.

## Logical Operators

| Sanskrit | Meaning | SQL |
|----------|---------|-----|
| `च` | and | AND |
| `वा` | or | OR |

There is no separate NOT: a condition is negated with `नास्ति` (see the
comparison operators), and NOT EXISTS is `यत् … इति न विद्यते`.

`च` and `वा` are enclitic: every item of a list takes its own particle, the
last one too, and a single item takes none. This holds for every kind of list:
select items, GROUP BY and ORDER BY items, IN values, columns, inserted values
and conditions.

```ssql
पुस्तकेभ्यः मूल्यम् ५० इत्यस्मात् अधिकम् अस्ति च भाषा "Sanskrit" इत्यनया समा अस्ति च इति नियमेन
    शीर्षकम् च मूल्यम् च आह्रियेताम् ।
```

`शीर्षकम् च मूल्यम्`, with the last item bare, is an error. One list never
mixes `च` with `वा`; group the part you mean with `यत् … इति`. Such a group
is needed only there, as an item of a list of the other particle
(`यत् A वा B वा इति च C च`); a group around a whole list, or around a
`च`-list inside another `च`-list, is an error.

## Aggregate Functions

| Sanskrit | Meaning | SQL |
|----------|---------|-----|
| `योगः` | sum | SUM() |
| `माध्यम्` | average | AVG() |
| `गणना` | count | COUNT() |
| `पङ्क्तीनाम् गणना` | count of the rows | COUNT(*) |
| `महत्तमम्` | maximum | MAX() |
| `लघुतमम्` | minimum | MIN() |

An aggregate takes its argument in the genitive: `मूल्यस्य योगः`, `सङ्ख्यायाः गणना`. A declared column is always declined; the `इति`-pronoun (`… इत्यस्य योगः`) is for a name with no declared stem, such as an alias or a CTE's computed column.

## Arithmetic

| SQL | SSQL | Cases |
|-----|------|-------|
| `a + b` | `<a> च <b> च इत्यनयोः योगः` | operands nominative; `इत्येषाम्` for three or more |
| `a * b` | `<a> च <b> च इत्यनयोः गुणनम्` | as for `योगः` |
| `a - b` | `<a> <b> विशोध्य अवशिष्टम्` | `<a>` ablative (minuend), `<b>` accusative (subtrahend) |
| `a / b` | `<a> <b> विभज्य लब्धम्` | `<a>` accusative (dividend), `<b>` instrumental (divisor) |

`मूल्यतः १० विशोध्य अवशिष्टम्` is `price - 10`; `मूल्यम् २ विभज्य लब्धम्` is
`price / 2`.

A sum, product, remainder or quotient is named by its result noun, which
takes the case of the slot it fills, so a compound operand needs no group:

| Result | ablative (minuend) | accusative (subtrahend, dividend) | instrumental (divisor) | genitive (of an aggregate or function) |
|--------|----|----|----|----|
| `योगः` | `योगात्` | `योगम्` | `योगेन` | `योगस्य` |
| `गुणनम्` | `गुणनात्` | `गुणनम्` | `गुणनेन` | `गुणनस्य` |
| `अवशिष्टम्` | `अवशिष्टात्` (a chain) | `अवशिष्टम्` | `अवशिष्टेन` | `अवशिष्टस्य` |
| `लब्धम्` | `लब्धात्` | `लब्धम्` | `लब्धेन` | `लब्धस्य` |

An aggregate is an operand the same way, its argument in the genitive and its
noun in the case of the slot, with or without a window:

| Aggregate | ablative | accusative | instrumental |
|-----------|----|----|----|
| `योगः` | `योगात्` | `योगम्` | `योगेन` |
| `गणना` | `गणनायाः` | `गणनाम्` | `गणनया` |
| `माध्यम्` | `माध्यात्` | `माध्यम्` | `माध्येन` |
| `महत्तमम्` | `महत्तमात्` | `महत्तमम्` | `महत्तमेन` |
| `लघुतमम्` | `लघुतमात्` | `लघुतमम्` | `लघुतमेन` |

Outside the a-stems the ablative singular is spelled like the genitive, so a
feminine or consonant-stem column is a minuend as it stands:
`सङ्ख्यायाः ५ विशोध्य अवशिष्टम्`, `नाम्नः १ विशोध्य अवशिष्टम्`. `-तः` is accepted too.

- `(price + 2) - 1`: `मूल्यम् च २ च इत्यनयोः योगात् १ विशोध्य अवशिष्टम्`
- `price - (price + 2)`: `मूल्यात् मूल्यम् च २ च इत्यनयोः योगम् विशोध्य अवशिष्टम्`
- `(price - 5) / 2`: `मूल्यात् ५ विशोध्य अवशिष्टम् २ विभज्य लब्धम्`
- `price / (year + 1)`: `मूल्यम् प्रकाशनवर्षम् च १ च इत्यनयोः योगेन विभज्य लब्धम्`
- `price / (year / 2)`: `मूल्यम् प्रकाशनवर्षम् २ विभज्य लब्धेन विभज्य लब्धम्`
- `SUM(price - 1)`: `मूल्यात् १ विशोध्य अवशिष्टस्य योगः`
- `AVG(price + 2)`: `मूल्यम् च २ च इत्यनयोः योगस्य माध्यम्`
- `MAX(price) - MIN(price)`: `मूल्यस्य महत्तमात् मूल्यस्य लघुतमम् विशोध्य अवशिष्टम्`
- `SUM(price) / COUNT(*)`: `मूल्यस्य योगम् पङ्क्तीनाम् गणनया विभज्य लब्धम्`
- `price - AVG(price) OVER (PARTITION BY language)`: `मूल्यात् मूल्यस्य माध्यम् यत्र भाषायाः आधारेण विभाजनम् विशोध्य अवशिष्टम्`

Several subtrahends are one च-list taken away together, and divisors follow
one another:

- `price - year - 1`: `मूल्यात् प्रकाशनवर्षम् च १ च विशोध्य अवशिष्टम्`
- `price / 2 / 3`: `मूल्यम् २ विभज्य लब्धम् ३ विभज्य लब्धम्`

- `price / 2 - 5`: `मूल्यम् २ विभज्य लब्धात् ५ विशोध्य अवशिष्टम्`

A remainder as a subtrahend (`a - (b - c)`) would put two ablatives in a
row, so it is grouped, and the group takes the accusative pronoun `इतीदम्`:
`price - (year - 1)` is `मूल्यात् यत् प्रकाशनवर्षात् १ विशोध्य अवशिष्टम् इति इतीदम्
विशोध्य अवशिष्टम्`. The same goes for a subtrahend with no accusative of its own,
such as a function or a subquery. Around anything that has one, the group is
redundant and is an error.

A function's further operand is in the instrumental: `मूल्यस्य २ इत्यनेन शेषः`,
`मूल्यस्य सङ्ख्यया शेषः`. The pronoun agrees with the first operand, so a feminine
column takes `इत्यनया`: `सङ्ख्यायाः ३ इत्यनया शेषः`. What `प्रतिस्थापनम्` replaces is in
the genitive: `शीर्षकस्य "a" इत्यस्य "o" इत्यनेन प्रतिस्थापनम्`.

A negative constant carries the sign against its digits: `मूल्यम् -५ इत्यस्मात् अधिकम् अस्ति`,
`मात्रा -२ इति परिवर्त्यताम्`, `मूल्यात् -५ विशोध्य अवशिष्टम्`. The sign is for constants
only; `-` is not subtraction.

A negative column follows the adjective `ऋणात्मक`, which agrees with it in gender and
case: `ऋणात्मकम् मूल्यम् आह्रियताम्`, `ऋणात्मकात् मूल्यात् ५ विशोध्य अवशिष्टम्`,
`ऋणात्मिका मात्रा`. The feminine stem is `ऋणात्मिका`. A longer value is named first (a
CTE), and the name is made negative.

## Clause Markers

| Sanskrit | Meaning | SQL Clause |
|----------|---------|------------|
| `नियमेन` | "by rule" | WHERE |
| `समूहेन` | "by group" | GROUP BY |
| `क्रमेण` | "by order" | ORDER BY |
| `समूहनियमेन` | "by the group rule" | HAVING |
| `मेलनेन` | "by joining" | JOIN ON |
| `मित्या` | "by limit" | LIMIT |
| `अपसारेण` | "by setting aside" | OFFSET, after a LIMIT (`५ इति मित्या २ अपसारेण`) |
| `पुनरुक्तिम् विना` | "without repetition" | SELECT DISTINCT (`पुनरुक्तिम् विना भाषा आह्रियताम्`); inside an aggregate, COUNT(DISTINCT x) is `विशिष्टायाः भाषायाः गणना` |

All of these come before the select list, which stands next to its verb:
`पुस्तकेभ्यः मूल्यम् अवरोहः इति क्रमेण ३ इति मित्या शीर्षकम् आह्रियताम् ।`

## Join Types

| Sanskrit | Meaning | SQL |
|----------|---------|-----|
| `<table> सह … इति मेलनेन` | joined with | JOIN |
| `इति वाममेलनेन` | left join | LEFT JOIN |
| `इति दक्षिणमेलनेन` | right join | RIGHT JOIN |
| `इति पूर्णमेलनेन` | full join | FULL OUTER JOIN |
| `<table> सह सर्वमेलनेन` | cross join ("all-joining", no condition) | CROSS JOIN |

## Window Functions

| Sanskrit | Meaning | SQL |
|----------|---------|-----|
| `श्रेणी` | rank | RANK() |
| `यत्र …` | "where …" | OVER (...) |
| `<column genitive> आधारेण विभाजनम्` | partition | PARTITION BY |
| `<column genitive> अनुसारम् आरोहक्रमेण` / `अवरोहक्रमेण` | in ascending / descending order | ORDER BY inside OVER |
| `पूर्वपङ्क्तेः <column>` / `अनन्तरपङ्क्तेः <column>` | the previous / next row's value | LAG / LEAD |
| `द्वितीयपूर्वपङ्क्तेः <column>` / `द्वितीयानन्तरपङ्क्तेः <column>` | the value two rows before / after (`तृतीय-` … `दशम-` for 3 to 10) | LAG(x, 2) / LEAD(x, 2) |

## Special Values

| Sanskrit | Meaning | SQL |
|----------|---------|-----|
| `रिक्तः` / `रिक्ता` / `रिक्तम्` | empty (agrees with the column) | NULL, in the check `रिक्तम् अस्ति` / `रिक्तम् नास्ति` |

The same adjective empties a column in an UPDATE: `पुस्तकेषु सर्वत्र विधा रिक्ता इति परिवर्त्यताम्` is `UPDATE books SET genre = NULL`.

## Connectors

| Sanskrit | Vibhakti | Usage |
|----------|----------|-------|
| `इत्यस्य` | षष्ठी | Possessive (of this...) |
| `इत्यनेन` | तृतीया | Instrumental (with this...) |
| `इत्यस्मात्` | पञ्चमी | Ablative (than this...) |
| `इत्यनयोः` | षष्ठी / सप्तमी द्विवचनम् | Of / among these two (BETWEEN, two-value IN, two-branch set operations) |
| `इत्येषु` | सप्तमी बहुवचनम् | Among these (IN) |
| `तस्मात्` | पञ्चमी | Ablative, anaphoric (from that [just-named]...) - ALTER TABLE RENAME |
| `इति` | - | Quotation marker: names a thing (`… इति पटलम्`), closes a group (`यत् … इति`), and with `आख्या` gives an alias (`<expr> <alias> इति आख्या`) |

The pronouns are the masculine/neuter forms of इदम्; with a feminine column
they are `इत्यस्याः` (genitive and ablative), `इत्यनया` and `इत्यासु`.
