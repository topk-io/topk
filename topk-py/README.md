<p align="center" style="padding: 40px 0;">
  <img src="../assets/topk-logo-light.svg#gh-light-mode-only">
  <img src="../assets/topk-logo-dark.svg#gh-dark-mode-only">
</p>

# TopK Python SDK

**Full documentation is available at [docs.topk.io](https://docs.topk.io).**

The **TopK SDK** provides a Python API for managing collections, inserting and deleting documents, and running powerful search queries. With **automatic embeddings** via `semantic_index()`, you can perform **semantic search without needing to manage vector embeddings manually**.

TopK’s **query language** is designed to be simple yet expressive, allowing you to search using **semantic similarity, keyword matching, and filters—all in a single query**.

## Features

- **Automatic embeddings**—no need for external vector models.
- **Create and manage collections** with custom schemas.
- **Perform hybrid search** using **semantic similarity, keyword search, and filters**.
- **Upsert and delete documents** within collections.
- **Automatic schema validation** and easy collection management.
- **Intuitive Python API** for seamless integration.

## 1. Install the SDK

Install the TopK SDK via **pip**:

```bash
pip install topk-sdk
```

## 2. Create an API Key

To authenticate with TopK, you'll need an **API key**:

1. Go to the <a href="https://console.topk.io" target="_blank">TopK Console</a>.
2. Create an account and generate your API key.

> **Keep your API key secure**—you'll need it for authentication.

## 3. Create a Collection

Collections are the primary data structure in TopK. They store documents and enable queries.

### **Initialize the Client**

```python
from topk_sdk import Client

client = Client(api_key="YOUR_TOPK_API_KEY", region="aws-us-east-1-elastica")
```

### **Define and Create a Collection**

```python
from topk_sdk.schema import text, semantic_index

client.collections().create(
  "books",
  schema={
    "title": text().required().index(semantic_index()),  # Semantic search enabled on title
  },
)
```

> **Note:** Other fields can still be upserted even if they are not defined in the schema.

## 4. Add Documents

Now, add documents to the collection. **Each document must have an `_id` field**.

```python
client.collection("books").upsert(
  [
    {"_id": "gatsby", "title": "The Great Gatsby"},
    {"_id": "1984", "title": "1984"},
    {"_id": "catcher", "title": "The Catcher in the Rye"}
  ],
)
```

## 5. Run a Search Query

Now, **retrieve books using semantic search**:

```python
from topk_sdk.query import select, fn

results = client.collection("books").query(
  select(
    "title",
    title_similarity=fn.semantic_similarity("title", "classic American novel"), # Semantic search
  )
  .topk(field("title_similarity"), 10) # Return top 10 most relevant results
)
```

## 6. (Optional) Delete a Collection

To remove the entire collection:

```
client.collections().delete("books")
```
