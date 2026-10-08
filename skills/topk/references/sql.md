# TopK SQL reference

TopK speaks the PostgreSQL wire protocol, so any Postgres client works (`psql`, `psycopg`, `node-postgres`, ...). Each collection is a table.

## Connect

```bash
psql "host=$TOPK_REGION.sql.topk.io port=5432 user=topk password=$TOPK_API_KEY dbname=topk"
```

```python
import os, psycopg  # pip install "psycopg[binary]"

conn = psycopg.connect(
    host=f"{os.environ['TOPK_REGION']}.sql.topk.io", port=5432,
    user="topk", password=os.environ["TOPK_API_KEY"], dbname="topk", autocommit=True,
)
```

There are no transactions; use autocommit.

## Create a table

Declare only the fields you index. Other columns are schemaless.

```sql
CREATE TABLE articles (
  title  TEXT NOT NULL INDEX keyword_index(),
  body   TEXT          INDEX semantic_index(),   -- semantic + BM25, max 4,096 chars
  year   INT
);
```

Other indexes: `INDEX vector_index(metric = 'cosine')` on `f32_vector(1536)`, `INDEX multi_vector_index(metric = 'maxsim')` on `f16_matrix(128)`, and `INDEX ngram_index()`.

## Write

```sql
INSERT INTO articles (_id, title, body, year) VALUES ('a1', 'Intro to BM25', 'BM25 ranks ...', 2024);
UPDATE articles SET year = 2023 WHERE _id = 'a1';
DELETE FROM articles WHERE year < 2020;
```

`INSERT` with an existing `_id` replaces the document.

## Query

Each search query needs `ORDER BY ... LIMIT n`:

```sql
-- Semantic
SELECT _id, title, semantic_similarity(body, 'how does ranking work') AS score
FROM articles ORDER BY score DESC LIMIT 10;

-- Keyword (bm25_score() requires match() in WHERE)
SELECT _id, title, bm25_score() AS score
FROM articles WHERE match('ranking function', body)
ORDER BY score DESC LIMIT 10;

-- Hybrid
SELECT _id, title,
       semantic_similarity(body, 'how does ranking work') AS sem,
       bm25_score() AS kw
FROM articles
WHERE match('how does ranking work', body) AND year >= 2020
ORDER BY sem * 0.7 + kw * 0.3 DESC LIMIT 10;

-- Count
SELECT COUNT(*) FROM articles WHERE year >= 2020;
```

Other text predicates: `match_all(field, 'a b')`, `match_any(field, ARRAY['a','b'])`, and `match_tokens(ARRAY['a','b'], field)`.

## Partitions (multi-tenancy)

Use `table$partition` or `table PARTITION name` in `SELECT`, `INSERT`, `UPDATE`, and `DELETE`. Partitions are created on first insert.

```sql
INSERT INTO support$acme (_id, body) VALUES ('d1', '...');
SELECT _id FROM support$acme ORDER BY semantic_similarity(body, 'refunds') DESC LIMIT 5;
```

## Inspect

```sql
SELECT table_name FROM information_schema.tables;
SELECT column_name, data_type FROM information_schema.columns WHERE table_name = 'articles';
```

Untyped columns come back as JSON. To force a wire type, cast: `year::int4`, `rating::float8`.
