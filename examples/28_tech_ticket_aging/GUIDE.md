# Project 28: Tech Support Ticket Aging (Scalar Date Math)

This project tracks open technical support ticket duration in days using scalar math.

## DDL Schema (`schema.सं`)
```samskrutam
पत्राणि इति पटलम् "tickets" इति आख्या पत्र इति नपुंसकलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "ticket_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    समस्याप्रकारः इति वाक्यम् "issue_type" इति आख्या समस्याप्रकार इति पुंलिङ्गम् एकवचनम् च
    उद्घाटनदिनम् इति पूर्णाङ्कः "open_day" इति आख्या उद्घाटनदिन इति नपुंसकलिङ्गम् एकवचनम् च
    वर्तमानदिनम् इति पूर्णाङ्कः "current_day" इति आख्या वर्तमानदिन इति नपुंसकलिङ्गम् एकवचनम् च
    स्थितिः इति वाक्यम् "status" इति आख्या स्थिति इति स्त्रीलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
पत्रेभ्यः स्थितिः "Open" इत्यनया समा अस्ति इति नियमेन सङ्ख्या च समस्याप्रकारः च वर्तमानदिनात् उद्घाटनदिनम् विशोध्य अवशिष्टम् "open_days" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.ticket_id,
    t1.issue_type,
    t1.current_day - t1.open_day AS open_days
FROM
    tickets t1
WHERE
    t1.status = 'Open';
```
