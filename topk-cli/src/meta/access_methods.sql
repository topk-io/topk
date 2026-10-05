-- `\dA [PATTERN]`: access methods, i.e. the index kinds `INDEX …()` accepts.
SELECT amname AS name, CASE amtype WHEN 'i' THEN 'index' WHEN 't' THEN 'table' END AS type
FROM pg_catalog.pg_am
WHERE amname LIKE {{pattern}} ESCAPE '\'
ORDER BY amname
