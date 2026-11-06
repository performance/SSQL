# Project 26: Customer Cohorts (First Purchase Window Function)

This project determines each user's initial purchase date using SSQL Window Minimum.

## DDL Schema (`schema.सं`)
```samskrutam
ग्राहकादेशाः इति पटलम् "user_orders" इति आख्या ग्राहकादेश इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "order_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    ग्राहकः इति पूर्णाङ्कः "user_id" इति आख्या ग्राहक इति पुंलिङ्गम् एकवचनम् च
    उद्घाटनदिनम् इति वाक्यम् "order_date" इति आख्या उद्घाटनदिन इति नपुंसकलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
ग्राहकादेशेभ्यः ग्राहकः च उद्घाटनदिनम् च उद्घाटनदिनानाम् लघुतमम् यत्र ग्राहकस्य आधारेण विभाजनम् "first_order_date" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.user_id,
    t1.order_date,
    MIN(t1.order_date) OVER (PARTITION BY t1.user_id) AS first_order_date
FROM
    user_orders t1;
```
