import {
  binaryVector as binaryVectorData,
  i8Vector as i8VectorData,
  u8Vector as u8VectorData,
} from "../lib/data";
import { field, fn, select } from "../lib/query";
import {
  binaryVector,
  f32Vector,
  i8Vector,
  text,
  u8Vector,
  vectorIndex,
} from "../lib/schema";
import { isSorted, newProjectContext, ProjectContext } from "./setup";

describe("Vector Queries", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("query vector distance", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text(),
      summary_embedding: f32Vector({ dimension: 16 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "1984",
        title: "1984",
        summary_embedding: [1.0, ...Array(15).fill(0)],
      },
      {
        _id: "pride",
        title: "Pride and Prejudice",
        summary_embedding: [1.5, ...Array(15).fill(0)],
      },
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        summary_embedding: [2.0, ...Array(15).fill(0)],
      },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        title: field("title"),
        summary_distance: fn.vectorDistance("summary_embedding", [
          2.0,
          ...Array(15).fill(0),
        ]),
      })
        .sort(field("summary_distance"), true)
        .limit(3)
    );

    expect(isSorted(result, "summary_distance")).toBe(true);
    expect(result[0]).toHaveProperty("_id");
    expect(result[0]).toHaveProperty("title");
    expect(result[0]).toHaveProperty("summary_distance");
    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["1984", "pride", "mockingbird"])
    );
  });

  test("query vector distance nullable", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      nullable_embedding: f32Vector({ dimension: 16 }).index(
        vectorIndex({ metric: "euclidean" })
      ),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", nullable_embedding: [1.0, ...Array(15).fill(0)] },
      { _id: "mockingbird", nullable_embedding: [2.0, ...Array(15).fill(0)] },
      { _id: "catcher", nullable_embedding: null },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        summary_distance: fn.vectorDistance("nullable_embedding", [
          3.0,
          ...Array(15).fill(0),
        ]),
      })
        .sort(field("summary_distance"), true)
        .limit(3)
    );

    expect(isSorted(result, "summary_distance")).toBe(true);
    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["1984", "mockingbird"])
    );
  });

  test("query vector distance u8 vector", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      scalar_embedding: u8Vector({ dimension: 16 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "harry",
        scalar_embedding: u8VectorData([1, ...Array(15).fill(0)]),
      },
      {
        _id: "1984",
        scalar_embedding: u8VectorData([2, ...Array(15).fill(0)]),
      },
      {
        _id: "catcher",
        scalar_embedding: u8VectorData([3, ...Array(15).fill(0)]),
      },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        summary_distance: fn.vectorDistance(
          "scalar_embedding",
          u8VectorData([8, ...Array(15).fill(0)])
        ),
      })
        .sort(field("summary_distance"), true)
        .limit(3)
    );

    expect(isSorted(result, "summary_distance")).toBe(true);
    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["harry", "1984", "catcher"])
    );
  });

  test("query vector distance binary vector", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      binary_embedding: binaryVector({ dimension: 2 })
        .required()
        .index(vectorIndex({ metric: "hamming" })),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", binary_embedding: binaryVectorData([0, 0]) },
      { _id: "mockingbird", binary_embedding: binaryVectorData([0, 1]) },
      { _id: "catcher", binary_embedding: binaryVectorData([1, 0]) },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        summary_distance: fn.vectorDistance(
          "binary_embedding",
          binaryVectorData([0, 1])
        ),
      })
        .sort(field("summary_distance"), true)
        .limit(2)
    );

    expect(isSorted(result, "summary_distance")).toBe(true);
    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["1984", "mockingbird"])
    );
  });

  test("query vector distance i8 vector", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      scalar_i8_embedding: i8Vector({ dimension: 16 }).index(
        vectorIndex({ metric: "euclidean" })
      ),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "pride",
        scalar_i8_embedding: i8VectorData([-50, ...Array(15).fill(0)]),
      },
      {
        _id: "1984",
        scalar_i8_embedding: i8VectorData([0, ...Array(15).fill(0)]),
      },
      {
        _id: "gatsby",
        scalar_i8_embedding: i8VectorData([50, ...Array(15).fill(0)]),
      },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        summary_distance: fn.vectorDistance(
          "scalar_i8_embedding",
          i8VectorData([-10, ...Array(15).fill(0)])
        ),
      })
        .sort(field("summary_distance"), true)
        .limit(3)
    );

    expect(isSorted(result, "summary_distance")).toBe(true);
    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["1984", "pride", "gatsby"])
    );
  });
});
