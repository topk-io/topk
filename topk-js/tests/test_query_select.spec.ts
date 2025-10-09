import { bytes, f64List, i32List, i64List, u32List, u8Vector } from "../lib/data";
import { field, fn, literal, match, select } from "../lib/query";
import { f32Vector, int, keywordIndex, text, vectorIndex } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Select Queries", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("query select literal", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", title: "1984", published_year: 1949 },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813 },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({ literal: literal(1.0) })
        .filter(field("title").eq("1984"))
        .sort(field("published_year"), true)
        .limit(100)
    );

    expect(results).toEqual([{ _id: "1984", literal: 1.0 }]);
  });

  test("query select non-existing field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "1984", title: "1984", published_year: 1949 }]);

    const results = await ctx.client.collection(collection.name).query(
      select({ literal: field("non_existing_field") })
        .filter(field("title").eq("1984"))
        .sort(field("published_year"), true)
        .limit(100)
    );

    expect(results).toEqual([{ _id: "1984" }]);
  });

  test("query topk limit", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", published_year: 1813 },
      { _id: "moby", published_year: 1851 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "1984", published_year: 1949 },
    ]);

    let results = await ctx.client
      .collection(collection.name)
      .query(select({}).topk(field("published_year"), 3, true));
    expect(results.length).toBe(3);

    results = await ctx.client
      .collection(collection.name)
      .query(select({}).topk(field("published_year"), 2, true));
    expect(results.length).toBe(2);

    results = await ctx.client
      .collection(collection.name)
      .query(select({}).topk(field("published_year"), 1, true));
    expect(results.length).toBe(1);
  });

  test("query topk asc", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", published_year: 1813 },
      { _id: "moby", published_year: 1851 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "1984", published_year: 1949 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({ published_year: field("published_year") }).topk(
          field("published_year"),
          3,
          true
        )
      );

    expect(results).toEqual([
      { _id: "pride", published_year: 1813 },
      { _id: "moby", published_year: 1851 },
      { _id: "gatsby", published_year: 1925 },
    ]);
  });

  test("query topk desc", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "harry", published_year: 1997 },
      { _id: "alchemist", published_year: 1988 },
      { _id: "mockingbird", published_year: 1960 },
      { _id: "1984", published_year: 1949 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({ published_year: field("published_year") }).topk(
          field("published_year"),
          3,
          false
        )
      );

    expect(results).toEqual([
      { _id: "harry", published_year: 1997 },
      { _id: "alchemist", published_year: 1988 },
      { _id: "mockingbird", published_year: 1960 },
    ]);
  });

  test("query sort limit k", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", published_year: 1813 },
      { _id: "moby", published_year: 1851 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "1984", published_year: 1949 },
    ]);

    let results = await ctx.client
      .collection(collection.name)
      .query(select({}).sort(field("published_year"), true).limit(3));
    expect(results.length).toBe(3);

    results = await ctx.client
      .collection(collection.name)
      .query(select({}).sort(field("published_year"), true).limit(2));
    expect(results.length).toBe(2);

    results = await ctx.client
      .collection(collection.name)
      .query(select({}).sort(field("published_year"), true).limit(1));
    expect(results.length).toBe(1);
  });

  test("query sort limit asc", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", published_year: 1813 },
      { _id: "moby", published_year: 1851 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "1984", published_year: 1949 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({ published_year: field("published_year") })
          .sort(field("published_year"), true)
          .limit(3)
      );

    expect(results).toEqual([
      { _id: "pride", published_year: 1813 },
      { _id: "moby", published_year: 1851 },
      { _id: "gatsby", published_year: 1925 },
    ]);
  });

  test("query sort limit desc", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "harry", published_year: 1997 },
      { _id: "alchemist", published_year: 1988 },
      { _id: "mockingbird", published_year: 1960 },
      { _id: "1984", published_year: 1949 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({ published_year: field("published_year") })
          .sort(field("published_year"), false)
          .limit(3)
      );

    expect(results).toEqual([
      { _id: "harry", published_year: 1997 },
      { _id: "alchemist", published_year: 1988 },
      { _id: "mockingbird", published_year: 1960 },
    ]);
  });

  test("query select bm25 score", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", title: "Pride and Prejudice" },
      { _id: "1984", title: "1984" },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({ bm25_score: fn.bm25Score() })
          .filter(match("pride"))
          .sort(field("bm25_score"), true)
          .limit(100)
      );

    expect(new Set(results.map((doc) => doc._id))).toEqual(new Set(["pride"]));
  });

  test("query select vector distance", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary_embedding: f32Vector({ dimension: 16 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", summary_embedding: [1.0, ...Array(15).fill(0)] },
      { _id: "mockingbird", summary_embedding: [1.5, ...Array(15).fill(0)] },
      { _id: "pride", summary_embedding: [2.0, ...Array(15).fill(0)] },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        summary_distance: fn.vectorDistance("summary_embedding", [
          2.0,
          ...Array(15).fill(0),
        ]),
      })
        .sort(field("summary_distance"), true)
        .limit(3)
    );

    expect(new Set(results.map((doc) => doc._id))).toEqual(
      new Set(["1984", "mockingbird", "pride"])
    );
  });

  test("query select null field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "1984", a: null }, { _id: "pride" }]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({ a: field("a"), b: literal(1) })
          .sort(field("b"), true)
          .limit(100)
      );

    // Assert that `a` is null for all documents, even when not specified when upserting
    expect(new Set(results.map((doc) => doc.a))).toEqual(new Set([null, null]));
  });

  test("query select text match", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      summary: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "1984",
        title: "1984",
        summary:
          "A totalitarian regime uses surveillance and mind control to oppress its citizens.",
        published_year: 1949,
      },
      {
        _id: "pride",
        title: "Pride and Prejudice",
        summary:
          "A witty exploration of love, social class, and marriage in 19th-century England.",
        published_year: 1813,
      },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        match_surveillance: field("summary").matchAll(
          "surveillance control mind"
        ),
        match_love: field("summary").matchAny("love class marriage"),
      })
        .filter(field("title").eq("1984").or(field("_id").eq("pride")))
        .sort(field("published_year"), true)
        .limit(100)
    );

    // Sort results by _id to match the Rust test behavior
    results.sort((a, b) => a._id.localeCompare(b._id));

    expect(results).toEqual([
      { _id: "1984", match_surveillance: true, match_love: false },
      { _id: "pride", match_surveillance: false, match_love: true },
    ]);
  });

  test("query select union", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {});

    const lsn = await ctx.client.collection(collection.name).upsert([
      { _id: "0", rank: 0, mixed: null },
      { _id: "1", rank: 1, mixed: 1 },
      { _id: "2", rank: 2, mixed: 2 },
      { _id: "3", rank: 3, mixed: 3 },
      { _id: "4", rank: 4, mixed: 4 },
      { _id: "5", rank: 5, mixed: 5 },
      { _id: "6", rank: 6, mixed: 6 },
      { _id: "7", rank: 7, mixed: true },
      { _id: "8", rank: 8, mixed: "hello" },
      { _id: "9", rank: 9, mixed: u8Vector([1, 2, 3]) },
      { _id: "10", rank: 10, mixed: [1.0, 2.0, 3.0] },
      { _id: "11", rank: 11, mixed: bytes([1, 2, 3]) },
      { _id: "12", rank: 12, mixed: u32List([17, 6, 1997]) },
      { _id: "13", rank: 13, mixed: i32List([17, 6, 1997]) },
      { _id: "14", rank: 14, mixed: i64List([17, 6, 1997]) },
      { _id: "15", rank: 15, mixed: f64List([17.5, 6.5, 1997.5]) },
      { _id: "16", rank: 16, mixed: ["foo", "bar"] },
    ]);

    const _ = await ctx.client.collection(collection.name).count({ lsn });

    const results = await ctx.client
      .collection(collection.name)
      .query(select({ mixed: field("mixed") })
        .sort(field("rank"), true)
        .limit(100));

    expect(results).toEqual([
      { _id: "0", mixed: null },
      { _id: "1", mixed: 1 },
      { _id: "2", mixed: 2 },
      { _id: "3", mixed: 3 },
      { _id: "4", mixed: 4 },
      { _id: "5", mixed: 5.0 },
      { _id: "6", mixed: 6.0 },
      { _id: "7", mixed: true },
      { _id: "8", mixed: "hello" },
      { _id: "9", mixed: [1, 2, 3] },
      { _id: "10", mixed: [1.0, 2.0, 3.0] },
      { _id: "11", mixed: Buffer.from([1, 2, 3]) },
      { _id: "12", mixed: [17, 6, 1997] },
      { _id: "13", mixed: [17, 6, 1997] },
      { _id: "14", mixed: [17, 6, 1997] },
      { _id: "15", mixed: [17.5, 6.5, 1997.5] },
      { _id: "16", mixed: ["foo", "bar"] },
    ]);
  });

  test("query select list", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {});

    const lsn = await ctx.client.collection(collection.name).upsert([
      { _id: "0", rank: 0, list: null },
      { _id: "1", rank: 1, list: ["foo", "bar"] },
      { _id: "2", rank: 2 },
      { _id: "3", rank: 3, list: [] },
      { _id: "4", rank: 4, list: ["baz"] },
    ]);

    const _ = await ctx.client.collection(collection.name).count({ lsn });

    const results = await ctx.client
      .collection(collection.name)
      .query(select({ list: field("list") })
        .sort(field("rank"), true)
        .limit(100));

    expect(results).toEqual([
      { _id: "0", list: null },
      { _id: "1", list: ["foo", "bar"] },
      { _id: "2", list: null },
      { _id: "3", list: [] },
      { _id: "4", list: ["baz"] },
    ]);
  });
});
