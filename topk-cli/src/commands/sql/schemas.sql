-- `\dn [PATTERN]`: schemas, without the system ones, as psql lists them.
SELECT schema_name AS name, schema_owner AS owner
FROM information_schema.schemata
WHERE schema_name NOT LIKE 'pg\_%' ESCAPE '\' AND schema_name LIKE {{pattern}} ESCAPE '\'
ORDER BY schema_name
