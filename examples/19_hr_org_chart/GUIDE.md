# Project 19: HR Org Chart (Recursive Hierarchy Mapping)

This project constructs a recursive organizational chart using SSQL Recursive CTEs and cycle prevention.

## DDL Schema (`schema.सं`)
```samskrutam
कर्मचारिणः इति पटलम् "employees" इति आख्या कर्मचारिन् इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "employee_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    नाम इति वाक्यम् "employee_name" इति आख्या नामन् इति नपुंसकलिङ्गम् एकवचनम् च
    प्रबन्धकसङ्ख्या इति पूर्णाङ्कः "manager_id" इति आख्या प्रबन्धकसङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
इति पटलम् ।
```

## SSQL Query (`queries/q1_elegant.सं`)
```samskrutam
प्रवाहः इति आख्या प्रवाह इति पुंलिङ्गम् एकवचनम् आवर्तिका सञ्ज्ञा कर्मचारिभ्यः प्रबन्धकसङ्ख्या रिक्ता अस्ति इति नियमेन सङ्ख्या च नाम च १ स्तरः इति आख्या च आह्रियन्ताम् च कर्मचारिभ्यः प्रवाहेण सह कर्मचारिणः प्रबन्धकसङ्ख्या प्रवाहस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन कर्मचारिणः सङ्ख्या च कर्मचारिणः नाम च प्रवाहस्य स्तरः च १ च इत्यनयोः योगः स्तरः इति आख्या च आह्रियन्ताम् च इत्यनयोः सङ्गमः सङ्ख्या चक्रनिवारणेन ।

प्रवाहात्
सङ्ख्या च नाम च स्तरः च
आह्रियन्ताम् ।
```

## Generated SQL (SQLite)
```sql
WITH RECURSIVE प्रवाह AS (
SELECT
    t1.employee_id,
    t1.employee_name,
    1 AS स्तरः,
    CHAR(31) || t1.employee_id || CHAR(31) AS path
FROM
    employees t1
WHERE
    t1.manager_id IS NULL UNION ALL SELECT
    t2.employee_id,
    t2.employee_name,
    t3.स्तरः + 1 AS स्तरः,
    t3.path || t2.employee_id || CHAR(31) AS path
FROM
    employees t2
JOIN प्रवाह t3 ON t2.manager_id = t3.employee_id
WHERE
    instr(t3.path, CHAR(31) || t2.employee_id || CHAR(31)) = 0
)
SELECT
    t4.employee_id,
    t4.employee_name,
    t4.स्तरः
FROM
    प्रवाह t4;
```
