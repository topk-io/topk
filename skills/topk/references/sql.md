# TopK SQL reference

Run TopK SQL with `topk sql` (see [cli.md](cli.md)). Each collection is a table. The full SQL reference, including every function and data type, is at https://docs.topk.io/sdk/topk-sql/overview (fetch it as markdown by adding `.md`).

## Create a table

Declare only the fields you index. Other fields are schemaless.

```sql
CREATE TABLE IF NOT EXISTS articles (
  title  TEXT NOT NULL INDEX keyword_index(),
  body   TEXT          INDEX semantic_index(),   -- semantic + BM25, max 32,768 chars
  year   INT
);
```

Other indexes: `INDEX vector_index(metric = 'cosine')` on `f32_vector(1536)`, `INDEX multi_vector_index(metric = 'maxsim')` on `f16_matrix(128)`, and `INDEX ngram_index()`. There's no `CREATE INDEX` or `ALTER TABLE`. `DROP TABLE [IF EXISTS] articles` deletes a collection (SKILL.md rule 7).

## Write

```sql
INSERT INTO articles (_id, title, body, year) VALUES
  ('a1', 'Intro to BM25', 'BM25 ranks ...', 2024),
  ('a2', 'Vector search', 'Nearest neighbours ...', 2023)
RETURNING _lsn;
UPDATE articles SET year = 2023 WHERE _id = 'a1';
DELETE FROM articles WHERE year < 2020;
```

- `INSERT` is an upsert: an existing `_id` is replaced. `_id` must be in the column list. `INSERT … SELECT` isn't supported.
- `UPDATE` needs `WHERE _id = …` or `WHERE _id IN (…)`. `DELETE` takes any filter.
- Vectors in `VALUES` use TopK casts: `'[0.1, 0.2]'::f32_vector`, `'{"0": 1.0}'::f32_sparse_vector`.
- There are no transactions: each statement is applied on its own.

## Read your own writes

`RETURNING _lsn` returns the LSN of the write. Pass it to a read so it includes that write:

```sql
SELECT COUNT(*) FROM articles WITH (required_lsn = 42);
SELECT _id, semantic_similarity(body, 'ranking') AS score
FROM articles WITH (required_lsn = 42) WHERE year >= 2020
ORDER BY score DESC LIMIT 5;
```

`WITH (consistency = 'strong')` reads the latest writes without an LSN, at a higher latency.

## Query

Each search query needs `ORDER BY … LIMIT n`, and every computed column needs an `AS` alias:

```sql
-- Semantic
SELECT _id, title, semantic_similarity(body, 'how does ranking work') AS score
FROM articles ORDER BY score DESC LIMIT 10;

-- Keyword (bm25_score() requires match() or should() in WHERE)
SELECT _id, title, bm25_score() AS score
FROM articles WHERE match('ranking function', body)
ORDER BY score DESC LIMIT 10;

-- Hybrid: should() adds BM25 without dropping documents that lack the words
SELECT _id, title,
       semantic_similarity(body, 'how does ranking work') AS sem,
       bm25_score() AS kw
FROM articles
WHERE should('how does ranking work', body) AND year >= 2020
ORDER BY sem + kw * 0.2 DESC LIMIT 10;

-- Boost: multiply a score when a condition holds
SELECT _id, title, semantic_similarity(body, 'ranking') AS sem
FROM articles
ORDER BY boost(sem, match_any(title, 'ranking'), 1.5) DESC LIMIT 10;

-- Your own vectors (DESC for cosine/dot_product, ASC for euclidean)
SELECT _id, vector_distance(embedding, '[0.1, 0.2, 0.3]'::f32_vector) AS score
FROM articles ORDER BY score DESC LIMIT 10;

-- Count, and counts per group
SELECT COUNT(*) FROM articles WHERE year >= 2020;
SELECT year, COUNT(*) AS n FROM articles GROUP BY year ORDER BY n DESC LIMIT 10;
```

- Only one `ORDER BY` key: combine signals into one expression. `OFFSET` needs a `LIMIT`.
- `COUNT(*)` without `GROUP BY` can't be combined with other columns. `GROUP BY` aggregates (`MAX`, `AVG`, …) take column names, not scores.
- Text predicates: `match(q, field)` (filters, enables BM25), `should(q, field)` (BM25 only, no filtering), `match_all(field, 'a b')` and `match_any(field, ARRAY['a','b'])` (filter only, no BM25). `match` and `should` can't be `OR`-ed with metadata conditions; use `match_any`/`match_all` there.
- Other filters: `=`, `<>`, `<`, `IN`, `BETWEEN`, `LIKE`, `~` (regex), `IS NULL`, `contains(tags, 'a')`, `'a' = ANY(tags)`.
- Timestamps: `published_at < TIMESTAMP '2024-01-01'`. A plain string compares as text and matches nothing.
- Struct fields: `SELECT metadata.publisher …`; a whole struct can't be selected.
- No string functions (`SUBSTR`, `LEFT`, `lower`), `ILIKE` or `SELECT DISTINCT` (SKILL.md rule 3). For case-insensitive word matches, use `match_any`/`match_all` on a keyword or semantic index; to list each value once, `SELECT source, COUNT(*) FROM docs GROUP BY source`.

## Partitions (multi-tenancy)

Use `table$partition` or `table PARTITION name` in `SELECT`, `INSERT`, `UPDATE`, and `DELETE`. Partitions are created on first insert.

```sql
INSERT INTO support$acme (_id, body) VALUES ('d1', '...');
SELECT _id, semantic_similarity(body, 'refunds') AS score FROM support$acme ORDER BY score DESC LIMIT 5;
DELETE FROM support$acme;          -- drops the whole partition (applies in about a minute)
SHOW PARTITIONS FROM TABLE support;
```

In the shell, quote the statement with single quotes, or escape the `$` (`support\$acme`) inside double quotes.

## Inspect

```sql
SHOW TABLES;
SHOW TABLES STARTS WITH 'arti' LIMIT 10;
SHOW PARTITIONS FROM TABLE support STARTS WITH 'acme';
DESCRIBE articles;
```

`DESCRIBE` lists each field with its type, nullability and index, written as in `CREATE TABLE`.
