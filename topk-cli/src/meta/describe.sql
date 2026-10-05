-- `\d TABLE`: the table's columns, then its indexes.
SELECT column_name AS name, data_type AS type, is_nullable AS nullable
FROM information_schema.columns
WHERE table_schema = 'public' AND table_name = {{table}}
ORDER BY column_name;

SELECT indexname AS name, indexdef AS definition
FROM pg_catalog.pg_indexes
WHERE schemaname = 'public' AND tablename = {{table}}
ORDER BY indexname
