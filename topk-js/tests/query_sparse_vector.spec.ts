import * as data from "../lib/data";
import { field, fn, select } from "../lib/query";
import {
  f32SparseVector,
  text,
  u8SparseVector,
  vectorIndex,
} from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

function isSorted(result: any[], fieldName: string): boolean {
  const values = result.map((doc) => doc[fieldName]);
  return values.every((value, i) => i === 0 || value >= values[i - 1]);
}

describe("Sparse Vector Queries", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("query sparse vector distance f32", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text(),
      sparse_f32_embedding: f32SparseVector()
        .required()
        .index(vectorIndex({ metric: "dot_product" })),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "1984",
        title: "1984",
        sparse_f32_embedding: { 0: 1.0, 1: 2.0, 2: 3.0 },
      },
      {
        _id: "pride",
        title: "Pride and Prejudice",
        sparse_f32_embedding: { 0: 1.5, 1: 2.5, 2: 3.5 },
      },
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        sparse_f32_embedding: { 0: 0.5, 1: 1.5, 2: 2.5 },
      },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        title: field("title"),
        score: fn.vectorDistance("sparse_f32_embedding", {
          0: 1.0,
          1: 2.0,
          2: 3.0,
        }),
      }).topk(field("score"), 3, false)
    );

    expect(result.map((doc) => doc._id)).toEqual([
      "pride",
      "1984",
      "mockingbird",
    ]);
  });

  test("query sparse vector distance u8 vector", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      sparse_u8_embedding: u8SparseVector()
        .required()
        .index(vectorIndex({ metric: "dot_product" })),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "1984",
        sparse_u8_embedding: data.u8SparseVector({ 0: 1, 1: 2, 2: 3 }),
      },
      {
        _id: "pride",
        sparse_u8_embedding: data.u8SparseVector({ 0: 1, 1: 2, 2: 4 }),
      },
      {
        _id: "mockingbird",
        sparse_u8_embedding: data.u8SparseVector({ 0: 0, 1: 1, 2: 2 }),
      },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        score: fn.vectorDistance(
          "sparse_u8_embedding",
          data.u8SparseVector({ 0: 1, 1: 2, 2: 3 })
        ),
      }).topk(field("score"), 3, false)
    );

    expect(result.map((doc) => doc._id)).toEqual([
      "pride",
      "1984",
      "mockingbird",
    ]);
  });

  test("query sparse vector distance nullable", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      sparse_u8_embedding: u8SparseVector().index(
        vectorIndex({ metric: "dot_product" })
      ),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "1984",
        sparse_u8_embedding: data.u8SparseVector({ 0: 1, 1: 2, 2: 3 }),
      },
      {
        _id: "mockingbird",
        sparse_u8_embedding: data.u8SparseVector({ 0: 0, 1: 1, 2: 2 }),
      },
      {
        _id: "catcher",
        sparse_u8_embedding: data.u8SparseVector({ 0: 2, 1: 3, 2: 4 }),
      },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        title: field("title"),
        sparse_u8_distance: fn.vectorDistance(
          "sparse_u8_embedding",
          data.u8SparseVector({
            0: 1,
            1: 2,
            2: 3,
          })
        ),
      }).topk(field("sparse_u8_distance"), 3, false)
    );

    expect(result.map((doc) => doc._id)).toEqual([
      "catcher",
      "1984",
      "mockingbird",
    ]);

    // Get the mockingbird document and set its sparse_u8_embedding to null
    const mockingbird = await ctx.client
      .collection(collection.name)
      .get(["mockingbird"]);
    mockingbird["mockingbird"]["sparse_u8_embedding"] = null;

    // Upsert the modified document
    const lsn = await ctx.client
      .collection(collection.name)
      .upsert([mockingbird["mockingbird"]]);

    // Query again with the LSN to ensure we get the updated data
    const result2 = await ctx.client.collection(collection.name).query(
      select({
        title: field("title"),
        sparse_u8_distance: fn.vectorDistance(
          "sparse_u8_embedding",
          data.u8SparseVector({
            0: 1,
            1: 2,
            2: 3,
          })
        ),
      }).topk(field("sparse_u8_distance"), 3, false),
      { lsn }
    );

    expect(result2.map((doc) => doc._id)).toEqual(["catcher", "1984"]);
  });
});
