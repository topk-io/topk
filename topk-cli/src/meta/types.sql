-- `\dT [PATTERN]`: column types, without the internal array types, as psql lists them.
SELECT typname AS name
FROM pg_catalog.pg_type
WHERE typname NOT LIKE '\_%' ESCAPE '\' AND typname LIKE {{pattern}} ESCAPE '\'
ORDER BY typname
