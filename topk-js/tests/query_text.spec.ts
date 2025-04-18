import { field, match, select, bm25Score } from "../lib/query";
import { text, keywordIndex, int } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Text Queries", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("query text filter single term disjunctive", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "pride",
        summary: "A story about love and class",
        published_year: 1813,
      },
      {
        _id: "gatsby",
        summary: "A tale of love and wealth",
        published_year: 1925,
      },
      { _id: "lotr", summary: "A fantasy epic", published_year: 1954 },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({})
          .filter(match("love", "summary"))
          .topK(field("published_year"), 100, true)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["pride", "gatsby"])
    );
  });

  test("query text filter single term conjunctive", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "pride",
        summary: "A story about love and class",
        published_year: 1813,
      },
      {
        _id: "gatsby",
        summary: "A tale of love and wealth",
        published_year: 1925,
      },
      { _id: "lotr", summary: "A fantasy epic", published_year: 1954 },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({})
          .filter(match("love", "summary"))
          .topK(field("published_year"), 100, true)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["gatsby", "pride"])
    );
  });

  test("query text filter two terms disjunctive", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "pride",
        title: "Pride and Prejudice",
        summary: "A story about love and class",
        published_year: 1813,
      },
      {
        _id: "gatsby",
        title: "The Great Gatsby",
        summary: "A tale of lust and wealth",
        published_year: 1925,
      },
      {
        _id: "lotr",
        title: "The Lord of the Rings",
        summary: "A fantasy epic",
        published_year: 1954,
      },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({})
        .filter(match("LOVE", "summary").or(match("rings", "title")))
        .topK(field("published_year"), 100, true)
    );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["pride", "lotr"])
    );
  });

  test("query text filter two terms conjunctive", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "pride",
        summary: "A story about love and class",
        published_year: 1813,
      },
      {
        _id: "gatsby",
        summary: "A tale of love and wealth",
        published_year: 1925,
      },
      { _id: "lotr", summary: "A fantasy epic", published_year: 1954 },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({})
        .filter(match("LOVE", "summary").and(match("class", "summary")))
        .topK(field("published_year"), 100, true)
    );

    expect(new Set(result.map((doc) => doc._id))).toEqual(new Set(["pride"]));
  });

  test("query text filter stop word", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "pride",
        summary: "A story about love and class",
        published_year: 1813,
      },
      {
        _id: "gatsby",
        summary: "A tale of love and wealth",
        published_year: 1925,
      },
      { _id: "lotr", summary: "A fantasy epic", published_year: 1954 },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({})
          .filter(match("the", "summary"))
          .topK(field("published_year"), 100, true)
      );

    expect(result.length).toBe(0);
  });

  test("query select bm25 without text queries", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
    });

    await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "pride", summary: "A story about love and class" }]);

    await expect(
      ctx.client
        .collection(collection.name)
        .query(
          select({ bm25_score: bm25Score() })
            .filter(field("_id").eq("pride"))
            .topK(field("bm25_score"), 100, true)
        )
    ).rejects.toThrow(
      "invalid argument: Invalid query: Query must have at least one text filter to compute bm25 scores"
    );
  });
});
