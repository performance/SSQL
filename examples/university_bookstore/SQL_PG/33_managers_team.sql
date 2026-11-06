-- 33. Manager's Team: List all interactions between Managers and Reps
SELECT 
    m.name as manager_name,
    r.name as rep_name,
    r.email as rep_email
FROM 
    sales_reps m
JOIN 
    sales_reps r ON m.id = r.manager_id
ORDER BY 
    manager_name, rep_name;
