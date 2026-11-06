# Project 16: Influencer Moving Average (Window Frames)

This project demonstrates how to compute a 3-post moving average of daily engagement/likes using SSQL window frames.

## DDL Schema (`schema.सं`)
```samskrutam
प्रभावकाः इति पटलम् "influencers" इति आख्या प्रभावक इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "influencer_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    लेखदिनाङ्कः इति वाक्यम् "post_date" इति आख्या लेखदिनाङ्क इति पुंलिङ्गम् एकवचनम् च
    प्रतिक्रिया इति पूर्णाङ्कः "likes" इति आख्या प्रतिक्रिया इति स्त्रीलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
प्रभावकेभ्यः लेखदिनाङ्कः च प्रतिक्रिया च प्रतिक्रियाणाम् माध्यम् यत्र लेखदिनाङ्कस्य च सङ्ख्यायाः च अनुसारम् आरोहक्रमेण पङ्क्तयः २ पूर्ववर्तितः आरभ्य वर्तमानपङ्क्तिपर्यन्तम् "moving_avg" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.post_date,
    t1.likes,
    AVG(t1.likes) OVER (ORDER BY t1.post_date ASC NULLS LAST, t1.influencer_id ASC NULLS LAST ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) AS moving_avg
FROM
    influencers t1;
```
