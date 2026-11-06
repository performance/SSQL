# Project 25: Genomic Variant Frequencies (CASE Conditional Classification)

This project classifies human genomic variants into rarity tiers (`Rare` vs `Common`) based on allele frequency using SSQL `CASE` conditional expressions.

## DDL Schema (`schema.सं`)
```samskrutam
जनुकविकल्पाः इति पटलम् "genomic_variants" इति आख्या जनुकविकल्प इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    वंशाणुसङ्केतः इति वाक्यम् "gene_symbol" इति आख्या वंशाणुसङ्केत इति पुंलिङ्गम् एकवचनम् च
    विकल्पसङ्ख्या इति वाक्यम् "variant_id" इति आख्या विकल्पसङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    युग्मविकल्पावृत्तिः इति वास्तविकः "allele_freq" इति आख्या युग्मविकल्पावृत्ति इति स्त्रीलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
जनुकविकल्पेभ्यः वंशाणुसङ्केतः च विकल्पसङ्ख्या च युग्मविकल्पावृत्तिः च वरणम् यस्मिन् युग्मविकल्पावृत्तिः ०.०१ इत्यस्याः न्यूना अस्ति तस्मिन् "Rare" अन्यस्मिन् "Common" "frequency_class" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.gene_symbol,
    t1.variant_id,
    t1.allele_freq,
    CASE WHEN t1.allele_freq < 0.01 THEN 'Rare' ELSE 'Common' END AS frequency_class
FROM
    genomic_variants t1;
```
