SELECT language, CAST(AVG(price) AS DOUBLE PRECISION) as avg_price
FROM books
GROUP BY language
