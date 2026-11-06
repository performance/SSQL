# Project 23: Athlete Performance Index (Sports Analytics & Multi-level Aggregation)

This project computes average and total scores per athlete and sport using multi-column `GROUP BY` and ordering by aggregate sum.

## DDL Schema (`schema.सं`)
```samskrutam
क्रीडाङ्काः इति पटलम् "athlete_scores" इति आख्या क्रीडाङ्क इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "score_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    क्रीडकनाम इति वाक्यम् "athlete_name" इति आख्या क्रीडकनामन् इति नपुंसकलिङ्गम् एकवचनम् च
    क्रीडा इति वाक्यम् "sport" इति आख्या क्रीडा इति स्त्रीलिङ्गम् एकवचनम् च
    अङ्कः इति वास्तविकः "score" इति आख्या अङ्क इति पुंलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
क्रीडाङ्केभ्यः क्रीडकनाम च क्रीडा च इति समूहेन अङ्कानाम् योगः अवरोहः इति क्रमेण क्रीडकनाम च क्रीडा च अङ्कानाम् माध्यम् "avg_score" इति आख्या च अङ्कानाम् योगः "total_score" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.athlete_name,
    t1.sport,
    AVG(t1.score) AS avg_score,
    SUM(t1.score) AS total_score
FROM
    athlete_scores t1
GROUP BY t1.athlete_name, t1.sport
ORDER BY SUM(t1.score) DESC NULLS LAST;
```
