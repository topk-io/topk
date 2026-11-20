# TopK patterns

Proven designs for common tasks, using only the `topk` CLI and the bundled [`scripts/chunk.py`](../scripts/chunk.py). Each one avoids mistakes that raise no error and so go unnoticed: unsearchable text, leaked tenant data, stale chunks after re-indexing, scanned pages that are never found. Start from the closest pattern and adapt it.

## Contents
- 1. Search long documents (chunking, hybrid ranking, one result per file)
- 2. Keep each customer's data separate (partitions, safe deletion)
- 3. Catalog search with filters and boosts
- 4. Index PDF and Word files (page/section citations, scanned pages)

Every command assumes a logged-in CLI with a selected project and region (see [cli.md](cli.md)).

## 1. Search long documents

**Use when** documents are longer than a paragraph (docs, articles, tickets, transcripts) and users type both exact terms and natural questions.

Chunk the files, then import the chunks:

```bash
python scripts/chunk.py docs/ > chunks.jsonl          # {"_id", "source", "chunk_index", "location", "text"} per line
wc -l chunks.jsonl

F=source=keyword,text=semantic   # `source` can be filtered, `text` searched; other fields import unindexed
topk import ./chunks.jsonl --to docs -f $F --dry-run  # check the plan
topk import ./chunks.jsonl --to docs -f $F --yes
topk sql "SELECT COUNT(*) FROM docs"                  # must equal `wc -l chunks.jsonl`
```

Search with semantic similarity plus BM25. `should()` scores the keywords without dropping chunks that don't contain them, and the weight goes in `ORDER BY`:

```bash
Q='how do I filter by metadata'
topk sql -o json "SELECT source, location, text,
    semantic_similarity(text, '$Q') AS semantic, bm25_score() AS keyword
  FROM docs WHERE should('$Q', text)
  ORDER BY semantic + keyword * 0.2 DESC LIMIT 20" > hits.jsonl
```

Look at the real `semantic` and `keyword` values before settling on the `0.2`; it is a starting point, not a constant. If the user wants **one result per file**, over-fetch chunks (`LIMIT k * 4`) and keep each file's best chunk:

```bash
jq -s 'map(.score = .semantic + .keyword * 0.2)
       | group_by(.source) | map(max_by(.score))
       | sort_by(-.score) | .[:5][] | {source, location, score}' hits.jsonl
```

**Re-indexing changed files:** delete their old chunks first, then import the new ones into the same collection. Tell the user which files will be re-indexed before deleting (SKILL.md rule 11). A file that now produces fewer chunks would otherwise leave its old ones behind, still showing up in search.

```bash
python scripts/chunk.py docs/ > chunks.jsonl
SOURCES=$(jq -r '.source' chunks.jsonl | sort -u | sed "s/'/''/g; s/.*/'&'/" | paste -sd, -)
topk sql "DELETE FROM docs WHERE source IN ($SOURCES)"
topk import ./chunks.jsonl --to docs -f $F --yes
```

For hundreds of files, delete in batches of a few hundred sources. To rebuild everything with a different schema, import into a new name (`docs_v2`) instead.

**Why:** chunking keeps every part of a long file searchable; truncating to fit doesn't. One `semantic_index()` field serves both semantic and BM25 scoring. Escape `'` as `''` in queries you build from user text.

## 2. Keep each customer's data separate

**Use when** a support bot, RAG app or agent serves many customers whose data must never mix, or you must delete one customer's data (offboarding, GDPR).

Lay the files out as `tenants/<customer>/…` and import one partition per customer:

```bash
python scripts/chunk.py --partition-from-dir tenants/ > chunks.jsonl   # adds "tenant": "<customer>"
F=source=keyword,text=semantic
topk import ./chunks.jsonl --to support --partition tenant -f $F --dry-run
topk import ./chunks.jsonl --to support --partition tenant -f $F --yes
topk sql "SHOW PARTITIONS FROM TABLE support"
```

Files that need no chunking can be imported straight from the folders: `topk import 'tenants/{tenant}/*.jsonl' --to support --partition tenant -f $F --yes`.

Search one customer's partition. A quoted heredoc (`<<'SQL'`) keeps the shell from expanding `$acme`:

```bash
topk sql -o json -f - <<'SQL'
SELECT source, text, semantic_similarity(text, 'what is the max document size') AS score
FROM support$acme ORDER BY score DESC LIMIT 5
SQL
```

A partition that was never written, or was deleted, fails with `Partition does not exist`: that customer has no results, not an error to fix.

Delete a customer only after the user confirms which one, then wait until the data is really gone (it stays readable for about a minute):

```bash
topk sql 'DELETE FROM support$globex'
for i in $(seq 36); do
  topk sql 'SELECT COUNT(*) FROM support$globex' 2>&1 | grep -q 'Partition does not exist' && { echo deleted; break; }
  sleep 5
done
```

**Why:** a forgotten `tenant_id` filter leaks data, while a partition can't. Report a deletion as done only after the loop prints `deleted`, and don't write to a deleted customer's partition name again right away. Take the customer name from the authenticated user or the directory layout, never from model output.

## 3. Catalog search with filters and boosts

**Use when** records have short text (name, description) plus structured fields to filter on (category, price, stock), and exact name matches should rank higher.

Index both text fields semantically and the category as a keyword; `price` and `in_stock` import with their types and need no index to filter on:

```bash
F=name=semantic,description=semantic,category=keyword
topk import ./products.json --to products -f $F --dry-run   # check the types: price: float, in_stock: bool
topk import ./products.json --to products -f $F --yes
```

Comparisons accept whole numbers (`price <= 100`); writes need a float literal (SKILL.md rule 10).

Rank mostly by meaning, filter in `WHERE`, and boost products whose name shares a query word:

```bash
Q='shoes for running'
topk sql -o json "SELECT _id, name, price,
    semantic_similarity(name, '$Q') AS name_sim,
    semantic_similarity(description, '$Q') AS desc_sim
  FROM products
  WHERE in_stock = true AND category = 'outdoor' AND price <= 100.0
  ORDER BY boost(name_sim * 0.4 + desc_sim * 0.6, match_any(name, '$Q'), 1.5) DESC
  LIMIT 5"
```

**Why:** filters run inside the search, so `LIMIT 5` returns five in-stock matches rather than five matches of which some are filtered out later. `semantic_index()` also supports `match_any`, so the name needs no second index. Semantic search always returns its nearest neighbours; add a minimum-score cutoff if off-topic queries should return nothing.

## 4. Index PDF and Word files

**Use when** source documents are PDFs or `.docx` files. TopK indexes text, not files, so extract the text first. `scripts/chunk.py` does this, keeping a page number or heading on every chunk:

```bash
pip install "pypdf>=5" "python-docx>=1.1"
python scripts/chunk.py --root files files/ > chunks.jsonl
# stderr: handbook.pdf: no text in page 7 (scanned? run OCR)
```

Then import and search exactly as in pattern 1, selecting `location` so answers can cite "handbook.pdf, page 2" or "onboarding.docx, Expense limits".

**Why:**
- **Keep the location on every chunk.** Page numbers and headings are lost after chunking unless they're stored, and citations without them are hard to check.
- **Repeated headers and footers are stripped.** Otherwise they appear in every chunk, adding noise to every match. Only a page's first and last lines that contain letters are candidates, so numbers in tables are never removed.
- **Scanned pages have no text layer.** Extraction returns nothing, so the page would silently never be found. The script lists those pages; index them as page images (below), or run OCR on them (for example `ocrmypdf in.pdf out.pdf`) and chunk again.
- **Tables stay with their heading,** one row per line, so a question like "hotel limit per night" finds the row with its column names.
- **Complex layouts** (multi-column pages, tables spanning pages, forms, charts) extract poorly with `pypdf`. Index them as page images (below), or use a layout-aware parser such as Docling or Unstructured and write the same JSONL fields.

**Page images.** A `semantic` index on an image field embeds each page as a picture, so scans, charts and forms are searchable without OCR. `scripts/pdf_pages.py` renders every page to a JPEG under the 512KB image limit:

```bash
pip install "pypdfium2>=4" "pillow>=10"
python scripts/pdf_pages.py --root files files/ > pages.jsonl      # {"_id", "source", "page", "image"}
topk import ./pages.jsonl --to handbook_pages -f source=keyword,image=image:semantic --yes
topk sql -o json "SELECT source, page, semantic_similarity(image, 'hotel limit per night') AS score
  FROM handbook_pages ORDER BY score DESC LIMIT 5"
```

A page image can be cited by page but not quoted, and BM25 doesn't work on images (`match` fails with `Missing keyword index`). Keep text chunks for passages, exact terms and quotes; add page images for pages the text misses.
