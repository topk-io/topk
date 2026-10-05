# TopK Python reference (`topk-sdk`)

## Contents
- Install and client
- Create a collection
- Write documents (upsert, update, delete, chunking)
- Query: semantic, keyword, hybrid, vector, filters
- Partitions (multi-tenancy)
- Read your own writes
- Async client
- Errors

## Install and client

```bash
pip install topk-sdk
```

```python
import os
from topk_sdk import Client

client = Client(api_key=os.environ["TOPK_API_KEY"], region=os.environ["TOPK_REGION"])
```

## Create a collection

```python
from topk_sdk.error import CollectionAlreadyExistsError
from topk_sdk.schema import text, int, float, bool, keyword_index, semantic_index

try:
    client.collections().create(
        "articles",
        schema={
            "title": text().required().index(keyword_index()),
            "body": text().index(semantic_index()),  # semantic + BM25, max 4,096 chars per value
            "source": text().required(),
            "year": int(),
        },
    )
except CollectionAlreadyExistsError:
    pass
```

`int`, `float`, `bool`, and `list` shadow Python builtins when you import them. Alias them if needed: `from topk_sdk.schema import int as int_`.

Other field types: `f32_vector(dim)`, `f16_vector`, `f8_vector`, `u8_vector`, `i8_vector`, `binary_vector`, `f32_sparse_vector()`, `u8_sparse_vector()`, `matrix(...)`, `bytes()`, `timestamp()`, `list("text" | "integer" | "float")`, `struct({...})`.

Vector index: `f32_vector(dimension=1536).index(vector_index(metric="cosine"))`.

To manage collections, use `client.collections().list()`, `.get(name)`, and `.delete(name)`. Don't recreate a deleted collection under the same name; use a new name.

## Write documents

```python
lsn = client.collection("articles").upsert([
    {"_id": "a1", "title": "Intro to BM25", "body": "BM25 ranks documents by ...", "source": "blog", "year": 2024},
])
```

- Upsert replaces whole documents. To merge fields into existing documents, use `update(docs, fail_on_missing=False)`. In `topk-sdk` 0.15 the `fail_on_missing` argument is required at runtime, even though the type stub marks it optional. Set it to `True` to raise `DocumentValidationError` (`DocumentNotFound`) for unknown ids.
- To delete by id: `delete(["a1", "a2"])`. To delete by filter: `delete(field("year") < 2020)`.
- Each call returns an LSN string. Keep the last one if you need to read your writes (see below).

### Chunk long text for `semantic_index()` fields

```python
def chunk(text: str, size: int = 3000, overlap: int = 200) -> list[str]:
    """Split on paragraph boundaries into chunks of at most `size` characters."""
    paras, chunks, cur = text.split("\n\n"), [], ""
    for p in paras:
        while len(p) > size:  # a single huge paragraph
            chunks.append(p[:size]); p = p[size - overlap:]
        if len(cur) + len(p) + 2 > size and cur:
            chunks.append(cur); cur = cur[-overlap:]
        cur = f"{cur}\n\n{p}" if cur else p
    if cur:
        chunks.append(cur)
    return chunks

docs = [
    {"_id": f"{path}#{i}", "source": path, "chunk_index": i, "title": title, "body": c}
    for i, c in enumerate(chunk(full_text))
]
for i in range(0, len(docs), 500):  # stay under the 8MB request limit
    lsn = client.collection("articles").upsert(docs[i : i + 500])
```

### One result per file

```python
def search_files(q: str, k: int = 5) -> list[dict]:
    rows = client.collection("articles").query(
        select("source", "title", score=fn.semantic_similarity("body", q))
        .sort(field("score"), asc=False)
        .limit(k * 4)  # over-fetch chunks, then keep the best chunk per file
    )
    best: dict[str, dict] = {}
    for r in rows:
        if r["source"] not in best or r["score"] > best[r["source"]]["score"]:
            best[r["source"]] = r
    return sorted(best.values(), key=lambda r: r["score"], reverse=True)[:k]
```

## Query

Import from `topk_sdk.query`: `select, field, fn, match, should, not_, all, any, literal`.

Each query is `select(...)` → optional `.filter(...)` → `.sort(expr, asc=...)` → `.limit(k)`. Results are a list of dicts that include `_id` and every selected key.

### Semantic search

```python
from topk_sdk.query import select, field, fn

results = client.collection("articles").query(
    select("title", "source", score=fn.semantic_similarity("body", "how does ranking work"))
    .sort(field("score"), asc=False)
    .limit(10)
)
```

### Keyword search (BM25)

```python
from topk_sdk.query import select, field, fn, match

results = client.collection("articles").query(
    select("title", score=fn.bm25_score())
    .filter(match("ranking function", field="body"))  # required for bm25_score()
    .sort(field("score"), asc=False)
    .limit(10)
)
```

`match(q)` matches any query term (OR). Pass `all=True` to require every term. To boost without filtering documents out, use `should(term, field=...)`, and combine with `&`: `match("bm25") & should("ranking")`.

### Hybrid search (semantic + keyword)

```python
q = "how does ranking work"
results = client.collection("articles").query(
    select(
        "title",
        sem=fn.semantic_similarity("body", q),
        kw=fn.bm25_score(),
    )
    .filter(match(q, field="body"))
    .sort(field("sem") * 0.7 + field("kw") * 0.3, asc=False)  # weight inside sort, not a 2nd select
    .limit(10)
)
```

The `sem` and `kw` scores have different ranges. Print them for a few queries before trusting the weights. `match(q)` drops documents with no shared terms. If that's too strict, filter with `should(q)`, or rank by `sem` and add `field("title").match_any(q).choose(1.0, 0.0)` as a boost.

To boost a match in another field, use `field("name").match_any("noise cancelling")`. It returns a boolean you can turn into a number with `.choose(x, y)` or `.boost(condition, factor)`.

### Vector search (your own embeddings)

```python
results = client.collection("items").query(
    select("name", score=fn.vector_distance("embedding", query_vector))
    .sort(field("score"), asc=False)  # cosine/dot_product: desc; euclidean: asc=True
    .limit(10)
)
```

### Filters

```python
from topk_sdk.query import field, not_, all, any

.filter(field("year") >= 2020)
.filter((field("source") == "blog") & (field("year") < 2025))
.filter(field("category").in_(["audio", "office"]))
.filter(field("price").lte(100) & field("in_stock") == True)
.filter(field("title").starts_with("Intro"))
.filter(field("tags").contains("search"))  # list fields
.filter(field("deleted_at").is_null())
.filter(not_(field("draft")))
```

Python operator precedence: wrap each comparison in parentheses before combining with `&` or `|`. For example, `(field("price") <= 100) & (field("in_stock") == True)`. `field("price").lte(100) & field("in_stock") == True` parses as `(... & field("in_stock")) == True`, which is wrong.

### Count and get

```python
client.collection("articles").count()
client.collection("articles").get(["a1", "a2"], fields=["title"])  # {"a1": {...}, "a2": {...}}
```

`get()` without `fields` also returns internal `_embedding_*` fields for semantically indexed fields, which are large. Pass `fields=[...]`.

## Partitions (multi-tenancy)

```python
acme = client.collection("support", "acme")   # partition is created on first write
acme.upsert([...])
acme.query(select(...).sort(...).limit(5))   # only sees acme's documents

client.collection("support").delete_partition("globex")  # remove a tenant's data
[p.name for p in client.collection("support").list_partitions()]
```

A partition that was never written, or was deleted, raises `PartitionNotFoundError` on read. Treat that as an empty result:

```python
from topk_sdk.error import PartitionNotFoundError

def retrieve(tenant: str, question: str, k: int = 5) -> list[dict]:
    try:
        return client.collection("support", tenant).query(
            select("source", "body", score=fn.semantic_similarity("body", question))
            .sort(field("score"), asc=False)
            .limit(k)
        )
    except PartitionNotFoundError:
        return []
```

After `delete_partition`, writes to the same partition name are rejected for a while. Don't reuse the name right away.

## Read your own writes

```python
lsn = client.collection("articles").upsert(docs)
client.collection("articles").count(lsn=lsn)  # includes the write
client.collection("articles").query(q, lsn=lsn)
```

`consistency="strong"` is also accepted, but passing the LSN is the precise way to read a specific write.

## Async client

`from topk_sdk import AsyncClient` has the same API. Every call is awaitable, and `list_partitions()` is an async iterator.

## Errors

```python
from topk_sdk.error import (
    CollectionNotFoundError, CollectionAlreadyExistsError, DocumentValidationError,
    SchemaValidationError, PermissionDeniedError, RequestTooLargeError,
    QuotaExceededError, SlowDownError, QueryLsnTimeoutError,
)
```

A `DocumentValidationError` message lists every bad document, for example `TextTooLong { doc_id: "a", field: "body", max_length: 4096, got_length: 6545 }`.
