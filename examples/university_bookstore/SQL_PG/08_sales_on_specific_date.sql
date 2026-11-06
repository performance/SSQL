-- Topic: Specific Date Sales
-- Note: Postgres date can optionally be cast, but string comparison works if format standard
SELECT id, total_amount FROM sales WHERE sale_date = '2023-10-01';
