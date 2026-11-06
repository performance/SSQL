-- Topic: Low Inventory
-- Goal: List books with quantity < 5 in any store
-- Source: 11_books_with_low_inventory.सं
-- Logic: Join Inventory, Books, Stores. Filter quantity < 5.

SELECT b.title, s.name, i.quantity
FROM inventory i
JOIN books b ON i.book_id = b.id
JOIN stores s ON i.store_id = s.id
WHERE i.quantity < 5;
