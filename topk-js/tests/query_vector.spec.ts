import { vectorDistance, field, select } from "../lib/query";
import {
  f32Vector,
  u8Vector,
  binaryVector,
  vectorIndex,
  text,
} from "../lib/schema";
import {
  u8Vector as u8VectorData,
  binaryVector as binaryVectorData,
} from "../lib/data";
import { newProjectContext, ProjectContext } from "./setup";

function isSorted(result: any[], fieldName: string): boolean {
  const values = result.map((doc) => doc[fieldName]);
  return values.every((value, i) => i === 0 || value >= values[i - 1]);
}

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
      summary_embedding: f32Vector(16)
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
        summary_distance: vectorDistance("summary_embedding", [
          2.0,
          ...Array(15).fill(0),
        ]),
      }).topk(field("summary_distance"), 3, true)
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
      nullable_embedding: f32Vector(16).index(
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
        summary_distance: vectorDistance("nullable_embedding", [
          3.0,
          ...Array(15).fill(0),
        ]),
      }).topk(field("summary_distance"), 3, true)
    );

    expect(isSorted(result, "summary_distance")).toBe(true);
    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["1984", "mockingbird"])
    );
  });

  test("query vector distance u8 vector", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      scalar_embedding: u8Vector(16)
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
        summary_distance: vectorDistance(
          "scalar_embedding",
          u8VectorData([8, ...Array(15).fill(0)])
        ),
      }).topk(field("summary_distance"), 3, true)
    );

    expect(isSorted(result, "summary_distance")).toBe(true);
    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["harry", "1984", "catcher"])
    );
  });

  test("query vector distance binary vector", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      binary_embedding: binaryVector(2)
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
        summary_distance: vectorDistance(
          "binary_embedding",
          binaryVectorData([0, 1])
        ),
      }).topk(field("summary_distance"), 2, true)
    );

    expect(isSorted(result, "summary_distance")).toBe(true);
    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["1984", "mockingbird"])
    );
  });
});
