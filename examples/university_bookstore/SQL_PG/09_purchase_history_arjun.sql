SELECT b.title, s.sale_date, si.price_at_sale
FROM sales s
JOIN customers c ON s.customer_id = c.id
JOIN sale_items si ON s.id = si.sale_id
JOIN books b ON si.book_id = b.id
WHERE c.name = 'Arjun';
