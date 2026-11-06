# Project 02: Global Electronics

**Domain:** Retail · **Difficulty:** 🟢 Basic

## Problem Description

A global electronics retailer sells phones, laptops, and tablets to customers
across several countries. Management wants a per-category revenue rollup from
the order history — but only for categories that see meaningful order volume,
so low-traffic categories should be filtered out of the report entirely.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `customers` | ग्राहकाः | `customer_id` (PK), `name`, `gender`, `city`, `state`, `country` |
| `products` | उत्पादाः | `product_id` (PK), `product_name`, `brand`, `category`, `unit_cost_usd`, `unit_price_usd` |
| `orders` | आदेशाः | `order_number` (PK), `order_date`, `customer_id` (FK → customers), `product_id` (FK → products), `quantity` |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground) — "Global Electronics Retailer," a free practice dataset for a fictitious global electronics retailer (multi-table: customer, product, sales-transaction, store, and currency-exchange-rate data, used to explore seasonality, delivery times, and online-vs-in-store order value). This project keeps only three of those tables (customers, products, orders) and uses a small hand-picked fixture — 3 customers, 4 products across 3 categories (Phones, Laptops, Tablets), and 509 orders (505 of them generated, in Phones, so that one category passes the `> 500` threshold) — not the full multi-country, multi-year dataset; stores, exchange rates, and delivery/channel data are dropped entirely. Note that the `> 500` order-count threshold in Objective 1 is scaled for the full dataset, not this 4-order fixture: run against the seed data, the query returns an empty result set, which is expected.

## SQL/SSQL Concepts Demonstrated

- Multi-table `JOIN` (orders ⋈ products)
- `GROUP BY` aggregation with `SUM` over a computed expression (`quantity × unit_price_usd`)
- `HAVING`-equivalent group filtering on an aggregate (`COUNT(...) > 500`), expressed postfix via SSQL's समूह-नियमेन

## Objectives

1. **Objective 1 — Revenue by category, high-volume only**: Calculate total revenue (quantity × unit price, summed) per product category, keeping only categories with more than 500 total orders.

## Deliverables

- Native SSQL query (`queries/q1_revenue_by_category.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_revenue_by_category.सं`): compiles for both SQLite and PostgreSQL (structurally identical SQL; PostgreSQL additionally wraps the `SUM` in `CAST(... AS DOUBLE PRECISION)`). Against the seed data it returns one row, Phones; Laptops and Tablets fall below the `> 500` threshold.
