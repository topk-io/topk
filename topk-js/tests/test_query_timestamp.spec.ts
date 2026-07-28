import { agg, field, filter, groupBy, literal } from "../lib/query";
import { int, keywordIndex, text, timestamp } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

// published_ts per book in the `books` dataset:
//   mockingbird 1960-07-11, 1984 1949-06-08, pride 1813-01-28, gatsby 1925-04-10,
//   catcher 1951-07-16, moby 1851-10-18, hobbit 1937-09-21, harry 1997-06-26,
//   lotr 1954-07-29, alchemist 1988-01-01

describe("Timestamp Queries", () => {
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
      published_ts: timestamp().required(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        published_year: 1960,
        published_ts: new Date(Date.UTC(1960, 6, 11)),
      },
      {
        _id: "1984",
        title: "1984",
        published_year: 1949,
        published_ts: new Date(Date.UTC(1949, 5, 8)),
      },
      {
        _id: "pride",
        title: "Pride and Prejudice",
        published_year: 1813,
        published_ts: new Date(Date.UTC(1813, 0, 28)),
      },
      {
        _id: "gatsby",
        title: "The Great Gatsby",
        published_year: 1925,
        published_ts: new Date(Date.UTC(1925, 3, 10)),
      },
      {
        _id: "catcher",
        title: "The Catcher in the Rye",
        published_year: 1951,
        published_ts: new Date(Date.UTC(1951, 6, 16)),
      },
      {
        _id: "moby",
        title: "Moby Dick",
        published_year: 1851,
        published_ts: new Date(Date.UTC(1851, 9, 18)),
      },
      {
        _id: "hobbit",
        title: "The Hobbit",
        published_year: 1937,
        published_ts: new Date(Date.UTC(1937, 8, 21)),
      },
      {
        _id: "harry",
        title: "Harry Potter",
        published_year: 1997,
        published_ts: new Date(Date.UTC(1997, 5, 26)),
      },
      {
        _id: "lotr",
        title: "The Lord of the Rings",
        published_year: 1954,
        published_ts: new Date(Date.UTC(1954, 6, 29)),
      },
      {
        _id: "alchemist",
        title: "The Alchemist",
        published_year: 1988,
        published_ts: new Date(Date.UTC(1988, 0, 1)),
      },
    ]);

    return collection;
  }

  test("query filter timestamp", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        filter(
          field("published_ts").lt(literal(new Date(Date.UTC(1929, 0, 1))))
        ).limit(20)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["pride", "moby", "gatsby"])
    );
  });

  test("query date_part eq field", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        filter(
          field("published_ts").datePart("year").eq(field("published_year"))
        ).count()
      );

    expect(result[0]._count).toBe(10);
  });

  test("query date_part lt literal", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client
      .collection(collection.name)
      .query(filter(field("published_ts").datePart("month").lt(6)).limit(10));

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["gatsby", "pride", "alchemist"])
    );
  });

  test("query date_part group by", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { published_month: field("published_ts").datePart("month") },
        { count: agg.count() }
      )
    );

    const rows = result
      .map((row) => [row.published_month, row.count])
      .sort((a, b) => a[0] - b[0]);

    expect(rows).toEqual([
      [1, 2],
      [4, 1],
      [6, 2],
      [7, 3],
      [9, 1],
      [10, 1],
    ]);
  });
});
