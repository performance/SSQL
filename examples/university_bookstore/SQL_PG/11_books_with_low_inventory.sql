-- Copy of SQLite logic for standard SQL queries

-- 11
SELECT b.title, s.name, i.quantity
FROM inventory i
JOIN books b ON i.book_id = b.id
JOIN stores s ON i.store_id = s.id
WHERE i.quantity < 5;
