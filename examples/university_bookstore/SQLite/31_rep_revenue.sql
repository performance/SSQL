-- Topic: Rep Revenue
-- Goal: Sales Rep Name and Total Sales Amount
-- Source: 31_rep_revenue.सं

SELECT sr.name, SUM(s.total_amount)
FROM sales_reps sr
JOIN sales s ON sr.id = s.sales_rep_id
GROUP BY sr.name
ORDER BY SUM(s.total_amount) DESC;
