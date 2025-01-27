
# TopK SDK

**Full documentation is available on [docs.topk.io](https://docs.topk.io).**

TopK SDK provides a Python API for managing collections, querying data, and performing advanced search operations, including keyword and vector-based searches. It is designed for scalability and flexibility in building search applications.

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

Create a client:

```python
from topk_sdk import Client

client = Client(api_key="your_api_key", region="aws-us-east-1-thunderbird")
```

### 1. Create a Collection

Define a schema and create a collection.

```python
from topk_sdk.schema import text, vector, vector_index, keyword_index

schema = {
    "title": text().required().index(keyword_index()),
    "embedding": vector(3).required().index(vector_index(metric="cosine")),
}

client.collections().create("books", schema=schema)
```

### 2. Upsert Documents

Add or update documents in a collection.

```python
client.collection("books").upsert(
    [
        {"_id": "doc1", "title": "Hello World", "embedding": [1.0, 2.0, 3.0]},
        {"_id": "doc2", "title": "Rust Programming", "embedding": [4.0, 5.0, 6.0]},
    ]
)
```

### 3. Query for Keyword Search

Perform a keyword search with token matching and scoring.

```python
from topk_sdk.query import match, fn, select

results = client.collection("books").query(
    select(
        text_score=fn.keyword_score(),
    ).filter(
        match("title", token="Rust", weight=10.0)
    ).top_k("text_score", k=3)
)
```

### 4. Query for Vector Search

Perform a nearest-neighbor search with vector distances.

```python
results = client.collection("books").query(
    select(
        vector_distance=fn.vector_distance("embedding", [1.0, 2.0, 3.0]),
    ).top_k("vector_distance", k=3, asc=True)
)
```

### 5. Delete Documents

Remove documents from a collection.

```python
client.collection("books").delete(["doc1"])
```

### 6. List and Delete Collections

Manage collections in your workspace.

```python
# List collections
collections = client.collections().list()

# Delete a collection
client.collections().delete("books")
```

## Testing

Run the test suite with `pytest`:

```bash
pytest
```
