-- `\du [PATTERN]`, `\dg [PATTERN]`: roles.
SELECT rolname AS name
FROM pg_catalog.pg_roles
WHERE rolname LIKE {{pattern}} ESCAPE '\'
ORDER BY rolname
