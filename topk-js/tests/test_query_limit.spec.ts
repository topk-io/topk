import { field, fn, match, select } from "../lib/query";
import { f32Vector, int, keywordIndex, text, vectorIndex } from "../lib/schema";
import { newProjectContext, ProjectContext, docFields } from "./setup";

describe("test_query_limit", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("test_query_bare_limit", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test_query_bare_limit", {
      title: text().required().index(keywordIndex()),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "a", title: "A" },
      { _id: "b", title: "B" },
      { _id: "c", title: "C" },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(select({}).limit(100));

    expect(result.length).toBe(3);
    expect(new Set(result.map((d) => d._id))).toEqual(new Set(["a", "b", "c"]));
  });

  test("test_query_limit_select_filter", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test_query_limit_select_filter", {
      summary: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", summary: "dystopia", published_year: 1949 },
      { _id: "alchemist", summary: "journey", published_year: 1988 },
      { _id: "catcher", summary: "teen", published_year: 1951 },
      { _id: "gatsby", summary: "wealth", published_year: 1925 },
      { _id: "harry", summary: "magic", published_year: 1997 },
      { _id: "hobbit", summary: "adventure", published_year: 1937 },
      { _id: "pride", summary: "love", published_year: 1813 },
      { _id: "mockingbird", summary: "injustice", published_year: 1960 },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          _id: field("_id"),
          summary: field("summary"),
          is_recent: field("published_year").gt(1980),
        })
          .filter(field("_id").lte("hobbit"))
          .limit(3)
      );

    expect(result.length).toBe(3);
    expect(docFields(result)).toEqual(new Set(["_id", "summary", "is_recent"]));

    const expectedIds = new Set(["1984", "alchemist", "catcher", "gatsby", "harry", "hobbit"]);
    for (const doc of result) {
      expect(expectedIds.has(doc._id)).toBe(true);
    }
  });

  test("test_query_limit_with_bm25", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test_query_limit_with_bm25", {
      summary: text().required().index(keywordIndex()),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "moby", summary: "quest whale" },
      { _id: "hobbit", summary: "quest home" },
      { _id: "pride", summary: "love" },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(select({ bm25: fn.bm25Score() }).filter(match("quest")).limit(10));

    expect(result.length).toBe(2);
    expect(docFields(result)).toEqual(new Set(["_id", "bm25"]));
    expect(new Set(result.map((d) => d._id))).toEqual(new Set(["moby", "hobbit"]));
  });

  test("test_query_limit_vector_distance", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test_query_limit_vector_distance", {
      title: text(),
      summary_embedding: f32Vector({ dimension: 16 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", title: "1984", summary_embedding: Array(16).fill(2.0) },
      { _id: "hobbit", title: "The Hobbit", summary_embedding: Array(16).fill(7.0) },
    ]);

    // skip_refine is implicitly true
    const resultLimit = await ctx.client
      .collection(collection.name)
      .query(
        select({ title: field("title") })
          .select({ summary_distance: fn.vectorDistance("summary_embedding", Array(16).fill(2.0)) })
          .limit(100)
      );

    // explicitly set skip_refine to true
    const resultTopk = await ctx.client
      .collection(collection.name)
      .query(
        select({ title: field("title") })
          .select({ summary_distance: fn.vectorDistance("summary_embedding", Array(16).fill(2.0), { skipRefine: true }) })
          .sort(field("summary_distance"), true)
          .limit(100)
      );

    const byId = (arr: any[]) => new Map(arr.map((d) => [d._id, d]));
    const docsLimit = byId(resultLimit);
    const docsTopk = byId(resultTopk);

    expect(docFields(resultLimit)).toEqual(new Set(["_id", "title", "summary_distance"]));
    expect(docFields(resultTopk)).toEqual(new Set(["_id", "title", "summary_distance"]));
    expect(docsLimit.size).toBe(docsTopk.size);
    expect(docsLimit).toEqual(docsTopk);
  });

  test("test_query_invalid_collectors", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test_query_invalid_collectors", {
      title: text(),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "a", title: "A", published_year: 2000 },
    ]);

    const invalidQueries = [
      // topk + limit - multiple collectors
      select({ title: field("title") }).topk(field("published_year"), 100, true).limit(100),
      // limit + count - multiple collectors
      select({ title: field("title") }).limit(100).count(),
      // no collector
      select({ title: field("title") }).sort(field("published_year"), true),
      // multiple sorts
      select({ title: field("title") }).sort(field("published_year"), true).sort(field("published_year"), false),
      // topk + sort - effectively multiple sorts
      select({ title: field("title") }).topk(field("published_year"), 100, true).sort(field("published_year"), true),
    ];

    for (const q of invalidQueries) {
      await expect(ctx.client.collection(collection.name).query(q)).rejects.toThrow();
    }
  });
});
