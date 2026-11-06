# Project 19: HR Org Chart

**Domain:** HR · **Difficulty:** 🔴 Advanced

## Problem Description

An HR analyst has a flat employee table where each row only points at its
own direct manager, and wants the full reporting hierarchy unrolled from
that: starting at whoever has no manager (the top of the chart) and
walking down every reporting chain, produce each employee's id, name, and
how many levels deep they sit.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `employees` | कर्मचारिणः | `employee_id` (PK), `employee_name`, `manager_id` (self-referencing FK → employees, null at the root) |

**Source:** Kaggle's [Human Resources Data Set](https://www.kaggle.com/datasets/rhuebner/human-resources-data-set)
(Rich Huebner) — a widely-used HR analytics teaching dataset covering
~311 employees of a fictitious company across 36 columns, including
`EmpID`/`ManagerName`/`ManagerID` fields that encode exactly this kind of
self-referencing reporting hierarchy. This is the
most prominent public dataset with this shape. This project keeps only the
hierarchy shape (id, name, manager id) and discards everything else
(department, pay rate, performance score, termination reasons, etc.),
building a hand-picked, minimal 3-column, 4-row fixture — a single linear
chain four levels deep (CEO → VP → Manager → Dev) — not the real dataset's
broader, shallower org.

## SQL/SSQL Concepts Demonstrated

- Recursive CTE (`आवर्तिका सञ्ज्ञा`) with a non-recursive base case unioned
  with a recursive term that joins the CTE back to itself
- Explicit cycle prevention (`चक्रनिवारणेन`): both dialects carry a path
  column of ids, each closed by the unit separator, and stop a row whose id
  is already on the path (`instr` on SQLite, `strpos` on PostgreSQL)
- Native list-sum addition (`स्तरः च १ च इत्यनयोः योगः`) for the recursive
  level counter's `+ 1`; the dual `इत्यनयोः` closes the sum, so it needs no
  `यत् … इति` group inside the surrounding `च`-joined select list

## Objectives

1. **Objective 1 — Recursive hierarchy mapping**: Build the full org
   chart, returning every employee's id, name, and depth/level (1 = top,
   incrementing one per reporting level down), starting from whoever has
   no manager.

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles
  to the same path-tracking cycle guard on both dialects (`instr` on
  SQLite, `strpos` on PostgreSQL). Against the seed data it returns
  the 4 employees at levels 1 through 4 (CEO → VP → Manager → Dev).
