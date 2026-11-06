# Project 01: Maven Toys Sales — Plain-SQL Solution Guide

This is how a SQL expert would solve both objectives directly, without SSQL. Both dialects are shown; where they differ, the difference is called out explicitly.

## Objective 1 — Product catalog

Retrieve every product's name, cost, and price. A plain single-table SELECT — identical in both dialects.

```sql
-- SQLite and PostgreSQL (identical)
SELECT
    product_name,
    product_cost,
    product_price
FROM
    products;
```

No dialect differences: no joins, no aggregates, no type coercion at play.

## Objective 2 — Revenue per store

Total revenue (units × price, summed) per store. Requires joining `sales` to both `stores` and `products`, grouping by store, and summing a computed expression.

```sql
-- SQLite and PostgreSQL (identical)
SELECT
    st.store_name,
    SUM(sa.units * p.product_price) AS revenue
FROM
    sales sa
    JOIN stores st ON sa.store_id = st.store_id
    JOIN products p ON sa.product_id = p.product_id
GROUP BY
    st.store_name;
```

**Dialect notes:**
- The hand-written query above needs no dialect-specific syntax — standard `JOIN`/`GROUP BY`/`SUM` is portable between SQLite and Postgres. The SSQL compiler's PostgreSQL output differs in one place: it wraps the `SUM` in `CAST(... AS DOUBLE PRECISION)`.
- **NULL handling**: if `units` or `product_price` could be `NULL` for some row, `units * product_price` is `NULL` for that row, and `SUM` silently skips `NULL` values in both engines — behavior is consistent, but worth confirming the schema's `NOT NULL` constraints actually hold if exact-match equivalence with SSQL's output matters. Per `schema.सं`, `units`, `store_id`, and `product_id` are declared mandatory (`अनिवार्या`/`अनिवार्यः`), so this shouldn't arise in practice for this dataset.
- **Integer vs. float multiplication**: `units` is an integer (`पूर्णाङ्कः`) and `product_price` is a real/float (`वास्तविकः`), so `units * product_price` promotes to a float in both SQLite and Postgres — no truncation risk.
- **Grouping by name vs. id**: this groups by `store_name` rather than `store_id`. That's only safe if store names are unique (which the schema doesn't enforce structurally) — worth flagging as a modeling assumption inherited directly from the SSQL query, not something introduced by this translation.

## SSQL status

- Objective 1's SSQL (`queries/q1_products.सं`) compiles to output that matches the SQLite query above exactly, and the Postgres query above exactly (both generated outputs are byte-identical to each other and to hand-verification).
- Objective 2's SSQL (`queries/q2_revenue_per_store.सं`) compiles for both dialects and returns the same three rows as the query above (Downtown Toys 459.81, Metro Playroom 119.85, Suburbia Play 169.95). Its output lists the revenue column (aliased `राजस्वम्`) first and the store name second, the order the SSQL query names them; on PostgreSQL the `SUM` is wrapped in `CAST(... AS DOUBLE PRECISION)`.
