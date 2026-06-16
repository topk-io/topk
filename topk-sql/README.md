# topk-sql

Connect any PostgreSQL client — `psql`, `psycopg2`, `node-postgres`, `tokio-postgres`,
and others — directly to TopK and use SQL to create collections, insert data, and run
search.


## Prerequisites

- **API key** — sign in to [console.topk.io](https://console.topk.io) and generate an API key.
- **Region** — see [docs.topk.io/regions](https://docs.topk.io/regions) for available regions.


## Setup

### Connect

```bash
psql "host=<region>.sql.topk.io port=5432 user=topk password=<api-key> dbname=topk"
```

Replace `<region>` with your selected region and `<api-key>` with your API key. Available regions are listed at [docs.topk.io/regions](https://docs.topk.io/regions).


## Quick Start

Create a collection, insert documents, and run a semantic search:

```sql
-- 1. Create a collection
CREATE TABLE books (
    title   TEXT NOT NULL,
    content TEXT          INDEX semantic_index(),
    author  TEXT NOT NULL,
    rating  FLOAT
);

-- 2. Insert documents
INSERT INTO books (_id, title, content, author, rating)
VALUES
    ('1', 'The Hobbit',
     'A hobbit embarks on an unexpected journey through Middle-earth.',
     'Tolkien', 4.3),
    ('2', '1984',
     'A dystopian novel about totalitarian surveillance and control.',
     'George Orwell', 4.7),
    ('3', 'Dune',
     'An epic saga of politics, religion, and ecology on a desert planet.',
     'Frank Herbert', 4.5);

-- 3. Search
SELECT _id, title, semantic_similarity(content, 'political control and oppression') AS score
FROM books
ORDER BY score DESC
LIMIT 10;
```


## TopK Is Schemaless

TopK collections are schemaless by default. Documents can have any fields with any
types — no schema declaration is required to store or query them. The only exception:
**fields you want to index must be declared**, because index configuration (metric,
index type, etc.) is attached to the field definition.

Because most columns arrive as JSON, clients are expected to deserialize and upcast
values themselves. Most drivers do this automatically (`psycopg2` → `dict`/`list`,
`node-postgres` → object, `tokio-postgres` → `serde_json::Value`). To get a
typed pg column instead, use the `::` cast operator:

```sql
SELECT title, published_year::int4, rating::float8 FROM books LIMIT 10;
```


## SQL Reference

### Table references

DML statements use `<table>` as either a collection name or a collection plus partition:

```text
<table> ::= [schema.]collection[$partition]
```

The `schema` prefix is accepted but ignored — all collections live in a single namespace
per project. Partition can be specified with `$` or the `PARTITION` keyword:

| Form | Example |
|------|---------|
| `collection` | `books` |
| `schema.collection` | `public.books` |
| `collection$partition` | `books$2024` |
| `schema.collection$partition` | `public.books$2024` |
| `collection PARTITION name` | `books PARTITION 2024` |

Partition syntax applies to `SELECT`, `INSERT`, `UPDATE`, and `DELETE` only. DDL
(`CREATE TABLE`, `DROP TABLE`) names collections without partitions;
partitions are created implicitly on first write.


### CREATE TABLE

Schema is defined once at collection creation. Indexes are declared inline on each column.

```sql
CREATE TABLE [IF NOT EXISTS] <table> (
    <column> <type> [NOT NULL] [INDEX <method>(<options>)],
    ...
);
```

`IF NOT EXISTS` suppresses the error if the collection already exists.

#### Column types

| SQL type | TopK field type |
|----------|-----------------|
| `BOOLEAN` | `boolean` |
| `INTEGER` / `BIGINT` / `INT` | `integer` |
| `SMALLINT` / `INT2` / `INT4` | `integer` |
| `FLOAT` / `REAL` / `DOUBLE PRECISION` | `float` |
| `TEXT` / `VARCHAR` | `text` |
| `BYTEA` | `bytes` |
| `TEXT[]` | `list<string>` |
| `INTEGER[]` | `list<integer>` |
| `FLOAT[]` | `list<float>` |
| `JSONB` | `struct` |
| `f32_vector(n)` | `f32_vector(n)` |
| `f16_vector(n)` | `f16_vector(n)` |
| `f8_vector(n)` | `f8_vector(n)` |
| `u8_vector(n)` | `u8_vector(n)` |
| `i8_vector(n)` | `i8_vector(n)` |
| `binary_vector(n)` | `binary_vector(n)` |
| `f32_sparse_vector` | `f32_sparse_vector` |
| `f16_sparse_vector` | `f16_sparse_vector` |
| `f8_sparse_vector` | `f8_sparse_vector` |
| `u8_sparse_vector` | `u8_sparse_vector` |
| `i8_sparse_vector` | `i8_sparse_vector` |
| `f32_matrix(n)` | `f32_matrix(n)` |
| `f16_matrix(n)` | `f16_matrix(n)` |
| `f8_matrix(n)` | `f8_matrix(n)` |
| `u8_matrix(n)` | `u8_matrix(n)` |
| `i8_matrix(n)` | `i8_matrix(n)` |

`NOT NULL` marks a field as required. All columns are optional by default.

#### Index types

| Method | Applies to | Options |
|--------|------------|---------|
| `keyword_index()` | `TEXT`, `VARCHAR` | — |
| `semantic_index()` | `TEXT`, `VARCHAR` | — |
| `vector_index()` | `*_vector(n)`, `*_sparse_vector` | `metric`: `cosine` \| `dot_product` \| `euclidean` \| `hamming` |
| `multi_vector_index()` | `*_matrix(n)` | `metric`: `maxsim`; `quantization`: `1bit` \| `2bit` \| `scalar`; `width`; `top_k` |

#### Example

```sql
CREATE TABLE books (
    title          TEXT NOT NULL               INDEX keyword_index(),
    author         TEXT NOT NULL,
    published_year INTEGER NOT NULL,
    rating         FLOAT,
    genre          TEXT,
    in_print       BOOLEAN,
    bio            TEXT                        INDEX semantic_index(),
    embedding      f32_vector(4)               INDEX vector_index(metric = 'cosine'),
    sparse_emb     f32_sparse_vector           INDEX vector_index(metric = 'dot_product'),
    multi_emb      f32_matrix(4)               INDEX multi_vector_index(metric = 'maxsim'),
    tags           TEXT[],
    checksum       BYTEA,
    metadata       JSONB
);
```


### DROP TABLE

```sql
DROP TABLE [IF EXISTS] <table>;
```

`IF EXISTS` suppresses the error if the collection does not exist.


### INSERT

Upsert semantics: inserting a document with an existing `_id` replaces it.
`_id` is required and must appear in the column list.

```sql
INSERT INTO <table> (<col>, ...) VALUES (<val>, ...) [, (<val>, ...) ...];
```

Scalars use plain literals. TopK-native values use `::topk_type` casts or constructor
calls — see [Type System](#type-system). Prefer casts in `VALUES`:

```sql
INSERT INTO books (_id, title, author, published_year, rating, embedding, sparse_emb)
VALUES
    (
        'hobbit', 'The Hobbit', 'Tolkien', 1937, 4.3,
        '[1.0, 0.0, 0.0, 0.0]'::f32_vector,
        '{"0": 1.0, "1": 0.5}'::f32_sparse_vector
    );
```

Standard PostgreSQL casts (`::float8`, `CAST(… AS text)`, …) are not supported in
`VALUES`. `INSERT … SELECT`, `ON CONFLICT`, and `RETURNING` are not supported.


### UPDATE

Updates one or more fields on existing documents. `_id` cannot be updated. A `WHERE`
clause is required and must resolve to a set of document IDs.

Value expressions in `SET` follow the same rules as [INSERT](#insert) `VALUES`.

```sql
UPDATE <table> SET <col> = <val> [, ...] WHERE _id = '<id>';
UPDATE <table> SET <col> = <val> [, ...] WHERE _id IN ('<id1>', '<id2>', ...);
```


### DELETE

Deletes documents by ID or by filter expression. A `WHERE` clause is required unless
the target is a partition, in which case the entire partition is dropped.

```sql
DELETE FROM <table> WHERE _id = '<id>';
DELETE FROM <table> WHERE _id IN ('<id1>', '<id2>', ...);
DELETE FROM <table> WHERE <filter_expr>;
DELETE FROM <collection>$<partition>;
```


### SELECT

```sql
SELECT <projection> FROM <table>
  [WHERE <filter>]
  [ORDER BY <expr> [ASC | DESC]]
  [LIMIT <n>]
```

`OFFSET` is not supported — use `LIMIT` only.

Only a single `ORDER BY` key is supported. `ORDER BY` must be paired with `LIMIT`.

#### Projection

Each item in the select list must be one of:

- A named column: `SELECT title, rating`
- An aliased expression: `SELECT rating * 2 AS double_rating`
- A search function: `SELECT vector_distance(embedding, '[1,0,0,0]'::f32_vector) AS score`
- A wire-type cast: `SELECT rating::float8 AS rating_f64`

`SELECT *` is not supported — list columns explicitly.

Indexed vector fields (`embedding`, `sparse_emb`, `multi_emb`, …) cannot be selected
directly — use search functions (`vector_distance`, …) instead. Struct subfields
(`metadata.publisher`) can be selected; whole struct columns cannot.

Wire-type casts (`::int4`, `::float8`, `::text`, …) are accepted in the SELECT list
and affect `RowDescription` OIDs only — see [Output type mapping](#output-type-mapping).
Unaliased casts use the inner expression as the column name (`SELECT title::text` →
column `title`).

`COUNT(*)` is supported. Default column name is `_count`; `AS` renames it.
`COUNT(*)` cannot be combined with other columns in the same `SELECT` list.

#### WHERE filters

| Operator | Example |
|----------|---------|
| `=`, `<>`, `!=`, `<`, `<=`, `>`, `>=` | `rating > 4.0` |
| `AND`, `OR`, `NOT` | `genre = 'fantasy' AND in_print = true` |
| `IS NULL`, `IS NOT NULL` | `checksum IS NOT NULL` |
| `IN`, `NOT IN` | `genre IN ('fantasy', 'fiction')` |
| `BETWEEN`, `NOT BETWEEN` | `published_year BETWEEN 1900 AND 2000` |
| `LIKE`, `NOT LIKE` | `title LIKE 'The%'` |
| `~` (regex) | `author ~ 'Tol.*'` |
| Arithmetic | `rating * 10 > 40` |
| `CASE WHEN … THEN … ELSE … END` | — |
| `contains(field, scalar)` | `contains(tags, 'classic')` |
| `match(query [, field [, weight [, all]]])` | `match('hobbit rings', title)` |
| `match_tokens(tokens [, field [, all]])` | `match_tokens(ARRAY['love', 'classic'], tags)` |
| `match_all(field, query)` | `match_all(title, 'hobbit rings')` |
| `match_any(field, query)` | `match_any(tags, ARRAY['love', 'classic'])` |

`match(...)` and `match_tokens(...)` search keyword-indexed text and can be ranked
with `bm25_score()`. Combine text searches with `AND` / `OR`; add metadata filters
with `AND`:

```sql
WHERE (match('hobbit', title) OR match('rings', title))
  AND published_year > 1940
```

`match(...) OR published_year = 1813` is not supported. Use `match_any(...)` and
`match_all(...)` for boolean keyword predicates; they work in ordinary logical
expressions but are not ranked by `bm25_score()`.

Whole-value equality on complex types (e.g. `tags = ARRAY['a']`) is not supported.
`contains` requires a scalar needle, not an array.


### information_schema

Two virtual tables are supported. `SELECT *` is not allowed — specify column names explicitly.

#### information_schema.tables

Returns one row per collection in the project.

```sql
SELECT table_name, table_schema, table_type FROM information_schema.tables;
```

WHERE clauses are accepted but **silently ignored** — all collections are always returned.

| Column | Type | Value |
|--------|------|-------|
| `table_name` | `text` | collection name |
| `table_schema` | `text` | `"public"` |
| `table_type` | `text` | `"BASE TABLE"` |
| `table_owner` | `text` | `"topk"` |


#### information_schema.columns

Returns one row per declared field. `WHERE table_name = '<name>'` is required. Additional `AND` clauses (e.g. `AND table_schema = 'public'`) are accepted but ignored.

```sql
SELECT column_name, data_type
FROM information_schema.columns
WHERE table_name = 'books';
```

| Column | Type | Value |
|--------|------|-------|
| `column_name` | `text` | field name |
| `data_type` | `text` | see mapping below |
| `is_nullable` | `text` | `"YES"` or `"NO"` |
| `table_name` | `text` | collection name |

**Data type mapping:**

| TopK field type | `data_type` |
|-----------------|-------------|
| `text` | `text` |
| `integer` | `bigint` |
| `float` | `double precision` |
| `boolean` | `boolean` |
| `bytes` | `bytea` |
| `*_vector(n)` (all dense variants) | `real[]` |
| `*_sparse_vector` (all variants) | `jsonb` |
| `list`, `struct`, `*_matrix` | `jsonb` |


### EXPLAIN

Returns the parsed `Statement` AST as a single `plan TEXT` column.

```sql
EXPLAIN <statement>;
EXPLAIN VERBOSE <statement>;
```


### Session commands

| Command | Behavior |
|---------|----------|
| `SET consistency_level = 'indexed'` | Indexed consistency for subsequent reads |
| `SET consistency_level = 'strong'` | Strong consistency for subsequent reads |
| `SET consistency_level = 'default'` | Clears the session override (router default) |
| `SHOW consistency_level` | Returns the current consistency level |

`SET`/`SHOW` only recognize `consistency_level`; all other variable names return an error.

The following commands are accepted and silently succeed:

| Command |
|---------|
| `BEGIN` |
| `COMMIT` |
| `ROLLBACK` |
| `DISCARD <anything>` |

**Transactions are not supported.** `BEGIN`/`COMMIT`/`ROLLBACK` are accepted without
error so that clients which wrap every statement in a transaction by default (psycopg2,
SQLAlchemy, JDBC) can connect and operate normally. `ROLLBACK` does **not** undo
writes — do not rely on rollback semantics.


## Type System

TopK-native literals use **`::topk_type` casts** (preferred in `INSERT`/`UPDATE`
`VALUES` and search-function arguments) or equivalent **constructor calls**.
PostgreSQL wire-type casts (`::float8`, …) apply only in `SELECT` projection.

| Constructor | Example |
|-------------|---------|
| `f32_vector(ARRAY[…])` | `'[0.1, 0.2, 0.3]'::f32_vector` or `f32_vector(ARRAY[0.1, 0.2, 0.3])` |
| `f16_vector(ARRAY[…])` | (same pattern as f32) |
| `f8_vector(ARRAY[…])` | (same pattern) |
| `u8_vector(ARRAY[…])` | (same pattern) |
| `i8_vector(ARRAY[…])` | (same pattern) |
| `binary_vector(ARRAY[…])` | (same pattern) |
| `f32_sparse_vector(ARRAY[idx], ARRAY[val])` | `f32_sparse_vector(ARRAY[0, 2], ARRAY[1.0, 0.5])` |
| `f32_sparse_vector(JSON)` | `'{"0": 1.0, "2": 0.5}'::f32_sparse_vector` |
| `f16_sparse_vector(…)` | (same pattern as f32) |
| `f8_sparse_vector(…)` | (same pattern) |
| `u8_sparse_vector(…)` | (same pattern) |
| `i8_sparse_vector(…)` | (same pattern) |
| `f32_matrix(ARRAY[ARRAY[row1],…])` | `'[[1.0, 0.0], [0.5, 0.5]]'::f32_matrix` |
| `f16_matrix(…)` | (same pattern as f32) |
| `f8_matrix(…)` | (same pattern) |
| `u8_matrix(…)` | (same pattern) |
| `i8_matrix(…)` | (same pattern) |
| `bytes('hexstring')` | `bytes('deadbeef')` |
| `struct(key1, val1, …)` | `struct('publisher', 'Penguin', 'pages', 320)` |
| `ARRAY[elem, …]` | `ARRAY['classic', 'fiction']` (list) |

### Complex types → JSON

No PostgreSQL OID exists for TopK-native vector/matrix/sparse/list types. They are
always returned as JSON (OID 114).

| TopK type | JSON wire representation |
|-----------|--------------------------|
| Dense vector | `[0.1, 0.2, 0.3]` |
| Sparse vector | `{"indices":[0,2],"values":[1.0,0.5]}` or `{"0":1.0,"2":0.5}` |
| Matrix (multi-vector) | `[[1.0,0.0],[0.5,0.1]]` (row-major) |
| List | `["a","b"]` / `[1,2,3]` |
| Struct | `{"publisher":"Scribner","pages":180}` |
| Binary | `\xdeadbeef` |

### Output type mapping

pgwire maps SELECT-list expressions to PostgreSQL OIDs. Explicit `::cast` in the
projection list overrides inference. Casts are stripped from the query plan — they
only affect the wire type.

| Expression | pg OID |
|------------|--------|
| `::bool` | 16 `BOOL` |
| `::smallint` / `::int2` | 21 `INT2` |
| `::int` / `::int4` | 23 `INT4` |
| `::bigint` / `::int8` | 20 `INT8` |
| `::real` / `::float4` | 700 `FLOAT4` |
| `::float` / `::float8` | 701 `FLOAT8` |
| `::text` | 25 `TEXT` |
| `::bytea` | 17 `BYTEA` |
| `::json` / `::jsonb` | 114 `JSON` |
| plain column (no cast) | 114 `JSON` |
| search function (no cast) | 700 `FLOAT4` |
| `COUNT(*)` (no cast) | 20 `INT8` |


## Search Functions

Scoring functions go in `SELECT` only — alias them, then reference the alias in `ORDER BY`. Filter functions go in `WHERE`.

| Function | Use in | Description |
|----------|--------|-------------|
| `vector_distance(field, query [, skip_refine])` | SELECT | Dense or sparse ANN distance |
| `multi_vector_distance(field, query [, candidates])` | SELECT | Multi-vector MaxSim distance |
| `bm25_score([b, k1])` | SELECT | Keyword relevance score for `match(...)` / `match_tokens(...)` |
| `semantic_similarity(field, query)` | SELECT | Semantic embedding similarity |
| `boost(score, condition, factor)` | SELECT / ORDER BY | Multiply score when condition is true |
| `contains(field, scalar)` | WHERE | List membership or string substring |
| `match(query [, field [, weight [, all]]])` | WHERE | Keyword text search; can be ranked with `bm25_score()` |
| `match_tokens(tokens [, field [, all]])` | WHERE | Keyword token search; can be ranked with `bm25_score()` |
| `match_all(field, query)` | WHERE | Boolean keyword predicate requiring all terms |
| `match_any(field, query)` | WHERE | Boolean keyword predicate requiring any term |

All scoring functions return `f32` (`FLOAT4` OID when projected without cast).
`bm25_score()` requires at least one `match(...)` or `match_tokens(...)` text filter
in `WHERE`.

### Example queries

```sql
-- Vector ANN
SELECT
    _id,
    title,
    vector_distance(embedding, '[1,0,0,0]'::f32_vector) AS vec_dist
FROM books
ORDER BY vec_dist DESC  -- DESC for cosine/dot_product; ASC for euclidean
LIMIT 3;

-- Sparse vector ANN
SELECT
    _id,
    title,
    vector_distance(sparse_emb, '{"0":1.0,"1":0.5}'::f32_sparse_vector) AS vec_dist
FROM books
ORDER BY vec_dist DESC
LIMIT 3;

-- Multi-vector MaxSim
SELECT
    _id,
    title,
    multi_vector_distance(multi_emb, '[[1.0,0.0,0.0,0.0]]'::f32_matrix) AS vec_dist
FROM books
ORDER BY vec_dist DESC
LIMIT 3;

-- Full-text BM25
SELECT
    _id,
    title,
    bm25_score() AS bm25_score
FROM books
WHERE match('hobbit rings', title)
ORDER BY bm25_score DESC
LIMIT 5;

-- Semantic similarity
SELECT
    _id,
    title,
    semantic_similarity(bio, 'tales of magic and adventure') AS sem_similarity
FROM books
ORDER BY sem_similarity DESC
LIMIT 3;

-- Hybrid: vector + boost
SELECT
    _id,
    title,
    vector_distance(embedding, '[1,0,0,0]'::f32_vector) AS vec_dist
FROM books
WHERE match_any(title, 'tolkien')
ORDER BY boost(vec_dist, in_print = true, 1.5)
LIMIT 5;

-- Boolean keyword predicates
SELECT _id, title
FROM books
WHERE match_all(title, ARRAY['lord', 'rings'])
   OR match_any(tags, ARRAY['classic', 'adventure'])
LIMIT 5;
```
