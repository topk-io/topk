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
  const f = field("title")

  console.dir(f.expr, { depth: null })

  const sel = select({
    "lions_tigers_bears_oh_myy": field("title"),
    "text_score": bm25Score()
  })

  field("title")

  console.dir(sel.stages, { depth: null })

  console.dir(sel.stages[0].exprs['lions_tigers_bears_oh_myy'].expr, { depth: null })
}

main();
