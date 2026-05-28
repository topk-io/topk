# TopK JavaScript SDK

[![npm version](https://img.shields.io/npm/v/topk-js.svg)](https://www.npmjs.com/package/topk-js)

The TopK JavaScript library provides convenient access to the TopK API from Node.js environments with full TypeScript support.

## Documentation

The full documentation can be found at [docs.topk.io](https://docs.topk.io).

The JavaScript SDK reference can be found at [docs.topk.io/sdk/topk-js](https://docs.topk.io/sdk/topk-js).

## Installation

```sh
npm install topk-js
# or
yarn add topk-js
# or
pnpm install topk-js
```

## Prerequisites

- **API key** — sign in to [console.topk.io](https://console.topk.io) and generate an API key.
- **Region** — available regions are listed at [docs.topk.io/regions](https://docs.topk.io/regions).

## Usage

### Hybrid Search

```typescript
import { Client } from "topk-js";
import { text, keywordIndex, semanticIndex } from "topk-js/schema";
import { select, field, fn } from "topk-js/query";

const client = new Client({
  apiKey: process.env.TOPK_API_KEY!,
  region: "aws-us-east-1-elastica",
});

// Create a collection
await client.collections().create("books", {
  title: text().required().index(keywordIndex()),
  content: text().index(semanticIndex()),
});

// Upsert documents
await client.collection("books").upsert([
  {
    _id: "1",
    title: "Catcher in the Rye",
    content: "IF YOU REALLY WANT TO HEAR about it, the first thing you'll probably want to know is ...",
    author: "J.D. Salinger",
    rating: 3.8,
  },
  {
    _id: "2",
    title: "1984",
    content: "It was a bright cold day in April, and the clocks were striking thirteen. Winston Smith, ...",
    author: "George Orwell",
    rating: 4.7,
  },
]);

// Query with hybrid search
const results = await client.collection("books").query(
  select({
    title: field("title"),
    author: field("author"),
    // Compute semantic similarity of content field with the query
    similarity_score: fn.semanticSimilarity(
      "content",
      "What is the meaning of life?",
    ),
  })
  // Filter documents by metadata
  .filter(field("rating").gte(3.0))
  // Rank using the computed similarity score and rating
  .sort(field("rating").mul(field("similarity_score")), false)
  // Get top 10 highest ranked documents
  .limit(10)
);
```

### Document Search

```typescript
import { Client } from "topk-js";

const client = new Client({
  apiKey: process.env.TOPK_API_KEY,
  region: "aws-us-east-1-elastica",
});

// Create a dataset
await client.datasets().create("my-dataset");

// Upload a file
const handle = await client.dataset("my-dataset").upsertFile(
  "doc-1",                                   // document ID
  { path: "/path/to/document.pdf" },         // path to file
  { kind: "report", department: "finance" }, // optional metadata
);

// Wait for the file to process (optional)
await client.dataset("my-dataset").waitForHandle(handle);

// Ask a question
for await (const message of client.ask(
  "What was the total net income of Bank of America in 2024?",
  ["my-dataset"],
)) {
  console.log(message);
}
```

## Handling errors

The SDK throws plain `Error` objects. Check `err.message` to identify the error:

```typescript
try {
  for await (const message of client.ask(
    "What was the total net income of Bank of America in 2024?",
    ["my-dataset"],
  )) {
    console.log(message);
  }
} catch (err) {
  if (err instanceof Error) {
    if (err.message === "dataset not found") console.error("Dataset does not exist");
    else if (err.message === "permission denied") console.error("Check your API key");
    else console.error("Unexpected error:", err.message);
  }
}
```

| `err.message` | Description |
| --- | --- |
| `"collection not found"` | Collection does not exist |
| `"collection already exists"` | Collection with this name already exists |
| `"dataset not found"` | Dataset does not exist |
| `"dataset already exists"` | Dataset with this name already exists |
| `"permission denied"` | Invalid or missing API key |
| starts with `"request too large:"` | Request payload too large |

### Retries

The client automatically retries on slow-down and LSN consistency
timeouts. Retry behaviour can be configured via `retryConfig`:

```typescript
import { Client } from "topk-js";

const client = new Client({
  apiKey: process.env.TOPK_API_KEY,
  region: "aws-us-east-1-elastica",
  retryConfig: {
    maxRetries: 5,       // default: 3
    timeout: 60_000,     // total retry chain timeout in ms, default: 30,000
    backoff: {
      initBackoff: 200,  // default: 100 ms
      maxBackoff: 5_000, // default: 10,000 ms
    },
  },
});
```

## Requirements

Node.js 18 or higher.
