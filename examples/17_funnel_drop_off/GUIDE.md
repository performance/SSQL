# Project 17: Funnel Drop-off Analysis (LAG Window Function)

This project calculates the time difference (in seconds) between user clicks within the same session using the SSQL `LAG` window function, with the difference itself expressed via the native subtraction pattern (`विशोध्य`/`अवशिष्टम्`).

## DDL Schema (`schema.सं`)
```samskrutam
सम्पर्काः इति पटलम् "clicks" इति आख्या सम्पर्क इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "click_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    सङ्केतः इति वाक्यम् "session_id" इति आख्या सङ्केत इति पुंलिङ्गम् एकवचनम् च
    सम्पर्कसमयः इति पूर्णाङ्कः "click_time" इति आख्या सम्पर्कसमय इति पुंलिङ्गम् एकवचनम् च
    पुटम् इति वाक्यम् "page_name" इति आख्या पुट इति नपुंसकलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
सम्पर्केभ्यः सङ्केतः च पुटम् च सम्पर्कसमयः च सम्पर्कसमयात् पूर्वपङ्क्तेः सम्पर्कसमयम् यत्र सङ्केतस्य आधारेण विभाजनम् सम्पर्कसमयस्य अनुसारम् आरोहक्रमेण विशोध्य अवशिष्टम् "time_diff" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.session_id,
    t1.page_name,
    t1.click_time,
    t1.click_time - LAG(t1.click_time) OVER (PARTITION BY t1.session_id ORDER BY t1.click_time ASC NULLS LAST) AS time_diff
FROM
    clicks t1;
```
