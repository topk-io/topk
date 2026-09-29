-- `\di [PATTERN]`: indexes whose name matches the pattern (LIKE syntax), with their tables.
SELECT indexname AS name, tablename, indexdef AS definition
FROM pg_catalog.pg_indexes
WHERE schemaname = 'public' AND indexname LIKE {{pattern}} ESCAPE '\'
ORDER BY tablename, indexname
