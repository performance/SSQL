-- Topic: Case Expression
-- Source: 38_case_expression.सं

SELECT title,
       CASE 
           WHEN price < 20 THEN 'Budget'
           WHEN price < 50 THEN 'Standard'
           ELSE 'Premium'
       END
FROM books;
