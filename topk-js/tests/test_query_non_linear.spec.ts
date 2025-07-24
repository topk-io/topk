import { field, select, fn, match, filter } from "../lib/query";
import { int, keywordIndex, text, f32Vector } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Non-Linear Math Queries", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("query exp ln", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      summary: text().required().index(keywordIndex()),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "moby", title: "Moby Dick", summary: "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences." },
      { _id: "gatsby", title: "The Great Gatsby", summary: "A mysterious millionaire navigates love and wealth in the Roaring Twenties." },
      { _id: "pride", title: "Pride and Prejudice", summary: "A witty exploration of love, social class, and marriage in 19th-century England." },
      { _id: "hobbit", title: "The Hobbit", summary: "A reluctant hobbit embarks on a quest to help a group of dwarves reclaim their mountain home." },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        bm25_score: fn.bm25Score()
      })
        .select({
          bm25_score_scale: field("bm25_score").mul(1.5).exp(),
          bm25_score_smooth: field("bm25_score").add(1).ln()
        })
        .filter(match("millionaire love consequences dwarves"))
        .topk(field("bm25_score_scale"), 2, false)
    );

    expect(results.map(doc => doc._id)).toEqual(["gatsby", "hobbit"]);

    for (const doc of results) {
      const bm25Score = doc.bm25_score as number;
      const bm25ScoreScale = doc.bm25_score_scale as number;
      const bm25ScoreSmooth = doc.bm25_score_smooth as number;

      expect(Math.abs(Math.exp(bm25Score * 1.5) - bm25ScoreScale)).toBeLessThan(1e-4);
      expect(Math.abs(Math.log(bm25Score + 1.0) - bm25ScoreSmooth)).toBeLessThan(1e-4);
    }
  });

  test("query float inf", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int().required(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", published_year: 1813 },
      { _id: "moby", published_year: 1851 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "hobbit", published_year: 1937 },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        to_infinity: field("published_year").exp()
      })
        .topk(field("published_year"), 2, true)
    );

    expect(results.length).toBe(2);

    for (const doc of results) {
      const toInfinity = doc.to_infinity as number;
      expect(toInfinity).toBe(Infinity);
    }
  });

  test("query sqrt square", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      published_year: int().required(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", published_year: 1813 },
      { _id: "moby", published_year: 1851 },
      { _id: "gatsby", published_year: 1925 },
      { _id: "hobbit", published_year: 1937 },
      { _id: "harry", published_year: 1997 },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        published_year: field("published_year"),
        published_year_2: field("published_year").sqrt().square()
      })
        .topk(field("published_year_2"), 2, true)
    );

    expect(results.map(doc => doc._id)).toEqual(["pride", "moby"]);

    for (const doc of results) {
      const year2 = doc.published_year_2 as number;
      const yearOrig = doc.published_year as number;
      expect(Math.round(year2)).toBe(yearOrig);
    }
  });

  test("query sqrt filter", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required(),
      published_year: int().required(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813 },
      { _id: "moby", title: "Moby Dick", published_year: 1851 },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925 },
      { _id: "hobbit", title: "The Hobbit", published_year: 1937 },
      { _id: "harry", title: "Harry Potter and the Sorcerer's Stone", published_year: 1997 },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        title: field("title")
      })
        .filter(field("published_year").sqrt().gt(Math.sqrt(1990)))
        .topk(field("published_year"), 2, true)
    );

    expect(results).toEqual([
      { _id: "harry", title: "Harry Potter and the Sorcerer's Stone" }
    ]);
  });
});
