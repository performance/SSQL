SELECT sr.name, CAST(SUM(s.total_amount) AS DOUBLE PRECISION)
FROM sales_reps sr
JOIN sales s ON sr.id = s.sales_rep_id
GROUP BY sr.name
ORDER BY CAST(SUM(s.total_amount) AS DOUBLE PRECISION) DESC;
