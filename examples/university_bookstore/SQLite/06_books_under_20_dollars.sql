-- Topic: Affordable Books
-- Goal: Retrieve title and price of books costing less than 20
-- Source: 06_books_under_20_dollars.सं

SELECT title, price
FROM books
WHERE price < 20;
