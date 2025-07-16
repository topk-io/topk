import { field, fn, match, select, filter } from "../lib/query";
import { int, keywordIndex, text } from "../lib/schema";
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

    const result = await ctx.client.collection(collection.name).query(
      filter(match("love", { field: "summary" }))
        .topk(field("published_year"), 100)
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

    const result = await ctx.client.collection(collection.name).query(
      filter(match("love", { field: "summary" }))
        .topk(field("published_year"), 100)
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
      filter(
        match("LOVE", { field: "summary" }).or(
          match("rings", { field: "title" })
        )
      )
        .topk(field("published_year"), 100)
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
      filter(
        match("LOVE", { field: "summary" }).and(
          match("class", { field: "summary" })
        )
      )
        .topk(field("published_year"), 100)
    );

    expect(new Set(result.map((doc) => doc._id))).toEqual(new Set(["pride"]));
  });

  test("query text filter multiple terms conjunctive with all", async () => {
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
    ]);

    const result = await ctx.client.collection(collection.name).query(
      filter(match("story love", { field: "summary", all: true }))
        .topk(field("published_year"), 100)
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

    const result = await ctx.client.collection(collection.name).query(
      filter(match("the", { field: "summary" }))
        .topk(field("published_year"), 100)
    );

    expect(result.length).toBe(0);
  });

  test("query text filter with weight", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "pride",
        summary: "A story about love and class or love and wealth",
        published_year: 1813,
      },
      {
        _id: "gatsby",
        summary: "A tale of power and wealth",
        published_year: 1925,
      },
      {
        _id: "lotr",
        summary: "A fantasy epic",
        published_year: 1954,
      },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      select({
        summary: field("summary"),
        summary_score: fn.bm25Score(),
      })
        .filter(
          match("tale", { field: "summary", weight: 2 }).or(
            match("love", { field: "summary" })
          )
        )
        .topk(field("summary_score"), 100)
    );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["gatsby", "pride"])
    );
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
          select({ bm25_score: fn.bm25Score() })
            .filter(field("_id").eq("pride"))
            .topk(field("bm25_score"), 100)
        )
    ).rejects.toThrow(
      "invalid argument: Invalid query: Query must have at least one text filter to compute bm25 scores"
    );
  });

  test("query text matches single term", async () => {
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
    ]);

    for (const matchExpr of [
      field("summary").matchAny("love"),
      field("summary").matchAll("love"),
    ]) {
      const result = await ctx.client.collection(collection.name).query(
        filter(matchExpr)
          .topk(field("published_year"), 100)
      );

      expect(new Set(result.map((doc) => doc._id))).toEqual(
        new Set(["pride", "gatsby"])
      );
    }
  });

  test("query text match all two terms", async () => {
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
    ]);

    const result = await ctx.client.collection(collection.name).query(
      filter(field("summary").matchAll("love class"))
        .topk(field("published_year"), 100)
    );

    expect(new Set(result.map((doc) => doc._id))).toEqual(new Set(["pride"]));
  });

  test("query text match any two terms", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "pride",
        summary: "A witty exploration of love, social class, and marriage in 19th-century England.",
        published_year: 1813,
      },
      {
        _id: "gatsby",
        summary: "A mysterious millionaire navigates love and wealth in the Roaring Twenties.",
        published_year: 1925,
      },
      {
        _id: "lotr",
        summary: "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
        published_year: 1954,
      },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      filter(field("summary").matchAny("love ring"))
        .topk(field("published_year"), 100)
    );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["pride", "gatsby", "lotr"])
    );
  });

  test("query text matches with logical expr", async () => {
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
    ]);

    const result = await ctx.client.collection(collection.name).query(
      filter(
        field("summary").matchAll("love class").or(field("published_year").eq(1925))
      )
        .topk(field("published_year"), 10)
    );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["pride", "gatsby"])
    );
  });

  test("query text matches on invalid field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", published_year: 1813 },
    ]);

    await expect(
      ctx.client
        .collection(collection.name)
        .query(
          filter(field("published_year").matchAll("love class"))
            .count()
        )
    ).rejects.toThrow("invalid argument");
  });
});
