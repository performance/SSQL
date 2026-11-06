# Project 29: Cart Abandonment Analysis (EXCEPT Set Difference)

This project finds users who added items to shopping carts but never completed an order using SSQL `EXCEPT` set difference.

## DDL Schema (`schema.सं`)
```samskrutam
क्रयणशकटाः इति पटलम् "shopping_carts" इति आख्या क्रयणशकट इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "cart_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    ग्राहकः इति पूर्णाङ्कः "user_id" इति आख्या ग्राहक इति पुंलिङ्गम् एकवचनम् च
इति पटलम् ।

पूर्णादेशाः इति पटलम् "completed_orders" इति आख्या पूर्णादेश इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "order_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    ग्राहकः इति पूर्णाङ्कः "user_id" इति आख्या ग्राहक इति पुंलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
क्रयणशकटनामानि इति आख्या क्रयणशकटनामन् इति नपुंसकलिङ्गम् बहुवचनम् क्रयणशकटेभ्यः ग्राहकः आह्रियताम् ।
पूर्णादेशनामानि इति आख्या पूर्णादेशनामन् इति नपुंसकलिङ्गम् बहुवचनम् पूर्णादेशेभ्यः ग्राहकः आह्रियताम् ।

क्रयणशकटनामानि च पूर्णादेशनामानि च इत्यनयोः वर्जनम् आह्रियताम् ।
```

## Generated SQL (SQLite)
```sql
WITH क्रयणशकटनामन् AS (
SELECT
    t1.user_id
FROM
    shopping_carts t1
),
पूर्णादेशनामन् AS (
SELECT
    t2.user_id
FROM
    completed_orders t2
)
SELECT * FROM क्रयणशकटनामन् EXCEPT SELECT * FROM पूर्णादेशनामन्;
```
