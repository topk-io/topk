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
  // const m = match('el mambo').and(match('el mambooo'))

  // console.dir(m.expr, { depth: null })

  const collection = client.collection("books")

  const docs = await collection.query(
    select({
      "text_score": bm25Score()
    })
    .filter(match('el mambo'))
    .count()
  )

  // console.dir(docs, { depth: null })

  // const f = field("title")

  // console.dir(f.expr, { depth: null })

  // const sel = select({
  //   "lions_tigers_bears_oh_myy": field("title").gt(100).and(field("title").lt(1000)).or(field("title").eq("foo")),
  //   "text_score": bm25Score()
  // })

  // field("title").gt()

  // console.dir(sel.stages, { depth: null })

  // console.dir(sel.stages[0].exprs['lions_tigers_bears_oh_myy'].expr, { depth: null })
  // console.dir(sel.stages[0].exprs['text_score'], { depth: null })
}

main();
