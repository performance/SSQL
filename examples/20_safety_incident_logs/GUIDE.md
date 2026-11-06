# Project 20: Safety Incident Logs (String Extraction)

This project demonstrates how to extract error codes from industrial incident log descriptions using `INSTR`/`POSITION` (surfaced in SSQL as the reserved ablative keyword `स्थानात्`) and `SUBSTR` - both already dialect-aware builtins, so no custom function declaration is needed. Under SQLite, `स्थानात्` compiles to `instr(...)`; under PostgreSQL it compiles to `strpos(...)` (PostgreSQL has no `instr`, and its `position` keyword only accepts special `POSITION(... IN ...)` syntax rather than an ordinary function call, so `strpos` - same call syntax and argument order as `instr` - is the correct target, not `position`).

## DDL Schema (`schema.सं`)
```samskrutam
दुर्घटनाः इति पटलम् "incidents" इति आख्या दुर्घटना इति स्त्रीलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "incident_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    विवरणम् इति वाक्यम् "description" इति आख्या विवरण इति नपुंसकलिङ्गम् एकवचनम् च
    गम्भीरता इति वाक्यम् "severity" इति आख्या गम्भीरता इति स्त्रीलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
दुर्घटनाभ्यः सङ्ख्या च विवरणम् च गम्भीरता च विवरणे "ERR" इत्यस्य स्थानात् आरभ्य ६ वर्णानाम् खण्डः "error_code" इति आख्या च आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
SELECT
    t1.incident_id,
    t1.description,
    t1.severity,
    substr(t1.description, nullif(instr(t1.description, 'ERR'), 0), 6) AS error_code
FROM
    incidents t1;
```
