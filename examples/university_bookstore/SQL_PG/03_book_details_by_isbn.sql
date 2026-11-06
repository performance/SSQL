SELECT title, CAST(price AS DOUBLE PRECISION) as price, language FROM books WHERE isbn = $1
