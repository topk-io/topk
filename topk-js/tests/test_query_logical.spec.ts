import { field, select, not, filter } from "../lib/query";
import { int, keywordIndex, text } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Logical Queries", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("query lte", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", title: "The Catcher in the Rye", published_year: 1951 },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925 },
      { _id: "moby", title: "Moby Dick", published_year: 1851 },
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        published_year: 1960,
      },
      { _id: "alchemist", title: "The Alchemist", published_year: 1988 },
      { _id: "harry", title: "Harry Potter", published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954 },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813 },
      { _id: "1984", title: "1984", published_year: 1949 },
      { _id: "hobbit", title: "The Hobbit", published_year: 1937 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        filter(field("published_year").lte(1950))
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(results.map((doc) => doc._id))).toEqual(
      new Set(["1984", "pride", "hobbit", "moby", "gatsby"])
    );
  });

  test("query and", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", title: "The Catcher in the Rye", published_year: 1951 },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925 },
      { _id: "moby", title: "Moby Dick", published_year: 1851 },
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        published_year: 1960,
      },
      { _id: "alchemist", title: "The Alchemist", published_year: 1988 },
      { _id: "harry", title: "Harry Potter", published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954 },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813 },
      { _id: "1984", title: "1984", published_year: 1949 },
      { _id: "hobbit", title: "The Hobbit", published_year: 1937 },
    ]);

    const results = await ctx.client.collection(collection.name).query(
        filter(
          field("published_year")
            .lte(1950)
            .and(field("published_year").gte(1948))
        )
        .topk(field("published_year"), 100, true)
    );

    expect(new Set(results.map((doc) => doc._id))).toEqual(new Set(["1984"]));
  });

  test("query is null", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", published_year: 1951 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "moby", title: "Moby Dick", published_year: 1851 },
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        published_year: 1960,
      },
      { _id: "alchemist", title: "The Alchemist", published_year: 1988 },
      { _id: "harry", title: "Harry Potter", published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954 },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813 },
      { _id: "1984", title: "1984", published_year: 1949 },
      { _id: "hobbit", title: "The Hobbit", published_year: 1937 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        filter(field("title").isNull())
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(results.map((doc) => doc._id))).toEqual(
      new Set(["catcher", "gatsby"])
    );
  });

  test("query is not null", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", published_year: 1951 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "moby", title: "Moby Dick", published_year: 1851 },
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        published_year: 1960,
      },
      { _id: "alchemist", title: "The Alchemist", published_year: 1988 },
      { _id: "harry", title: "Harry Potter", published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954 },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813 },
      { _id: "1984", title: "1984", published_year: 1949 },
      { _id: "hobbit", title: "The Hobbit", published_year: 1937 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        filter(field("title").isNotNull())
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(results.map((doc) => doc._id))).toEqual(
      new Set([
        "moby",
        "mockingbird",
        "alchemist",
        "harry",
        "lotr",
        "pride",
        "1984",
        "hobbit",
      ])
    );
  });

  test("query not", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813 },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925 },
      { _id: "catcher", title: "The Catcher in the Rye", published_year: 1951 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        filter(not(field("_id").contains("gatsby")))
          .topk(field("published_year"), 100, false)
      );

    expect(new Set(results.map((doc) => doc._id))).toEqual(
      new Set([
        "pride",
        "catcher",
      ])
    );
  });

  test("query choose literal", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", summary: "A totalitarian regime uses surveillance and mind control to oppress its citizens." },
      { _id: "pride", summary: "A witty exploration of love, social class, and marriage in 19th-century England." },
      { _id: "gatsby", summary: "A mysterious millionaire navigates love and wealth in the Roaring Twenties." },
      { _id: "catcher", summary: "A rebellious teenager struggles with alienation and identity in mid-20th-century America." },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({
          love_score: field("summary").matchAll("love").choose(2.0, 0.1)
        })
          .filter(field("love_score").gt(1.0))
          .topk(field("love_score"), 10, false)
      );

    expect(new Set(results.map((doc) => doc._id))).toEqual(
      new Set(["pride", "gatsby"])
    );
  });

  test("query choose literal and field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "mockingbird", summary: "A young girl confronts racial injustice in the Deep South through the eyes of her lawyer father.", published_year: 1960 },
      { _id: "1984", summary: "A totalitarian regime uses surveillance and mind control to oppress its citizens.", published_year: 1949 },
      { _id: "pride", summary: "A witty exploration of love, social class, and marriage in 19th-century England.", published_year: 1813 },
      { _id: "gatsby", summary: "A mysterious millionaire navigates love and wealth in the Roaring Twenties.", published_year: 1925 },
      { _id: "catcher", summary: "A rebellious teenager struggles with alienation and identity in mid-20th-century America.", published_year: 1951 },
      { _id: "moby", summary: "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.", published_year: 1851 },
      { _id: "hobbit", summary: "A reluctant hobbit embarks on a quest to help a group of dwarves reclaim their mountain home.", published_year: 1937 },
      { _id: "harry", summary: "A young wizard discovers his magical heritage and attends a school for witchcraft and wizardry.", published_year: 1997 },
      { _id: "lotr", summary: "A group of unlikely heroes sets out to destroy a powerful, evil ring.", published_year: 1954 },
      { _id: "alchemist", summary: "A shepherd boy journeys to fulfill his destiny and discover the meaning of life.", published_year: 1988 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({
          love_score: field("summary").matchAll("love").choose(field("published_year"), 10)
        })
          .topk(field("love_score"), 2, false)
      );

    expect(results).toEqual([
      { _id: "gatsby", love_score: 1925 },
      { _id: "pride", love_score: 1813 },
    ]);
  });

  test("query choose field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      summary: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "mockingbird", summary: "A young girl confronts racial injustice in the Deep South through the eyes of her lawyer father.", published_year: 1960 },
      { _id: "1984", summary: "A totalitarian regime uses surveillance and mind control to oppress its citizens.", published_year: 1949 },
      { _id: "pride", summary: "A witty exploration of love, social class, and marriage in 19th-century England.", published_year: 1813 },
      { _id: "gatsby", summary: "A mysterious millionaire navigates love and wealth in the Roaring Twenties.", published_year: 1925 },
      { _id: "catcher", summary: "A rebellious teenager struggles with alienation and identity in mid-20th-century America.", published_year: 1951 },
      { _id: "moby", summary: "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.", published_year: 1851 },
      { _id: "hobbit", summary: "A reluctant hobbit embarks on a quest to help a group of dwarves reclaim their mountain home.", published_year: 1937 },
      { _id: "harry", summary: "A young wizard discovers his magical heritage and attends a school for witchcraft and wizardry.", published_year: 1997 },
      { _id: "lotr", summary: "A group of unlikely heroes sets out to destroy a powerful, evil ring.", published_year: 1954 },
      { _id: "alchemist", summary: "A shepherd boy journeys to fulfill his destiny and discover the meaning of life.", published_year: 1988 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({
          love_score: field("summary").matchAll("love").choose(field("published_year"), field("published_year").div(10))
        })
          .topk(field("love_score"), 3, false)
      );

    expect(results).toEqual([
      { _id: "gatsby", love_score: 1925 },
      { _id: "pride", love_score: 1813 },
      { _id: "harry", love_score: 199 },
    ]);
  });

  test("query coalesce nullable", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int(),
      nullable_importance: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "mockingbird", published_year: 1960 },
      { _id: "1984", published_year: 1949 },
      { _id: "pride", published_year: 1813, nullable_importance: 1 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "catcher", published_year: 1951 },
      { _id: "moby", published_year: 1851, nullable_importance: 5 },
      { _id: "hobbit", published_year: 1937 },
      { _id: "harry", published_year: 1997 },
      { _id: "lotr", published_year: 1954 },
      { _id: "alchemist", published_year: 1988 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({
          importance: field("nullable_importance").coalesce(1.0)
        })
          .filter(field("published_year").lt(1900))
          .topk(field("published_year"), 3, false)
      );

    expect(results).toEqual([
      { _id: "moby", importance: 5.0 },
      { _id: "pride", importance: 1.0 },
    ]);
  });

  test("query coalesce missing", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "mockingbird", published_year: 1960 },
      { _id: "1984", published_year: 1949 },
      { _id: "pride", published_year: 1813 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "catcher", published_year: 1951 },
      { _id: "moby", published_year: 1851 },
      { _id: "hobbit", published_year: 1937 },
      { _id: "harry", published_year: 1997 },
      { _id: "lotr", published_year: 1954 },
      { _id: "alchemist", published_year: 1988 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({
          importance: field("missing_field").coalesce(1.0)
        })
          .filter(field("published_year").lt(1900))
          .topk(field("published_year"), 3, false)
      );

    expect(results).toEqual([
      { _id: "moby", importance: 1.0 },
      { _id: "pride", importance: 1.0 },
    ]);
  });

  test("query coalesce non nullable", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "mockingbird", published_year: 1960 },
      { _id: "1984", published_year: 1949 },
      { _id: "pride", published_year: 1813 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "catcher", published_year: 1951 },
      { _id: "moby", published_year: 1851 },
      { _id: "hobbit", published_year: 1937 },
      { _id: "harry", published_year: 1997 },
      { _id: "lotr", published_year: 1954 },
      { _id: "alchemist", published_year: 1988 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({
          coalesced_year: field("published_year").coalesce(0)
        })
          .filter(field("published_year").lt(1900))
          .topk(field("published_year"), 3, false)
      );

    expect(results).toEqual([
      { _id: "moby", coalesced_year: 1851 },
      { _id: "pride", coalesced_year: 1813 },
    ]);
  });
});
