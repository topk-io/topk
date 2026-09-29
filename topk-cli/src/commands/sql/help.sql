-- `\?`: the meta-commands `topk sql` expands.
SELECT '\dt [PATTERN]' AS command, 'List tables' AS description
UNION ALL SELECT '\d [TABLE]', 'Describe a table''s columns and indexes; without TABLE, list tables'
UNION ALL SELECT '\di [PATTERN]', 'List indexes'
UNION ALL SELECT '\?', 'Show this help'
