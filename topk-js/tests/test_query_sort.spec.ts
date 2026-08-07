import { field, select } from "../lib/query";
import { int, keywordIndex, text } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Sort Queries", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  async function setupBooks(ctx: ProjectContext) {
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      rating: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813, rating: 10 },
      { _id: "moby", title: "Moby Dick", published_year: 1851, rating: 10 },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925, rating: 9 },
      { _id: "hobbit", title: "The Hobbit", published_year: 1937, rating: 9 },
      { _id: "1984", title: "1984", published_year: 1949, rating: 8 },
      { _id: "catcher", title: "The Catcher in the Rye", published_year: 1951, rating: 7 },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954, rating: 6 },
      { _id: "mockingbird", title: "To Kill a Mockingbird", published_year: 1960, rating: 5 },
      { _id: "alchemist", title: "The Alchemist", published_year: 1988, rating: 4 },
      { _id: "harry", title: "Harry Potter", published_year: 1997, rating: 3 },
    ]);

    return collection;
  }

  test("query sort by expression defaults to ascending", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const results = await ctx.client
      .collection(collection.name)
      .query(select({ title: field("title") }).sort(field("published_year")).limit(3));

    expect(results.map((doc) => doc._id)).toEqual(["pride", "moby", "gatsby"]);
  });

  test.each([
    [false, ["harry", "alchemist", "mockingbird"]],
    [true, ["pride", "moby", "gatsby"]],
  ])("query sort with asc=%p", async (asc, expected) => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({ title: field("title") }).sort(field("published_year"), asc).limit(3)
      );

    expect(results.map((doc) => doc._id)).toEqual(expected);
  });

  test("query sort by single sort expression", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({ title: field("title") })
          .sort([{ expr: field("published_year"), order: "asc" }])
          .limit(3)
      );

    expect(results.map((doc) => doc._id)).toEqual(["pride", "moby", "gatsby"]);
  });

  test("query sort by multiple sort expressions", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        select({ title: field("title") })
          .sort([
            { expr: field("rating"), order: "desc" },
            { expr: field("published_year"), order: "asc" },
          ])
          .limit(4)
      );

    expect(results.map((doc) => doc._id)).toEqual([
      "pride",
      "moby",
      "gatsby",
      "hobbit",
    ]);
  });

  test("query sort with invalid order throws", () => {
    expect(() =>
      // @ts-expect-error - invalid sort order
      select({ title: field("title") }).sort([
        { expr: field("published_year"), order: "up" },
      ])
    ).toThrow();
  });

  test("query sort with a missing order throws", () => {
    expect(() =>
      // @ts-expect-error - order is required on each sort expression
      select({ title: field("title") }).sort([{ expr: field("published_year") }])
    ).toThrow();
  });

  test("query sort with sort expressions and asc argument throws", () => {
    expect(() =>
      select({ title: field("title") }).sort(
        // @ts-expect-error - asc is only valid with a single sort expression
        [{ expr: field("published_year"), order: "asc" }],
        true
      )
    ).toThrow();
  });
});
