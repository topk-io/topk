-- `\?`: the meta-commands `topk sql` expands.
SELECT '\dt [PATTERN]' AS command, 'List tables' AS description
UNION ALL SELECT '\d [TABLE]', 'Describe a table''s columns and indexes; without TABLE, list tables'
UNION ALL SELECT '\di [PATTERN]', 'List indexes'
UNION ALL SELECT '\dA [PATTERN]', 'List access methods (index kinds)'
UNION ALL SELECT '\dT [PATTERN]', 'List data types'
UNION ALL SELECT '\dn [PATTERN]', 'List schemas'
UNION ALL SELECT '\l [PATTERN]', 'List databases'
UNION ALL SELECT '\du [PATTERN]', 'List roles (also \dg)'
UNION ALL SELECT '\?', 'Show this help'
