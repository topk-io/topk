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

```typescript
import { Client } from "topk-js";

const client = new Client({
  apiKey: process.env.TOPK_API_KEY,
  region: "aws-us-east-1-elastica",
});

// Create a dataset
await client.datasets().create("my-docs");

// Upload a file
const { handle } = await client.dataset("my-docs").upsertFile(
  "doc-1",
  { path: "/path/to/document.pdf" },
  { source: "internal" },
);

// Wait for the file to process (optional)
await client.dataset("my-docs").waitForHandle(handle);

// Ask a question
for await (const message of client.ask(
  "What was the total net income of Bank of America in 2024?",
  ["my-docs"],
)) {
  console.log(message);
}
```

## Handling errors

```typescript
import { DatasetNotFoundError, PermissionDeniedError, QuotaExceededError, SlowDownError } from "topk-js";

try {
  for await (const message of client.ask(
    "What was the total net income of Bank of America in 2024?",
    ["my-docs"],
  )) {
    console.log(message);
  }
} catch (err) {
  if (err instanceof DatasetNotFoundError) console.error("Dataset does not exist");
  else if (err instanceof PermissionDeniedError) console.error("Check your API key");
  else if (err instanceof QuotaExceededError) console.error("Usage quota exceeded");
  else if (err instanceof SlowDownError) console.error("Rate limited — the client will retry automatically");
}
```

| Error | Description |
| --- | --- |
| `CollectionNotFoundError` | Collection does not exist |
| `CollectionAlreadyExistsError` | Collection with this name already exists |
| `CollectionValidationError` | Invalid collection name or schema |
| `DatasetNotFoundError` | Dataset does not exist |
| `DatasetAlreadyExistsError` | Dataset with this name already exists |
| `DocumentValidationError` | Invalid document |
| `SchemaValidationError` | Invalid schema |
| `PermissionDeniedError` | Invalid or missing API key |
| `QuotaExceededError` | Usage quota exceeded |
| `RequestTooLargeError` | Request payload too large |
| `SlowDownError` | Rate limited by the server (retried automatically) |
| `QueryLsnTimeoutError` | Timed out waiting for write consistency |

### Retries

The client automatically retries on `SlowDownError` and on LSN consistency
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
