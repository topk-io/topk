import { Client } from "./index.js";
import { select, field, literal } from "./lib/query.js";

const LOCO_API_KEY = "gBaWwJZWSn3JhHhwjGeQuxgzBSwgPBuQ5n8SPvFBXT9t5";
const LOCO_COLLECTION = "locos";

const client = new Client({
  apiKey: "3EeR9b3vxsHkMD15Xv7yTp75jCKC5eFJiKBW3LsRaWucs",
  // apiKey: LOCO_API_KEY,
  region: "dev",
  https: false,
  host: "ddb",
});


async function main() {
  const collections = await client.collections().list();
  console.log(collections);
  const numDocs = Math.floor(Math.random() * 20) + 1; // Random number between 1 and 20
  const docs = Array(numDocs).fill(null).map(() => ({
    _id: Math.random().toString(36).substring(2, 15),
    title: Math.random().toString(36).substring(2, 15)
  }));

  const lsn = await client.collection("kargo").upsert(docs);
  console.log("upsert lsn", lsn);

  const result = await client.collection("kargo").query(select({
    title: field("title"),
    number: literal(5),
  }).topk(field("number"), 10))

  console.log('what is result', result);

  console.log("how many docs", docs)
}

main();