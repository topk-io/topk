#!/usr/bin/env node
const fs = require("fs");
const path = require("path");

const TYPES_FILE = path.join(__dirname, "..", "index.d.ts");

const EXTRA_TYPES = `
/** A source for ask/search operations. Can be a string (dataset name) or an object with filter. */
export interface Source {
  dataset: string
  filter?: query.LogicalExpression
}

/** A search result from context search or ask references. */
export interface ContextSearchResult {
  docId: string
  docType: string
  dataset: string
  content: Content
  metadata: Record<string, any>
}

/** An answer from the ask API. */
export interface Answer {
  facts: Array<Fact>
  refs: Record<string, ContextSearchResult>
}

/** A search step from the ask API. */
export interface SearchStep {
  objective: string
  facts: Array<Fact>
  refs: Record<string, ContextSearchResult>
}

/** Result from the ask API. Check \`type\` to determine which field is set. */
export interface AskResult {
  type: 'answer' | 'search' | 'reason'
  answer?: Answer
  search?: SearchStep
  reason?: Reason
}

/** An entry in a dataset listing. */
export interface ListEntry {
  id: string
  name: string
  size: number
  mimeType: string
  metadata: Record<string, any>
}
`;

let content = fs.readFileSync(TYPES_FILE, "utf8");

if (!content.includes("export interface AskResult")) {
  content = content.replace(
    "/* eslint-disable */",
    "/* eslint-disable */" + EXTRA_TYPES
  );
  fs.writeFileSync(TYPES_FILE, content);
  console.log("Patched index.d.ts with context API types");
} else {
  console.log("index.d.ts already contains context API types");
}
