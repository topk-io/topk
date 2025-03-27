import {
  Client,
  select,
  field,
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

  const res = await client
    .collection("books")
    .query(select([field("title")]).count());

  console.log(res);
}

main();
