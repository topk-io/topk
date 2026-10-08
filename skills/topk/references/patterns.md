# TopK patterns

Proven designs for common tasks. Each one avoids mistakes that raise no error and so go unnoticed: truncated text, leaked tenant data, invented citations. Start from the closest pattern and adapt it.

## Contents
- 1. Search long documents (chunking, hybrid ranking, one result per file) — Python
- 2. Keep each customer's data separate (partitions, safe deletion) — Python
- 3. Give an agent a retrieval tool (passages + checked citations) — Python
- 4. Catalog search with filters and boosts — TypeScript

Every Python snippet assumes:

```python
import os
from topk_sdk import Client

client = Client(api_key=os.environ["TOPK_API_KEY"], region=os.environ["TOPK_REGION"])
```

## 1. Search long documents

**Use when** documents are longer than a paragraph (docs, articles, tickets, transcripts) and users type both exact terms and natural questions.

```python
from topk_sdk.error import CollectionAlreadyExistsError
from topk_sdk.query import field, fn, select, should
from topk_sdk.schema import semantic_index, text

COLLECTION = "docs"
CHUNK_CHARS = 3000  # semantic_index() values are capped at 4,096 characters
OVERLAP_CHARS = 200
KEYWORD_WEIGHT = 0.2  # BM25 ~0-15 vs semantic ~1-4 here; recalibrate on your data


def chunk(body: str) -> list[str]:
    """Split on paragraph boundaries into chunks of at most CHUNK_CHARS."""
    chunks, current = [], ""
    for para in body.split("\n\n"):
        while len(para) > CHUNK_CHARS:
            chunks.append(para[:CHUNK_CHARS])
            para = para[CHUNK_CHARS - OVERLAP_CHARS :]
        if current and len(current) + len(para) + 2 > CHUNK_CHARS:
            chunks.append(current)
            current = current[-OVERLAP_CHARS:]
        current = f"{current}\n\n{para}" if current else para
    if current.strip():
        chunks.append(current)
    return chunks


def index(files: dict[str, str]) -> str:
    """files = {source_name: full_text}. Returns the LSN of the last write."""
    try:
        client.collections().create(
            COLLECTION,
            schema={"source": text().required(), "text": text().required().index(semantic_index())},
        )
    except CollectionAlreadyExistsError:
        pass  # re-running is safe; never delete and recreate under the same name
    docs = [
        {"_id": f"{source}#{i}", "source": source, "text": part}
        for source, body in files.items()
        for i, part in enumerate(chunk(body))
    ]
    lsn = ""
    for start in range(0, len(docs), 200):  # stay under the 8MB request limit
        lsn = client.collection(COLLECTION).upsert(docs[start : start + 200])
    return lsn


def search(query: str, k: int = 5) -> list[dict]:
    """Hybrid search; returns the best chunk per source file."""
    rows = client.collection(COLLECTION).query(
        select("source", "text", semantic=fn.semantic_similarity("text", query), keyword=fn.bm25_score())
        .filter(should(query, field="text"))  # match() would drop chunks with no shared words
        .sort(field("semantic") + field("keyword") * KEYWORD_WEIGHT, asc=False)  # weight in sort()
        .limit(k * 4)  # over-fetch chunks, then keep one per file
    )
    best: dict[str, dict] = {}
    for r in rows:
        r["score"] = r["semantic"] + r["keyword"] * KEYWORD_WEIGHT
        if r["source"] not in best or r["score"] > best[r["source"]]["score"]:
            best[r["source"]] = r
    return sorted(best.values(), key=lambda r: r["score"], reverse=True)[:k]
```

**Why:** one `semantic_index()` field serves both semantic and BM25 scoring. Chunking keeps every part of a long file searchable; truncating to fit doesn't. Weighting in a second `select()` returns wrong values, so it happens in `sort()`.

## 2. Keep each customer's data separate

**Use when** a RAG app or agent serves many customers whose data must never mix, or you must delete one customer's data (offboarding, GDPR).

```python
import time

from topk_sdk.error import CollectionAlreadyExistsError, PartitionNotFoundError
from topk_sdk.query import field, fn, select
from topk_sdk.schema import semantic_index, text

COLLECTION = "support"


def index_customer(customer: str, passages: list[dict]) -> str:
    """passages = [{"_id", "source", "body"}], each body under 4,096 characters."""
    try:
        client.collections().create(
            COLLECTION,
            schema={"source": text().required(), "body": text().required().index(semantic_index())},
        )
    except CollectionAlreadyExistsError:
        pass
    # One partition per customer, created on first write. Queries never cross partitions.
    return client.collection(COLLECTION, customer).upsert(passages)


def retrieve(customer: str, question: str, k: int = 5) -> list[dict]:
    try:
        return client.collection(COLLECTION, customer).query(
            select("source", "body", score=fn.semantic_similarity("body", question))
            .sort(field("score"), asc=False)
            .limit(k)
        )
    except PartitionNotFoundError:  # never written, or deleted: not an empty list
        return []


def delete_customer(customer: str, timeout_s: int = 180) -> None:
    """Delete a customer's data and wait until it is no longer readable."""
    client.collection(COLLECTION).delete_partition(customer)
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:  # deleted data stays readable for about a minute
        try:
            client.collection(COLLECTION, customer).count()
        except PartitionNotFoundError:
            return
        time.sleep(5)
    raise TimeoutError(f"{customer!r} still readable after {timeout_s}s")
```

**Why:** a forgotten `customer_id` filter leaks data, while a partition can't. Report a deletion as done only after `delete_customer()` returns, and don't write to a deleted customer's partition name again right away; those writes are rejected for a while. Take `customer` from the authenticated session, never from model output.

## 3. Give an agent a retrieval tool

**Use when** an agent must answer from your data with sources. The shape works with any agent framework: the tool returns passages with their source, and your code checks that cited sources were actually retrieved.

Example with Claude, using the Anthropic SDK's tool runner (`pip install anthropic`) and `search()` from pattern 1:

```python
import json
import re

import anthropic
from anthropic import beta_tool

SYSTEM_PROMPT = (
    "Answer using the search_docs tool. Search again with different wording if the first "
    "results don't answer the question. Use only passages the tool returned, cite each "
    "claim's source in square brackets like [limits.md], and say so if the answer isn't there."
)


def ask(question: str) -> dict:
    retrieved: set[str] = set()

    @beta_tool
    def search_docs(query: str) -> str:
        """Search the documentation and return the most relevant passages with their source.

        Args:
            query: What to look for, e.g. "maximum document size".
        """
        results = [{"source": r["source"], "passage": r["text"]} for r in search(query)]
        retrieved.update(r["source"] for r in results)
        return json.dumps(results)

    runner = anthropic.Anthropic().beta.messages.tool_runner(
        model="claude-opus-5-5",
        max_tokens=16000,
        system=SYSTEM_PROMPT,
        tools=[search_docs],
        messages=[{"role": "user", "content": question}],
        output_config={"effort": "medium"},
        betas=["server-side-fallback-2026-07-01"],
        fallbacks="default",  # on a refusal, retry on Anthropic's recommended fallback model
    )
    final = runner.until_done()
    if final.stop_reason == "refusal":
        raise RuntimeError("The model declined to answer.")
    answer = "".join(b.text for b in final.content if b.type == "text")
    cited = set(re.findall(r"\[([^\]]+\.md)\]", answer))
    return {"answer": answer, "cited": sorted(cited), "unsupported": sorted(cited - retrieved)}
```

**Why:** the model can only answer from text it sees, so return passages, not ids. A model can cite a source it never retrieved, so treat a non-empty `unsupported` list as a failed answer. Keep each tool result small; five 3,000-character passages are about 4k tokens per search.

## 4. Catalog search with filters and boosts

**Use when** records have short text (name, description) plus structured fields to filter on (category, price, stock), and exact name matches should rank higher.

```typescript
import { Client } from "topk-js";
import { field, fn, select } from "topk-js/query";
import { bool, int, keywordIndex, semanticIndex, text } from "topk-js/schema";

const client = new Client({ apiKey: process.env.TOPK_API_KEY!, region: process.env.TOPK_REGION! });

export async function ensureCollection() {
  try {
    await client.collections().create("products", {
      name: text().required().index(semanticIndex()),
      description: text().required().index(semanticIndex()),
      category: text().required().index(keywordIndex()),
      // Integer cents: topk-js sends 59 as an integer, which a float() field rejects.
      price_cents: int().required(),
      in_stock: bool().required(),
    });
  } catch (err) {
    if (!(err instanceof Error && err.message === "collection already exists")) throw err;
  }
}

export async function search(query: string, opts: { category?: string; maxPrice?: number } = {}) {
  const relevance = field("name_sim").mul(0.4).add(field("desc_sim").mul(0.6));
  let q = select({
    name: field("name"),
    price_cents: field("price_cents"),
    name_sim: fn.semanticSimilarity("name", query),
    desc_sim: fn.semanticSimilarity("description", query),
  }).filter(field("in_stock").eq(true));
  if (opts.category) q = q.filter(field("category").eq(opts.category)); // filters are ANDed
  if (opts.maxPrice !== undefined) q = q.filter(field("price_cents").lte(Math.round(opts.maxPrice * 100)));
  // Boost ×1.5 when the name shares a query word; weight and boost inside sort().
  return client.collection("products").query(q.sort(relevance.boost(field("name").matchAny(query), 1.5), false).limit(5));
}
```

**Why:** integer cents avoid the whole-number rejection and sort correctly. `semanticIndex()` also supports `matchAny`, so the name field needs no second index. Semantic search always returns its nearest neighbours; add a minimum-score cutoff if off-topic queries should return nothing.
