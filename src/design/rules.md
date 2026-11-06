# Comparison & Connector Token Design

This note documents a specific design decision in the lexer/parser
(`src/ssql_parser/token.rs`, `src/ssql_parser/expr.rs`): how gendered
Sanskrit comparison adjectives and vibhakti connectors are tokenized, and
why grammatical agreement is checked in a later pass rather than in the
grammar itself.

## Gender-grouped comparison adjectives

Sanskrit comparison adjectives decline by the gender of the operand they
qualify (masculine/feminine/neuter), e.g. "equal" is `समः` (masc.), `समा`
(fem.), or `समम्` (neut.). The lexer collapses each adjective's gendered
surface forms into a single `Token` variant that retains the original text:

| Concept | Surface forms | Token |
|---|---|---|
| Equal (with `नास्ति`: not equal) | `समः` / `समा` / `समम्` | `Token::Samam(String)` |
| Greater | `अधिकः` / `अधिका` / `अधिकम्` | `Token::Adhikam(String)` |
| Less | `न्यूनः` / `न्यूना` / `न्यूनम्` | `Token::Nyunam(String)` |
| Empty/Null | `रिक्तः` / `रिक्ता` / `रिक्तम्` | `Token::Riktah(String)` |
| Similar (LIKE) | `सदृशः` / `सदृशी` / `सदृशम्` | `Token::Sadrusham` |

Grouping by gender rather than emitting a distinct token per surface form
keeps the comparison grammar in `expr.rs` simple: a single `Token::Samam`
arm matches an equality comparison regardless of which gendered form was
used. The parser does not itself check that the adjective's gender
actually agrees with the operand's declared gender - that check belongs to
semantic analysis (`src/validator.rs`), which has the schema's gender
declarations in scope and can produce a proper `Grammar Error` diagnostic
rather than a parse failure. Keeping this concordance check out of the
grammar also means a genuinely malformed-gender query still parses (so the
compiler can report a precise grammatical error) instead of failing with a
generic syntax error at the token level.

## Vibhakti connectors

Comparisons and similar binary constructs are introduced by a vibhakti
connector matching the grammatical relationship being expressed, not a
symbolic operator:

- `इत्यनेन` (instrumental, "with this") - equality/similarity: `X इत्यनेन समम् अस्ति`.
- `इत्यस्मात्` (ablative, "than this") - difference/magnitude: `X इत्यस्मात् अधिकम् अस्ति`.

Existence/null checks use the copula directly rather than a connector:
`अस्ति`/`नास्ति` (is/is not), `विद्यते` (exists), paired with `रिक्तः`/`रिक्ता`/`रिक्तम्`
for NULL checks.

## CASE expressions

Conditional expressions use `वरणम्` ("choice") rather than a word implying
a single yes/no decision, since a `CASE` expression can have arbitrarily
many branches (`यस्मिन् ... तस्मिन् ...` per branch, `अन्यस्मिन्` for the
default).
