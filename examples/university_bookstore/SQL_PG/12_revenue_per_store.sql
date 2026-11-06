SELECT s.name, CAST(SUM(sa.total_amount) AS DOUBLE PRECISION) as revenue
FROM stores s
JOIN sales sa ON s.id = sa.store_id
GROUP BY s.name
ORDER BY revenue DESC
