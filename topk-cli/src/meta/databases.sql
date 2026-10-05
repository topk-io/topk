-- `\l [PATTERN]`: databases.
SELECT datname AS name, datcollate AS collation
FROM pg_catalog.pg_database
WHERE datname LIKE {{pattern}} ESCAPE '\'
ORDER BY datname
