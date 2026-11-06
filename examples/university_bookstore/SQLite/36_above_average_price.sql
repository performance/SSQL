-- Topic: Above Average Price
-- Source: 36_above_average_price.सं

SELECT title, price
FROM books
WHERE price > (SELECT AVG(price) FROM books);
