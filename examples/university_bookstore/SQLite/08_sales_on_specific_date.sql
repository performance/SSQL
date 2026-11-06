-- Topic: Specific Date Sales
-- Goal: Retrieve ID and Total Amount of sales on 2023-10-01
-- Source: 08_sales_on_specific_date.सं

SELECT id, total_amount
FROM sales
WHERE sale_date = '2023-10-01';
