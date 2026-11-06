SELECT title, CAST(price AS REAL) as price, language FROM books WHERE isbn = ?
