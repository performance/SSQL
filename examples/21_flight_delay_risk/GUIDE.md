# Project 21: Flight Delay Risk (Window Functions & Dense Rank)

This project ranks flights by delay duration within each carrier using SSQL Window Ranking.

## DDL Schema (`schema.सं`)
```samskrutam
विमानानि इति पटलम् "flights" इति आख्या विमान इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "flight_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    वाहकः इति वाक्यम् "carrier" इति आख्या वाहक इति पुंलिङ्गम् एकवचनम् च
    विमानसङ्ख्या इति पूर्णाङ्कः "flight_num" इति आख्या विमानसङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    विलम्बकलासङ्ख्या इति पूर्णाङ्कः "delay_minutes" इति आख्या विलम्बकलासङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
विमानेभ्यः वाहकः च विमानसङ्ख्या च विलम्बकलासङ्ख्या च सघनश्रेणी यत्र वाहकस्य आधारेण विभाजनम् विलम्बकलासङ्ख्यायाः अनुसारम् अवरोहक्रमेण "delay_rank" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.carrier,
    t1.flight_num,
    t1.delay_minutes,
    DENSE_RANK() OVER (PARTITION BY t1.carrier ORDER BY t1.delay_minutes DESC NULLS LAST) AS delay_rank
FROM
    flights t1;
```
