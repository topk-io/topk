import { Client, DataType } from "./index.js";

const client = new Client({
  apiKey: "2JM9NZ5yNSWQc9dhzaD5wUDcrDageTetYnScELkSr6qqqS",
  region: "elastica",
});

async function main() {
  const newCollection = await client.collections().create({
    name: "test",
    schema: {
      name: {
        dataType: DataType.Text,
        required: true,
      },
    },
  });

  console.dir(newCollection, { depth: null });

  const collections = await client.collections().list();

  console.dir(collections, { depth: null });
}

main();

// const collections = await client.collections.list()
// console.log(collections);
