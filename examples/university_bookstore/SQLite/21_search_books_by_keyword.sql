-- Topic: Search Books
-- Goal: Books with 'India' in title
-- Source: 21_search_books_by_keyword.सं

SELECT title, price
FROM books
WHERE title LIKE '%India%';
