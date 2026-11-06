# Project 18: Patient Readmission Analysis (Date Math & LAG)

This project tracks patient hospital readmissions by computing the gap between current admission and previous discharge using `LAG`, with the difference itself expressed via the native subtraction pattern (`विशोध्य`/`अवशिष्टम्`).

## DDL Schema (`schema.सं`)
```samskrutam
प्रवेशाः इति पटलम् "admissions" इति आख्या प्रवेश इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "admission_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    रोगी इति पूर्णाङ्कः "patient_id" इति आख्या रोगिन् इति पुंलिङ्गम् एकवचनम् च
    प्रवेशदिनम् इति पूर्णाङ्कः "admission_date" इति आख्या प्रवेशदिन इति नपुंसकलिङ्गम् एकवचनम् च
    निर्गमदिनम् इति पूर्णाङ्कः "discharge_date" इति आख्या निर्गमदिन इति नपुंसकलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
प्रवेशेभ्यः रोगी च प्रवेशदिनम् च निर्गमदिनम् च प्रवेशदिनात् पूर्वपङ्क्तेः निर्गमदिनम् यत्र रोगिणः आधारेण विभाजनम् प्रवेशदिनस्य अनुसारम् आरोहक्रमेण विशोध्य अवशिष्टम् "days_since_discharge" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.patient_id,
    t1.admission_date,
    t1.discharge_date,
    t1.admission_date - LAG(t1.discharge_date) OVER (PARTITION BY t1.patient_id ORDER BY t1.admission_date ASC NULLS LAST) AS days_since_discharge
FROM
    admissions t1;
```
