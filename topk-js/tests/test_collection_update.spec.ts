import { field, fn, select } from "../lib/query";
import { f32Vector, int, keywordIndex, semanticIndex, text, vectorIndex } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Update", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("should throw error when updating non-existent collection", async () => {
    const ctx = getContext();
    await expect(
      ctx.client.collection("missing").update([{ _id: "one" }], false)
    ).rejects.toThrow(/collection not found/);
  });

  test("should update batch", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    let lsn = await ctx.client.collection(collection.name).upsert([
      { _id: "1", foo: "bar1" },
      { _id: "2", foo: "bar2" },
      { _id: "3", foo: "bar3" },
      { _id: "4", foo: "bar4" },
    ]);
    expect(lsn).toBe("1");

    lsn = await ctx.client.collection(collection.name).update(
      [
        { _id: "2", foo: "bar2.2", baz: "foo" },
        { _id: "3", foo: null },
        { _id: "4", foo: "bar4.2" },
        { _id: "5", foo: "bar5" }, // missing id
      ],
      false
    );
    expect(lsn).toBe("2");

    const docs = await ctx.client
      .collection(collection.name)
      .get(["1", "2", "3", "4", "5"], null, { lsn });

    expect(Object.keys(docs)).toHaveLength(4);
    expect(docs["1"]).toEqual({ _id: "1", foo: "bar1" });
    expect(docs["2"]).toEqual({ _id: "2", foo: "bar2.2", baz: "foo" });
    expect(docs["3"]).toEqual({ _id: "3" });
    expect(docs["4"]).toEqual({ _id: "4", foo: "bar4.2" });
  });

  test("should ignore missing id", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    // Upsert some docs
    let lsn = await ctx.client.collection(collection.name).upsert([
      { _id: "1", foo: "bar1" },
      { _id: "2", foo: "bar2" },
    ]);
    expect(lsn).toBe("1");

    // Update non-existent doc
    const newLsn = await ctx.client
      .collection(collection.name)
      .update([{ _id: "3", foo: "bar3" }], false);
    expect(newLsn).toBe("");

    // Check that no changes were made
    const docs = await ctx.client
      .collection(collection.name)
      .get(["1", "2", "3"], null, { lsn });

    expect(Object.keys(docs)).toHaveLength(2);
    expect(docs["1"]).toEqual({ _id: "1", foo: "bar1" });
    expect(docs["2"]).toEqual({ _id: "2", foo: "bar2" });
  });

  test("should throw error when updating missing id with fail_on_missing", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    // Upsert some docs
    await ctx.client.collection(collection.name).upsert([
      { _id: "1", foo: "bar1" },
      { _id: "2", foo: "bar2" },
    ]);

    // Update non-existent doc
    await expect(
      ctx.client
        .collection(collection.name)
        .update([{ _id: "3", foo: "bar3" }], true)
    ).rejects.toThrow(/DocumentNotFound/);
  });

  test("should update vector index field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary_embedding: f32Vector({ dimension: 16 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "1984",
        summary_embedding: Array(16).fill(2.0),
      },
    ]);

    let res = await ctx.client.collection(collection.name).query(
      select({
        dist: fn.vectorDistance("summary_embedding", Array(16).fill(2.0)),
      })
        .filter(field("_id").eq("1984"))
        .limit(1)
    );

    expect(res).toHaveLength(1);
    expect(res[0].dist).toBe(0.0);

    const lsn = await ctx.client.collection(collection.name).update(
      [{ _id: "1984", summary_embedding: Array(16).fill(8.0) }],
      true
    );

    res = await ctx.client.collection(collection.name).query(
      select({
        dist: fn.vectorDistance("summary_embedding", Array(16).fill(2.0)),
      })
        .filter(field("_id").eq("1984"))
        .limit(1),
      { lsn }
    );

    expect(res).toHaveLength(1);
    const expectedDist = Math.pow(6.0, 2) * 16.0;
    expect(res[0].dist).toBe(expectedDist);
  });

  test("should update semantic index field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("semantic", {
      title: text()
        .required()
        .index(semanticIndex({ model: "dummy" })),
      summary: text().required().index(semanticIndex({ model: "dummy" })),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "doc1",
        title: "Original Title",
        summary: "Original summary",
      },
      {
        _id: "doc2",
        title: "Another Title",
        summary: "Another summary",
      },
    ]);

    let result = await ctx.client.collection(collection.name).query(
      select({ sim: fn.semanticSimilarity("title", "dummy") }).topk(
        field("sim"),
        1,
        true
      )
    );

    expect(result).toHaveLength(1);
    const id = result[0]._id;
    const originalSim = result[0].sim;

    const lsn = await ctx.client.collection(collection.name).update(
      [{ _id: id, title: "foobarbaz" }],
      true
    );

    const updated = await ctx.client.collection(collection.name).query(
      select({
        title: field("title"),
        sim: fn.semanticSimilarity("title", "dummy"),
      })
        .filter(field("_id").eq(id))
        .limit(1),
      { lsn }
    );

    expect(updated).toHaveLength(1);
    expect(updated[0]._id).toBe(id);
    expect(updated[0].title).toBe("foobarbaz");
    expect(updated[0].sim).not.toBe(originalSim);
  });

  test("should throw error when updating with invalid data type", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "1984",
        title: "1984",
        published_year: 1949,
      },
    ]);

    await expect(
      ctx.client
        .collection(collection.name)
        .update([{ _id: "1984", title: 1984 }], true)
    ).rejects.toThrow(/InvalidDataType/);
  });

  test("should throw error when updating missing required field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "1984",
        title: "1984",
        published_year: 1949,
      },
    ]);

    await expect(
      ctx.client
        .collection(collection.name)
        .update([{ _id: "1984", title: null }], true)
    ).rejects.toThrow(/MissingField|required/);
  });
});
