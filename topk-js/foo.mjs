import { Client, DataType, IndexType } from "./index.js";

const client = new Client({
  apiKey: "2JM9NZ5yNSWQc9dhzaD5wUDcrDageTetYnScELkSr6qqqS",
  region: "elastica",
});

async function main() {
  // const newCollection = await client.collections().create({
  //   name: "test",
  //   schema: {
  //     name: {
  //       dataType: DataType.Text,
  //       required: true,
  //     },
  //   },
  // });

  // console.dir(newCollection, { depth: null });

  const collections = await client.collections().list();

  console.dir(collections, { depth: null });

  await client.collections().delete("test");

  const collections2 = await client.collections().list();

  await client.collections().create({
    name: "esperanza",
    schema: {
      bom: {
        dataType: DataType.F32Vector,
        required: true,
        index: {
          type: "Vector",
          metric: 1,
        },
      },
    },
  });

  console.dir(collections2, { depth: null });
}

main();

// const collections = await client.collections.list()
// console.log(collections);
