# Project 08: Ingredient Costs

**Domain:** Food · **Difficulty:** 🔴 Advanced

## Problem Description

A pizzeria owner wants to know how profitable each pizza type actually is
once ingredient cost is accounted for, not just its menu price. Given a
catalog of pizza types (with their menu price) and a separate record of
each type's ingredient cost, the owner wants a per-pizza-type profit figure
(price minus ingredient cost).

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `pizza_types` | पिष्टकप्रकाराः | `pizza_type_id` (PK), `name`, `price` |
| `ingredients` | घटकाः | `ingredient_cost_id` (PK), `pizza_type_id` (FK → pizza_types), `ingredient_cost` |

**Source:** [Maven Analytics Data Playground](https://mavenanalytics.io/data-playground)
— the "Maven Pizza Challenge." The `pizza_types` table's identity
(`pizza_type_id`, `name`, `price`) is kept from the real dataset, but the
real dataset does *not* contain a per-type ingredient-cost table — its
actual `pizza_types` table instead carries a free-text `ingredients`
column (a comma-separated list of ingredient names, no cost breakdown).
The `ingredients`/`घटकाः` table here, with a numeric `ingredient_cost` per
pizza type, is a hand-built addition purpose-built to give this project a
second table to `JOIN` against and a profit figure to compute via
subtraction — it is not lifted from the real dataset's schema. It's a
small 3+3-row fixture (3 pizza types, one ingredient-cost row each).

## SQL/SSQL Concepts Demonstrated

- Multi-table `JOIN` (instrumental-case "with," `मेलनेन`)
- Native ablative-case subtraction (Shashthi-qualified minuend in the
  ablative + Shashthi-qualified subtrahend + `विशोध्य`/`अवशिष्टम्`),
  computing `price - ingredient_cost` across the two joined tables
- Selecting columns from both sides of a join in one projection list

## Objectives

1. **Objective 1 — Profit per pizza type**: Join `pizza_types` to
   `ingredients` on `pizza_type_id` and return each pizza's name alongside
   its profit (`price - ingredient_cost`).

## Deliverables

- Native SSQL query (`queries/q1_profit.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_profit.सं`): compiles to the same SQL for SQLite and
  PostgreSQL and returns 3 rows against the seed data: Margherita 7.49,
  Pepperoni 8.79, Veggie Supreme 7.69.
