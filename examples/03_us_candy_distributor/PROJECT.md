# Project 03: US Candy Distributor

**Domain:** Supply Chain · **Difficulty:** 🟡 Intermediate

## Problem Description

A US candy distributor manufactures its products across several factories and
ships orders to customers from whichever factory fulfilled them. An operations
analyst wants a single flattened view of every order — which product it was,
which factory shipped it, how many units, and what revenue it brought in — by
joining the order records against both the product catalog and the factory
directory.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `products` | उत्पादाः | `product_id` (PK), `product_name`, `division`, `price` |
| `factories` | उद्योगशालाः | `factory_id` (PK), `factory_name`, `state` |
| `orders` | आदेशाः | `order_id` (PK), `customer_id`, `product_id` (FK → products), `factory_id` (FK → factories), `units`, `gross_revenue` |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground) — "US Candy Distributor," a free practice dataset for a US candy distributor (499 records across 25 fields, covering sales and geospatial factory-to-customer shipment data — customer/factory locations, sales orders and goals, and product details — used to explore shipping-route efficiency and product margins). This project keeps three tables (products, factories, orders) and a small hand-picked fixture — 3 products across 2 divisions (Sugar, Chocolate), 2 factories, and 3 orders — not the full 499-record dataset; customer records, geolocation, and sales-goal data are dropped entirely, and `orders.customer_id` here is a bare integer with no `customers` table behind it (only `product_id` and `factory_id` are real foreign keys in this fixture).

## SQL/SSQL Concepts Demonstrated

- Multi-table `JOIN` chaining three tables in a single query (orders ⋈ products ⋈ factories)
- Plain column projection across joined tables with no aggregation

## Objectives

1. **Objective 1 — Order detail report**: For every order, return the order ID, product name, factory name, units, and gross revenue by joining `orders`, `products`, and `factories`.

## Deliverables

- Native SSQL query (`queries/q1_order_details.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_order_details.सं`): compiles to the same SQL for SQLite and PostgreSQL. Against the seed data it returns all 3 orders with product and factory names joined in (e.g. order 1 → "Sour Gummy Worms" / "SweetWorks NY" / 150 units / 448.5 revenue).
