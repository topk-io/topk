import {
  Client,
  select,
  field,
  match,
  bm25Score
} from "./index.js";

const client = new Client({
  apiKey: process.env.TOPK_API_KEY,
  region: "elastica",
});

async function main() {
  // const docs = await client.collection("books").upsert([
  //   {
  //     _id: "1",
  //     title: "The Great Gatsby",
  //     author: "F. Scott Fitzgerald",
  //     year: 1925,
  //   },
  //   {
  //     _id: "2",
  //     title: "To Kill a Mockingbird",
  //     author: "Harper Lee",
  //     year: 1960,
  //   },
  // ]);

  const docs = await client.collection("books").query(
    select({
        "title": field("title"),
        // Score documents using BM25 algorithm
        "text_score": bm25Score()
    })
    // Filter to documents that have the `great` keyword in the `title` field
    // or the `catcher` in any of the text-indexed fields.
    .filter(match("great", { field: "title", weight: 1 }).or(match("catcher")).or(match("Kill")))
    // Return top 10 documents with the highest text score
    .top_k(field("text_score"), 10)
  )

  console.dir(docs, { depth: null })
}

main();
