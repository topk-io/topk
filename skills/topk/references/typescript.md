# TopK TypeScript reference (`topk-js`)

## Contents
- Install and client
- Create a collection
- Write documents
- Query: semantic, keyword, hybrid, vector, filters
- Partitions (multi-tenancy)
- Read your own writes
- Errors

## Install and client

```bash
npm install topk-js
npm pkg set type=module   # needed for top-level await with tsx/node
```

```typescript
import { Client } from "topk-js";

const client = new Client({
  apiKey: process.env.TOPK_API_KEY!,
  region: process.env.TOPK_REGION!,
});
```

Import paths: `topk-js` (Client), `topk-js/schema`, `topk-js/query`, `topk-js/data`.

## Create a collection

```typescript
import { text, int, float, bool, keywordIndex, semanticIndex } from "topk-js/schema";

try {
  await client.collections().create("products", {
    name: text().required().index(semanticIndex()),        // semantic + BM25, max 4,096 chars
    description: text().index(semanticIndex()),
    category: text().required().index(keywordIndex()),
    price_cents: int().required(),  // see "Numbers" below
    in_stock: bool().required(),
  });
} catch (err) {
  if (!(err instanceof Error && err.message === "collection already exists")) throw err;
}
```

Other types: `f32Vector({ dimension })`, `f16Vector`, `f8Vector`, `u8Vector`, `i8Vector`, `binaryVector`, `f32SparseVector()`, `u8SparseVector()`, `matrix(...)`, `bytes()`, `timestamp()`, `list({ valueType })`, `struct({...})`.

Vector index: `f32Vector({ dimension: 1536 }).index(vectorIndex({ metric: "cosine" }))`.

One index per field: chaining `.index()` twice keeps only the last one.

### Numbers and empty lists

JavaScript has no separate integer type, so `topk-js` sends whole numbers like `99` as integers and fractional ones like `99.5` as floats. That causes two failures:

- A `float()` field rejects whole numbers: `InvalidDataType { expected_type: "float", got_value: "i64" }`.
- An untyped field that mixes both can be filtered, but sorting on it fails with `Sort expression must produce primitive or string type`.

For money and other values you sort on, store an integer: `price_cents: Math.round(price * 100)` in an `int()` field. Only use `float()` for values that are never whole numbers, such as scores.

An empty array `[]` is sent as a float list, so a `list({ valueType: "text" })` field rejects it. Wrap it instead: `import { stringList } from "topk-js/data"`, then `tags: stringList(tags)`.

## Write documents

```typescript
const lsn = await client.collection("products").upsert(
  products.map(({ id, ...rest }) => ({ _id: String(id), ...rest })), // _id must be a string
);
```

- To merge fields into existing documents: `update(docs)`. Pass `update(docs, true)` to fail on unknown ids.
- To delete by id: `delete(["p1", "p2"])`. To delete by filter: `delete(field("in_stock").eq(false))`.
- Batch large loads (8MB per request). Chunk text in `semanticIndex()` fields to 4,096 characters or less (see the Python reference for a chunking helper; the logic is identical).

## Query

```typescript
import { select, field, fn, match, should, not, all, any } from "topk-js/query";
```

`select` takes an object of named expressions. To return a stored field, use `field("name")`.

### Semantic search

```typescript
const rows = await client.collection("products").query(
  select({
    name: field("name"),
    score: fn.semanticSimilarity("description", "block noise while I work"),
  })
    .sort(field("score"), false) // false = descending
    .limit(5),
);
```

### Keyword search (BM25)

```typescript
select({ name: field("name"), score: fn.bm25Score() })
  .filter(match("espresso coffee", { field: "description" })) // required for bm25Score()
  .sort(field("score"), false)
  .limit(10);
```

`match(q, { field, weight, all })`: `all: true` requires every term. To boost without excluding documents, use `should(term, { field })`.

### Hybrid search, with a boost for words in a field

```typescript
const q = "something to block noise while I work";
const rel = field("name_sim").mul(0.4).add(field("desc_sim").mul(0.6));

const rows = await client.collection("products").query(
  select({
    name: field("name"),
    price_cents: field("price_cents"),
    name_sim: fn.semanticSimilarity("name", q),
    desc_sim: fn.semanticSimilarity("description", q),
  })
    .filter(field("in_stock").eq(true))
    .sort(rel.boost(field("name").matchAny(q), 1.5), false) // ×1.5 when the name shares a query word
    .limit(5),
);
```

Weight scores **inside `.sort()`**. A second `.select()` that does arithmetic on score aliases returns wrong values. `matchAny`/`matchAll` work on fields with a keyword or semantic index.

### Vector search

```typescript
select({ name: field("name"), score: fn.vectorDistance("embedding", queryVector) })
  .sort(field("score"), false) // cosine/dot_product: desc; euclidean: asc (true)
  .limit(10);
```

### Filters

Chain methods; there is no operator overloading:

```typescript
.filter(field("price_cents").lte(10000).and(field("in_stock").eq(true)))
.filter(field("category").in(["audio", "office"]))
.filter(field("name").startsWith("Wireless"))
.filter(field("tags").contains("sale"))
.filter(not(field("discontinued")))
.filter(all([field("a").gt(1), field("b").lt(5)]))
```

Calling `.filter()` more than once ANDs the conditions, which is handy for optional CLI flags:

```typescript
let q = select({...}).filter(field("in_stock").eq(true));
if (category) q = q.filter(field("category").eq(category));
if (maxPrice !== undefined) q = q.filter(field("price_cents").lte(Math.round(maxPrice * 100)));
const rows = await client.collection("products").query(q.sort(score, false).limit(5));
```

### Count and get

```typescript
await client.collection("products").count();
await client.collection("products").get(["p1", "p2"], ["name"]);
```

Without a field list, `get()` also returns large internal `_embedding_*` fields. Pass the fields you need.

## Partitions (multi-tenancy)

```typescript
const acme = client.collection("support", "acme"); // created on first write
await acme.upsert([...]);
await acme.query(q);                                // only acme's documents
await client.collection("support").deletePartition("globex");
```

Reading a partition that was never written, or was deleted, throws `"partition not found"`. Treat that as empty results. Don't reuse a deleted partition name right away.

## Read your own writes

```typescript
const lsn = await client.collection("products").upsert(docs);
await client.collection("products").count({ lsn });
await client.collection("products").query(q, { lsn });
```

## Errors

The SDK throws a plain `Error`. Match on `err.message`:

- Exact messages: `"collection not found"`, `"collection already exists"`, `"partition not found"`, `"permission denied"`.
- Prefix: `"request too large: ..."`.
- Messages containing `DocumentValidationError` (`TextTooLong`, `DocumentTooLarge`, `InvalidDataType`), `SchemaValidationError`, `QueryLsnTimeout`, `QuotaExceeded`, or `SlowDown`.
