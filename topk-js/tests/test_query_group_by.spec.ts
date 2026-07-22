import { agg, field, filter, groupBy, select } from "../lib/query";
import { float, int, keywordIndex, text } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

// published_year per book in the `books` dataset:
//   mockingbird 1960, 1984 1949, pride 1813, gatsby 1925, catcher 1951,
//   moby 1851, hobbit 1937, harry 1997, lotr 1954, alchemist 1988
//
// `published_year < 1940` splits them into:
//   old  (4): pride 1813, gatsby 1925, moby 1851, hobbit 1937
//   new  (6): mockingbird 1960, 1984 1949, catcher 1951, harry 1997, lotr 1954, alchemist 1988

describe("GroupBy Queries", () => {
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
      nullable_importance: float(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        published_year: 1960,
        nullable_importance: 2.5,
      },
      { _id: "1984", title: "1984", published_year: 1949 },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813 },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925 },
      { _id: "catcher", title: "The Catcher in the Rye", published_year: 1951 },
      {
        _id: "moby",
        title: "Moby Dick",
        published_year: 1851,
        nullable_importance: 5.5,
      },
      { _id: "hobbit", title: "The Hobbit", published_year: 1937 },
      { _id: "harry", title: "Harry Potter", published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954 },
      { _id: "alchemist", title: "The Alchemist", published_year: 1988 },
    ]);

    return collection;
  }

  test("group by bool key expr", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { is_old: field("published_year").lt(1940) },
        { count: agg.count() }
      )
    );

    expect(result.length).toBe(2);

    for (const row of result) {
      if (row.is_old) {
        expect(row.count).toBe(4);
      } else {
        expect(row.count).toBe(6);
      }
    }
  });

  test("group by count", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { is_old: field("published_year").lt(1940) },
        { count: agg.count() }
      )
    );

    const counts: Record<string, number> = {};
    for (const row of result) {
      counts[String(row.is_old)] = row.count;
    }

    expect(counts).toEqual({ true: 4, false: 6 });
  });

  test("group by count field", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    // `nullable_importance` is only present on `mockingbird` (new, 2.5) and `moby` (old, 5.5).
    // `count()` counts every row, while `count(field)` counts non-null values.
    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { is_old: field("published_year").lt(1940) },
        {
          total: agg.count(),
          with_importance: agg.count("nullable_importance"),
        }
      )
    );

    const byGroup: Record<string, [number, number]> = {};
    for (const row of result) {
      byGroup[String(row.is_old)] = [row.total, row.with_importance];
    }

    expect(byGroup).toEqual({ true: [4, 1], false: [6, 1] });
  });

  test("group by sum", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { is_old: field("published_year").lt(1940) },
        { total_year: agg.sum("published_year") }
      )
    );

    const sums: Record<string, number> = {};
    for (const row of result) {
      sums[String(row.is_old)] = row.total_year;
    }

    // old: 1813 + 1925 + 1851 + 1937 = 7526
    // new: 1960 + 1949 + 1951 + 1997 + 1954 + 1988 = 11799
    expect(sums).toEqual({ true: 7526, false: 11799 });
  });

  test("group by min max", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { is_old: field("published_year").lt(1940) },
        {
          oldest: agg.min("published_year"),
          newest: agg.max("published_year"),
        }
      )
    );

    const byGroup: Record<string, [number, number]> = {};
    for (const row of result) {
      byGroup[String(row.is_old)] = [row.oldest, row.newest];
    }

    // old: min 1813 (pride), max 1937 (hobbit)
    // new: min 1949 (1984), max 1997 (harry)
    expect(byGroup).toEqual({ true: [1813, 1937], false: [1949, 1997] });
  });

  test("group by avg", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { is_old: field("published_year").lt(1940) },
        { avg_year: agg.avg("published_year") }
      )
    );

    const avgs: Record<string, number> = {};
    for (const row of result) {
      avgs[String(row.is_old)] = row.avg_year;
    }

    expect(avgs.true).toBeCloseTo(1881.5, 9);
    expect(avgs.false).toBeCloseTo(1966.5, 9);
  });

  test("group by multiple aggregations", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { is_old: field("published_year").lt(1940) },
        {
          count: agg.count(),
          total_year: agg.sum("published_year"),
          oldest: agg.min("published_year"),
          newest: agg.max("published_year"),
          avg_year: agg.avg("published_year"),
        }
      )
    );

    expect(result.length).toBe(2);

    for (const row of result) {
      if (row.is_old) {
        expect(row.count).toBe(4);
        expect(row.total_year).toBe(7526);
        expect(row.oldest).toBe(1813);
        expect(row.newest).toBe(1937);
        expect(row.avg_year).toBeCloseTo(1881.5, 9);
      } else {
        expect(row.count).toBe(6);
        expect(row.total_year).toBe(11799);
        expect(row.oldest).toBe(1949);
        expect(row.newest).toBe(1997);
        expect(row.avg_year).toBeCloseTo(1966.5, 9);
      }
    }
  });

  test("group by multiple keys", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    // Two independent key expressions:
    //   is_old  = published_year < 1940
    //   is_19th = published_year < 1900
    //
    //   pride 1813:  (old, 19th)
    //   moby  1851:  (old, 19th)
    //   gatsby 1925: (old, !19th)
    //   hobbit 1937: (old, !19th)
    //   the other 6: (!old, !19th)
    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        {
          is_old: field("published_year").lt(1940),
          is_19th: field("published_year").lt(1900),
        },
        { count: agg.count() }
      )
    );

    const counts: Record<string, number> = {};
    for (const row of result) {
      counts[`${row.is_old},${row.is_19th}`] = row.count;
    }

    expect(counts).toEqual({
      "true,true": 2,
      "true,false": 2,
      "false,false": 6,
    });
  });

  test("group by with filter", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    // Filter to books published in 1940 or later (drops the 4 "old" books),
    // then group the remaining 6 by whether they were published after 1980.
    //   after 1980: harry 1997, alchemist 1988          -> 2
    //   otherwise:  mockingbird, 1984, catcher, lotr     -> 4
    const result = await ctx.client.collection(collection.name).query(
      filter(field("published_year").gte(1940)).groupBy(
        { recent: field("published_year").gt(1980) },
        { count: agg.count() }
      )
    );

    const counts: Record<string, number> = {};
    for (const row of result) {
      counts[String(row.recent)] = row.count;
    }

    expect(counts).toEqual({ true: 2, false: 4 });
  });

  test("group by with projected columns", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    // A preceding `select` projects computed columns (`year`, `old`) which the
    // group_by stage then references in both its key and its aggregations.
    const result = await ctx.client.collection(collection.name).query(
      select({
        year: field("published_year"),
        old: field("published_year").lt(1940),
      }).groupBy(
        { old: field("old") },
        { count: agg.count(), total_year: agg.sum("year") }
      )
    );

    const byGroup: Record<string, [number, number]> = {};
    for (const row of result) {
      byGroup[String(row.old)] = [row.count, row.total_year];
    }

    expect(byGroup).toEqual({ true: [4, 7526], false: [6, 11799] });
  });

  test("group by then filter", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    // Group into old (4) / new (6), then keep only groups with more than 4 members.
    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { is_old: field("published_year").lt(1940) },
        { count: agg.count() }
      ).filter(field("count").gt(4))
    );

    expect(result.length).toBe(1);
    expect(result[0].is_old).toBe(false);
    expect(result[0].count).toBe(6);
  });

  test("group by then sort limit", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    // Group, then take the single largest group by count.
    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { is_old: field("published_year").lt(1940) },
        { count: agg.count() }
      )
        .sort(field("count"), false)
        .limit(1)
    );

    expect(result.length).toBe(1);
    expect(result[0].is_old).toBe(false);
    expect(result[0].count).toBe(6);
  });

  test("group by then select", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    // A `select` after group_by projects a subset / renaming of the grouped output.
    const result = await ctx.client.collection(collection.name).query(
      groupBy(
        { is_old: field("published_year").lt(1940) },
        { count: agg.count() }
      ).select({ n: field("count") })
    );

    expect(result.length).toBe(2);

    const ns = result.map((row: any) => row.n).sort((a: number, b: number) => a - b);
    expect(ns).toEqual([4, 6]);
  });

  test("group by empty keys should reject", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    await expect(
      ctx.client.collection(collection.name).query(
        groupBy({}, { count: agg.count() })
      )
    ).rejects.toThrow(/at least one key/);
  });

  test("group by empty aggregations should reject", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    await expect(
      ctx.client.collection(collection.name).query(
        groupBy({ is_old: field("published_year").lt(1940) }, {})
      )
    ).rejects.toThrow(/at least one aggregation/);
  });
});
