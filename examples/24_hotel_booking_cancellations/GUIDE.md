# Project 24: Hotel Booking Cancellations (Date Difference Filtering)

This project calculates lead time between booking and arrival days and filters bookings with long lead times.

## DDL Schema (`schema.सं`)
```samskrutam
आवासग्रहणानि इति पटलम् "hotel_bookings" इति आख्या आवासग्रहण इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "booking_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    अतिथिनाम इति वाक्यम् "guest_name" इति आख्या अतिथिनामन् इति नपुंसकलिङ्गम् एकवचनम् च
    आरक्षणदिनम् इति पूर्णाङ्कः "booking_day" इति आख्या आरक्षणदिन इति नपुंसकलिङ्गम् एकवचनम् च
    आगमनदिनम् इति पूर्णाङ्कः "arrival_day" इति आख्या आगमनदिन इति नपुंसकलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
आवासग्रहणेभ्यः आगमनदिनात् आरक्षणदिनम् विशोध्य अवशिष्टम् ३० इत्यस्मात् अधिकम् अस्ति इति नियमेन सङ्ख्या च अतिथिनाम च आगमनदिनात् आरक्षणदिनम् विशोध्य अवशिष्टम् "lead_days" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.booking_id,
    t1.guest_name,
    t1.arrival_day - t1.booking_day AS lead_days
FROM
    hotel_bookings t1
WHERE
    t1.arrival_day - t1.booking_day > 30;
```
