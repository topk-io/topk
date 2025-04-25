import { f32Vector, text, vectorIndex } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Upsert", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("upsert to non-existent collection", async () => {
    const ctx = getContext();
    await expect(
      ctx.client.collection("missing").upsert([{ _id: "one" }])
    ).rejects.toThrow(/collection not found/);
  });

  test("upsert basic", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    const lsn = await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "one" }]);
    expect(lsn).toBe("1");
  });

  test("upsert batch", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    const lsn = await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "one" }, { _id: "two" }]);
    expect(lsn).toBe("1");
  });

  test("upsert sequential", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    let lsn = await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "one" }]);
    expect(lsn).toBe("1");

    lsn = await ctx.client.collection(collection.name).upsert([{ _id: "two" }]);
    expect(lsn).toBe("2");

    lsn = await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "three" }]);
    expect(lsn).toBe("3");
  });

  test("upsert with no documents", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    await expect(
      ctx.client.collection(collection.name).upsert([])
    ).rejects.toThrow(/NoDocuments/);
  });

  test("upsert with missing id field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    await expect(
      ctx.client.collection(collection.name).upsert([{}])
    ).rejects.toThrow(/MissingId { doc_offset: 0 }/);
  });

  test("upsert with missing name field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      name: text().required(),
    });

    await expect(
      ctx.client.collection(collection.name).upsert([{ _id: "one" }])
    ).rejects.toThrow(/MissingField { doc_id: \"one\", field: \"name\" }/);
  });

  test("upsert with invalid document - null field value", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
      title: text().required(),
      f32_embedding: f32Vector({ dimension: 3 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await expect(
      ctx.client
        .collection(ctx.scope("books"))
        .upsert([{ _id: "doc1", title: null, f32_embedding: [1, 2, 3] }])
    ).rejects.toThrow(
      /InvalidDataType { doc_id: \"doc1\", field: \"title\", expected_type: \"text\", got_value: \"null\" }/
    );
  });

  test("upsert with invalid document - missing required field", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
      title: text().required(),
      f32_embedding: f32Vector({ dimension: 3 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await expect(
      ctx.client
        .collection(ctx.scope("books"))
        .upsert([{ _id: "doc1", f32_embedding: [1, 2, 3] }])
    ).rejects.toThrow(/MissingField { doc_id: \"doc1\", field: \"title\" }/);
  });

  test("upsert with invalid document - wrong vector dimension", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
      title: text().required(),
      f32_embedding: f32Vector({ dimension: 3 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await expect(
      ctx.client
        .collection(ctx.scope("books"))
        .upsert([{ _id: "doc1", title: "one", f32_embedding: [1, 2] }])
    ).rejects.toThrow(
      /InvalidVectorDimension { doc_id: \"doc1\", field: \"f32_embedding\", expected_dimension: 3, got_dimension: 2 }/
    );
  });
});
