SELECT language, CAST(AVG(price) AS REAL) as avg_price
FROM books
GROUP BY language
