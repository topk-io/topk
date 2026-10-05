-- `\dt [PATTERN]`: tables whose name matches the pattern (LIKE syntax).
SELECT table_name AS name
FROM information_schema.tables
WHERE table_schema = 'public' AND table_name LIKE {{pattern}} ESCAPE '\'
ORDER BY table_name
