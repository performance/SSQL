# Project 01: Maven Toys Sales — SSQL Solution Guide

Detailed grammatical walkthrough of how each objective was expressed in native, spoken Sanskrit — not just "what parses," but *why* each vibhakti/word choice is the natural one for its role.

## Schema recap

Three tables, defined natively in `schema.सं` using native SSQL DDL:

- `क्रीडनकभाण्डाराः` ("toy-stores", masculine plural) → `stores`
- `उत्पादाः` ("products", masculine plural) → `products`
- `विक्रयाः` ("sales", masculine plural) → `sales`

## Objective 1 — Product catalog

**Goal**: select `name`, `cost`, `price` from products.

**Sanskritization**:
- The source table is `उत्पादाः` (masculine plural, nominative). In SSQL, a FROM-source must be in the **Ablative case** (पञ्चमी विभक्ति — "out of/from the products"), which declines `उत्पादाः` to `उत्पादेभ्यः`.
- The three selected fields — `नाम` (name), `व्ययः` (cost), `मूल्यम्` (price) — are each nominative-case (प्रथमा) column references, joined by the connective particle `च` ("and").
- Because exactly **three** columns are selected, Sanskrit's grammatical number agreement requires the **plural** fetch verb `आह्रियन्ताम्` ("let them be fetched") rather than singular or dual — the SSQL validator enforces this (`src/validator.rs`), matching real Paninian subject-verb number agreement (एकवचन/द्विवचन/बहुवचन).

**Query**:
```samskrutam
उत्पादेभ्यः नाम च व्ययः च मूल्यम् च आह्रियन्ताम् ।
```

**Reading it aloud**: "From-the-products, name and cost and price, let-them-be-fetched."

## Objective 2 — Revenue per store

**Goal**: sum of (units × price) per store, i.e. total revenue grouped by store.

**Sanskritization**:
- The source is `विक्रयाः` (sales), ablative `विक्रयेभ्यः`.
- Two joins are needed, each using the **Instrumental case** (तृतीया विभक्ति, "by means of/together with") marked by the postposition `सह` ("with") and closed by the particle `मेलनेन` ("by joining"):
  - `क्रीडनकभाण्डारैः सह विक्रयस्य भाण्डारसङ्ख्या क्रीडनकभाण्डारस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन` — join with stores.
  - `उत्पादैः सह विक्रयस्य उत्पादसङ्ख्या उत्पादस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन` — join with products.
  - Each join's condition compares two genitive (षष्ठी, property-access) field references with the equality construction `इत्यनया समा अस्ति` ("is equal to this") — e.g. `विक्रयस्य भाण्डारसङ्ख्या क्रीडनकभाण्डारस्य सङ्ख्या इत्यनया समा अस्ति` ("this sale's store-number is equal to this, the store's number"). Note the genitive is **singular** (`विक्रयस्य`, not `विक्रयाणाम्`) since this refers to one row's property, not the whole `विक्रयाः` collection.
- The **GROUP BY** target uses the same Instrumental-marking pattern with `समूहेन` ("by grouping"): `क्रीडनकभाण्डारस्य नाम इति समूहेन` — "by the store's name, grouped."
- **Math is postfix** in SSQL: `units * price` is expressed as `विक्रयस्य मात्रा च उत्पादस्य मूल्यम् च इत्यनयोः गुणनम्` — literally "sale's quantity and product's price, of-these-two, the multiplication." This is wrapped in the aggregate suffix pattern `इत्यस्य योगः` — "of this, the sum" — to produce `SUM(units * price)`, then aliased `इति राजस्वम्` ("called revenue").
- Because exactly **two** columns are ultimately selected (revenue, store name), the **dual** fetch verb `आह्रियेताम्` is required — distinct from objective 1's plural form.

**Query**:
```samskrutam
विक्रयेभ्यः
    क्रीडनकभाण्डारैः सह विक्रयस्य भाण्डारसङ्ख्या क्रीडनकभाण्डारस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    उत्पादैः सह विक्रयस्य उत्पादसङ्ख्या उत्पादस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
    क्रीडनकभाण्डारस्य नाम इति समूहेन
    विक्रयस्य मात्रा च उत्पादस्य मूल्यम् च इत्यनयोः गुणनम् इत्यस्य योगः राजस्वम् इति आख्या च
    क्रीडनकभाण्डारस्य नाम च
    आह्रियेताम् ।
```

**Compiler status**: both queries compile and execute in 100% strict mode without parentheses for both SQLite and PostgreSQL.
