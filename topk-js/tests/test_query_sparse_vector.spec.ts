import * as data from "../lib/data";
import { field, fn, select } from "../lib/query";
import { f32SparseVector, u8SparseVector, vectorIndex } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

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
      f32_sparse: f32SparseVector()
        .required()
        .index(vectorIndex({ metric: "dot_product" })),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", f32_sparse: { 0: 1.0, 1: 2.0, 2: 3.0 } },
      { _id: "pride", f32_sparse: { 0: 1.5, 1: 2.5, 2: 3.5 } },
      { _id: "mockingbird", f32_sparse: { 0: 0.5, 1: 1.5, 2: 2.5 } },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        score: fn.vectorDistance("f32_sparse", { 0: 1.0, 1: 2.0, 2: 3.0 }),
      })
        .sort(field("score"), false)
        .limit(3)
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
      sparse_u8: u8SparseVector()
        .required()
        .index(vectorIndex({ metric: "dot_product" })),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "moby", sparse_u8: data.u8SparseVector({ 0: 0, 1: 2, 2: 2 }) },
      { _id: "1984", sparse_u8: data.u8SparseVector({ 0: 1, 1: 2, 2: 3 }) },
      { _id: "pride", sparse_u8: data.u8SparseVector({ 0: 1, 1: 2, 2: 5 }) },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        score: fn.vectorDistance(
          "sparse_u8",
          data.u8SparseVector({ 0: 1, 1: 2, 2: 3 })
        ),
      })
        .sort(field("score"), false)
        .limit(3)
    );

    expect(result.map((doc) => doc._id)).toEqual(["pride", "1984", "moby"]);
  });

  test("query sparse vector distance nullable", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      sparse_u8: u8SparseVector().index(vectorIndex({ metric: "dot_product" })),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", sparse_u8: data.u8SparseVector({ 0: 1, 1: 2, 2: 3 }) },
      { _id: "moby", sparse_u8: data.u8SparseVector({ 0: 0, 1: 1, 2: 2 }) },
      { _id: "catcher", sparse_u8: data.u8SparseVector({ 0: 2, 1: 3, 2: 4 }) },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        distance: fn.vectorDistance(
          "sparse_u8",
          data.u8SparseVector({
            0: 1,
            1: 2,
            2: 3,
          })
        ),
      })
        .sort(field("distance"), false)
        .limit(3)
    );

    expect(result.map((doc) => doc._id)).toEqual(["catcher", "1984", "moby"]);

    // Get the moby document and set its sparse_u8 to null
    const moby = await ctx.client.collection(collection.name).get(["moby"]);
    moby["moby"]["sparse_u8"] = null;

    // Upsert the modified document
    const lsn = await ctx.client
      .collection(collection.name)
      .upsert([moby["moby"]]);

    // Query again with the LSN to ensure we get the updated data
    const result2 = await ctx.client.collection(collection.name).query(
      select({
        distance: fn.vectorDistance(
          "sparse_u8",
          data.u8SparseVector({
            0: 1,
            1: 2,
            2: 3,
          })
        ),
      })
        .sort(field("distance"), false)
        .limit(3),
      { lsn }
    );

    expect(result2.map((doc) => doc._id)).toEqual(["catcher", "1984"]);
  });
});
