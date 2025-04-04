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

  const f = field("title")

  console.dir(f.expr, { depth: null })

  // console.dir(f.expr, { depth: null })
  // console.dir(f.expr.expr.expr, { depth: null })

  const sel = select({
    // "lions_tigers_bears_oh_myy": field("title"),
    "text_score": bm25Score()
  })

  console.dir(sel.stages, { depth: null })

  // console.dir(sel.get_stages(), { depth: null })

  // const filter = match("great", { field: "title", weight: 1 }).or(match("catcher"))

  // const topK = top_k(select, 10)

  // console.time('query');
  // const docs = await client.collection("books").query(
  //   select({
  //       "lions_tigers_bears_oh_myy": field("title"),
  //       // Score documents using BM25 algorithm
  //       "text_score": bm25Score()
  //   })
  //   // Filter to documents that have the `great` keyword in the `title` field
  //   // or the `catcher` in any of the text-indexed fields.
  //   .filter(match("great", { field: "title", weight: 1 }).or(match("catcher")))
  //   // Return top 10 documents with the highest text score
  //   .top_k(field("text_score"), 10)
  // );
  // console.timeEnd('query');

  // console.dir(docs, { depth: null })

  // const collections = await client.collections().list();
  // console.dir(collections, { depth: null });
}

main();
