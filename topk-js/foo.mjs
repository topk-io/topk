import {
  Client,
  u8Vector,
  f32Vector
} from "./index.js";

import { schema, query } from "./index.js";

const client = new Client({
  apiKey: process.env.TOPK_API_KEY,
  region: "elastica",
});

async function main() {
  // const vec = u8Vector([1, 2, 3])
  // console.dir(vec, { depth: null })

  const vec = f32Vector([7.0, 8.0, 9.0])

  const heyo = query.vectorDistance("f32_embedding", vec)

  const dis = query.select({
    dis: heyo
  })

  // console.dir(dis, { depth: null })
  // const m = match('el mambo').and(match('el mambooo'))

  // console.dir(m.expr, { depth: null })

  // const schema = {
  //   title: bool()
  // }

  // console.dir(schema, { depth: null })

  // const collection = await client.collections().create("bookz", schema)

  // console.dir(collection, { depth: null })

  // const docs = await collection.query(
  //   select({
  //     "text_score": bm25Score()
  //   })
  //   .filter(match('el mambo'))
  //   .count()
  // )

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
