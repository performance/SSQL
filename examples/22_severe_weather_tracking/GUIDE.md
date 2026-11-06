# Project 22: Severe Weather Tracking (Meteorological Range Filtering)

This project tracks severe weather incidents by filtering temperature and rainfall thresholds using pure Sanskrit terms.

## DDL Schema (`schema.सं`)
```samskrutam
ऋतुवृत्तान्ताः इति पटलम् "weather_logs" इति आख्या ऋतुवृत्तान्त इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "log_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    केन्द्रनाम इति वाक्यम् "station_name" इति आख्या केन्द्रनामन् इति नपुंसकलिङ्गम् एकवचनम् च
    अभिलेखदिनाङ्कः इति वाक्यम् "record_date" इति आख्या अभिलेखदिनाङ्क इति पुंलिङ्गम् एकवचनम् च
    तापमानम् इति वास्तविकः "temperature" इति आख्या तापमान इति नपुंसकलिङ्गम् एकवचनम् च
    वृष्टिः इति वास्तविकः "rainfall" इति आख्या वृष्टि इति स्त्रीलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
ऋतुवृत्तान्तेभ्यः तापमानम् ४० इत्यस्मात् अधिकम् अस्ति च वृष्टिः ५० इत्यस्याः अधिका अस्ति च इति नियमेन केन्द्रनाम च अभिलेखदिनाङ्कः च तापमानम् च वृष्टिः च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.station_name,
    t1.record_date,
    t1.temperature,
    t1.rainfall
FROM
    weather_logs t1
WHERE
    (t1.temperature > 40 AND t1.rainfall > 50);
```
