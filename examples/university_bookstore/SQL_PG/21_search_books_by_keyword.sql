-- Topic: Search Books
-- Goal: Books with 'India' in title
-- Source: 21_search_books_by_keyword.सं
-- Same as SQLite (query is also used, unaltered, by 22/24/25/26/31's own golden files)

SELECT title, price FROM books WHERE title LIKE '%India%';
