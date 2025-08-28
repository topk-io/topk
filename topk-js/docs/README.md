**topk-js**

***

<p align="center" style="padding: 40px 0;">
  <img src="_media/topk-logo-light.svg#gh-light-mode-only">
  <img src="_media/topk-logo-dark.svg#gh-dark-mode-only">
</p>

# TopK Javacript SDK

**Full documentation is available at [docs.topk.io](https://docs.topk.io).**

The TopK SDK is a fully TypeScript-compatible JavaScript client that brings together keyword and vector search, semantic search with automatic or user-defined embeddings, advanced metadata filtering, and built-in reranking.

## Features

- Keyword and vector search
- Semantic search out of the box
- Filtering
- Reranking
- Collection and document management

## 1. Install the SDK

Install the TopK SDK:

```bash
npm install topk-js

# or

yarn add topk-js
```

## 2. Create an API Key

To authenticate with TopK, you'll need an **API key**:

1. Go to the <a href="https://console.topk.io" target="_blank">TopK Console</a>.
2. Create an account and generate your API key.

> **Keep your API key secure**—you'll need it for authentication.

## 3. Create a Collection

Collections are the primary data structure in TopK. They store documents and enable queries.

### **Initialize the Client**

```javascript
import { Client } from "topk-js"

const client = new Client({
  apiKey: "YOUR_TOPK_API_KEY",
  region: "aws-us-east-1-elastica"
});
```

### **Define and Create a Collection**

```javascript
import { text, semanticIndex } from "topk-js/schema"

await client.collections().create("books", {
  title: text().required().index(semanticIndex()), // Semantic search enabled on `title`
});
```

> **Note:** Other fields can still be upserted even if they are not defined in the schema.

## 4. Add Documents

Now, add documents to the collection. **Each document must have an `_id` field**.

```javascript
await client.collection("books").upsert([
  { _id: "gatsby", title: "The Great Gatsby" },
  { _id: "1984", title: "1984" },
  { _id: "catcher", title: "The Catcher in the Rye" },
]);
```

## 5. Run a Search Query

Now, **retrieve books using semantic search**:

```javascript
import { select, fn, match, field } from "topk-js/query"

const results = await client.collection("books").query(
  select({
    title: field("title"),
    // Perform semantic search on the `title` field
    title_similarity: fn.semanticSimilarity(
      "title",
      "classic American novel"
    ),
  })
    // Sort results by the `title_similarity` field, selecting the top 10 results
    .topk(field("title_similarity"), 10)
);
```

## 6. (Optional) Delete a Collection

To remove the entire collection:

```javascript
await client.collections().delete("books")
```
