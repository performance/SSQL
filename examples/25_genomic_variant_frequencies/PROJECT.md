# Project 25: Genomic Variant Frequencies

**Domain:** Genomics / Bioinformatics · **Difficulty:** 🔴 Advanced

## Problem Description

A genomics analyst is reviewing a set of genetic variants and wants each
one automatically classified as "rare" or "common" based on how frequently
its alternate allele shows up in the population. Rather than eyeballing
each frequency value, the analyst wants a query that labels every variant
with a `Rare`/`Common` tag in one pass.

## Dataset & Provenance

| Table | Sanskrit name | Columns |
|---|---|---|
| `genomic_variants` | जनुकविकल्पाः | `id` (PK), `gene_symbol`, `variant_id`, `allele_freq` (real) |

**Source:** No external public dataset. The scenario was invented for this example; it's an invented 4-column, 3-row
fixture, not an extract of gnomAD, ClinVar, or dbSNP. The three rows use real identifiers: `rs80357906`
(BRCA1), `rs28934578` (TP53), and `rs429358` (the well-known APOE ε4
variant) are genuine dbSNP reference SNP IDs paired with their correct
genes, and the query's `0.01` (1%) cutoff for "rare" is the actual
industry-standard minor-allele-frequency threshold used in population
genetics to distinguish rare from common variants. The *IDs and threshold* are real; the *table, rows, and frequency
values* are still a hand-built fixture purpose-built to demonstrate one
SQL feature: `CASE`-expression classification.

## SQL/SSQL Concepts Demonstrated

- `CASE WHEN ... THEN ... ELSE ... END` conditional classification,
  expressed via SSQL's native `वरणम् यस्मिन्/तस्मिन्/अन्यस्मिन्` construct
- Column aliasing of a computed expression (`"frequency_class" इति आख्या`)

## Objectives

1. **Objective 1 — Frequency classification**: For every genomic variant,
   return the gene symbol, variant ID, allele frequency, and a
   `frequency_class` label of `Rare` (allele frequency below 0.01) or
   `Common` (otherwise).

## Deliverables

- Native SSQL query (`queries/q1_elegant.सं`).
- A step-by-step walkthrough ([`GUIDE.md`](GUIDE.md)).

## Status

- **Objective 1** (`q1_elegant.सं`): compiles to the same SQL for SQLite and PostgreSQL:
  ```sql
  SELECT
      t1.gene_symbol,
      t1.variant_id,
      t1.allele_freq,
      CASE WHEN t1.allele_freq < 0.01 THEN 'Rare' ELSE 'Common' END AS frequency_class
  FROM
      genomic_variants t1;
  ```
  Against the seed data: BRCA1 and TP53 are labelled `Rare`, APOE `Common`.
