SELECT
    COUNT(DISTINCT t1.language),
    SUM(DISTINCT t1.price)
FROM
    books t1;
