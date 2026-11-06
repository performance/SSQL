# Project 30: Server Resource Utilization (Moving Average Window Framing)

This project computes a 3-entry moving average of CPU utilization per server over time using SSQL Window Frames.

## DDL Schema (`schema.सं`)
```samskrutam
सेवकवृत्तान्ताः इति पटलम् "server_logs" इति आख्या सेवकवृत्तान्त इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "log_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    सेवकः इति पूर्णाङ्कः "server_id" इति आख्या सेवक इति पुंलिङ्गम् एकवचनम् च
    अभिलेखसमयः इति पूर्णाङ्कः "log_time" इति आख्या अभिलेखसमय इति पुंलिङ्गम् एकवचनम् च
    उपयोगः इति वास्तविकः "cpu_usage" इति आख्या उपयोग इति पुंलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
सेवकवृत्तान्तेभ्यः सेवकः च अभिलेखसमयः च उपयोगः च उपयोगानाम् माध्यम् यत्र सेवकस्य आधारेण विभाजनम् अभिलेखसमयस्य च सङ्ख्यायाः च अनुसारम् आरोहक्रमेण पङ्क्तयः २ पूर्ववर्तितः आरभ्य वर्तमानपङ्क्तिपर्यन्तम् "moving_cpu_avg" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.server_id,
    t1.log_time,
    t1.cpu_usage,
    AVG(t1.cpu_usage) OVER (PARTITION BY t1.server_id ORDER BY t1.log_time ASC NULLS LAST, t1.log_id ASC NULLS LAST ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) AS moving_cpu_avg
FROM
    server_logs t1;
```
