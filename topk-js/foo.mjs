import { Client } from "./index.js";

const client = new Client({
  apiKey: "n9adNtTTeDiGf7JDW71wSsHUJrPB1ciYFGg7zFnha5ker",
  region: "elastica",
});

async function main() {
  const collection = client.collection("testo");

  const upsert = await collection.upsert([
    {
      type: "String",
      name: "wow",
    },
  ]);

  console.log(upsert);

  const results = await collection.query({ stages: [] });

  console.dir(results, { depth: null });

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

  // const collections = await client.collections().list();

  // console.dir(collections, { depth: null });

  // await client.collections().delete("test");

  // const collections2 = await client.collections().list();

  // await client.collections().create({
  //   name: "esperanza",
  //   schema: {
  //     bom: {
  //       dataType: DataType.F32Vector,
  //       required: true,
  //       index: {
  //         type: "Vector",
  //         metric: 1,
  //       },
  //     },
  //     name: {
  //       dataType: DataType.Text,
  //       required: true,
  //     },
  //   },
  // });

  // console.dir(collections2, { depth: null });

  // await client.collection('esperanza').query({
  //   stages: [
  //     {
  //       type: 'Select',
  //       'exprs': {
  //         'bom': {
  //         }
  //       }
  //     },
  //   ],
  // });
}

main();

// const collections = await client.collections.list()
// console.log(collections);
