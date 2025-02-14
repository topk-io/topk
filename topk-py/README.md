# TopK SDK

**Full documentation is available on [docs.topk.io](https://docs.topk.io).**

TopK SDK provides a Python API for managing collections, inserting and deleting documents, and running queries. Our query language is designed
to be simple and expressive, allowing you to search using keywords, vectors, filters in a single query.

## Features

- Create and manage collections with custom schemas.
- Perform keyword and vector-based searches with scoring and ranking.
- Upsert and delete documents in collections.
- Support for schema validation and collection listing.
- Pythonic API for seamless integration.

## Installation

Install the SDK using `pip`:

```bash
pip install topk-sdk
```

## Usage Examples
Create a client with your API key and region. If you don't have an API key yet, you can get one for free
from our [console](https://console.topk.io).

```python
from topk_sdk import Client

client = Client(api_key="YOUR_TOPK_API_KEY", region="aws-us-east-1-elastica")
```

### 1. Create a Collection
First, you need to create a collection to store your documents. The example below creates a `books` collection with a
schema that includes a required `title` field with keyword index, a required `title_embedding` field with vector index,
and an optional `published_year` field with integer type.

```python
from topk_sdk.schema import text, int, f32_vector, vector_index, keyword_index

client.collections().create(
    "books",
    schema={
        # `title` field must be present and its value must be a string.
        "title": text()
          .required()
          .index(keyword_index()),

        # `title_embedding` field must be present and its value must be
        # a 1536-dimensional vector of floats.
        "title_embedding": f32_vector(1536)
            .required()
            .index(vector_index(metric="cosine")),

        # `published_year` is optional but if it is present, its value
        # must be an integer.
        "published_year": int()
    },
)
```

### 2. Upsert Documents
Now, you can upsert documents into the collection. `topk` will insert new documents and overwrite existing ones if they
have the same `_id`.

```python
import numpy as np

lsn = client.collection("books").upsert(
    [
        {
          "_id": "gatsby",
          "title": "The Great Gatsby",
          "title_embedding": np.random.rand(1536).tolist(),
          "published_year": 1925
        },
        {
          "_id": "1984",
          "title": "1984",
          "title_embedding": np.random.rand(1536).tolist(),
          "published_year": 1949
        },
        {
          "_id": "catcher",
          "title": "The Catcher in the Rye",
          "title_embedding": np.random.rand(1536).tolist(),
          "published_year": 1951
        }
    ],
)
```

### 3. Query Documents
Querying documents is what `topk` was built for. You can search documents using keywords, vectors and filters in a single
query. No need to manage multiple search systems and merge results from different queries.

```python
from topk_sdk.query import select, field, fn, match

results = client.collection("books").query(
    select(
      "title",
      vector_distance=fn.vector_distance("title_embedding", np.random.rand(1536).tolist()),
      text_score=fn.bm25_score(),
    )
    # Keyword search - filter books that contain keyword "catcher"
    .filter(match("catcher"))
    # Metadata search - filter books by published year
    .filter(field("published_year") > 1920)
    # Scoring - return top 10 results by calculating score as `vector_distance * 0.8 + text_score * 0.2`
    .top_k(field("vector_distance") * 0.8 + field("text_score") * 0.2, 10),

    # Pass the LSN to make sure the query is consistent with the writes.
    lsn=lsn,
)
```

### 4. Delete Documents
To delete documents from you collection, simply call the `delete` method and provide a list of `_id`s you want to delete.

```python
client.collection("books").delete(["1984"])
```

### 5. Delete a Collection
You can also delete the whole collection that will also delete all the documents in it.

```python
client.collections().delete("books")
```

## Testing

Run the test suite with `pytest`:

```bash
pytest
```
