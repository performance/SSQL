-- 34. Zero Sales Reps: Find Reps who have made NO sales
SELECT 
    sr.name
FROM 
    sales_reps sr
LEFT JOIN 
    sales s ON sr.id = s.sales_rep_id
WHERE 
    s.id IS NULL
ORDER BY 
    sr.name;
