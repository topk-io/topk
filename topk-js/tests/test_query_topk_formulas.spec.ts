import { field, select, fn } from "../lib/query";
import { int, keywordIndex, text, f32Vector } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("TopK Formulas", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  async function createTestCollection(ctx: ProjectContext) {
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int().required(),
      summary_embedding: f32Vector({ dimension: 16 }).required(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", title: "The Catcher in the Rye", published_year: 1951, summary_embedding: Array(16).fill(1) },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925, summary_embedding: Array(16).fill(2) },
      { _id: "moby", title: "Moby Dick", published_year: 1851, summary_embedding: Array(16).fill(3) },
      { _id: "mockingbird", title: "To Kill a Mockingbird", published_year: 1960, summary_embedding: Array(16).fill(4) },
      { _id: "alchemist", title: "The Alchemist", published_year: 1988, summary_embedding: Array(16).fill(5) },
      { _id: "harry", title: "Harry Potter", published_year: 1997, summary_embedding: Array(16).fill(6) },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954, summary_embedding: Array(16).fill(7) },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813, summary_embedding: Array(16).fill(8) },
      { _id: "1984", title: "1984", published_year: 1949, summary_embedding: Array(16).fill(9) },
      { _id: "hobbit", title: "The Hobbit", published_year: 1937, summary_embedding: Array(16).fill(10) },
    ]);

    return collection;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("query topk clamping", async () => {
    const ctx = getContext();
    const collection = await createTestCollection(ctx);

    await expect(
      ctx.client.collection(collection.name).query(
        select({
          summary_distance: fn.vectorDistance("summary_embedding", Array(16).fill(2)),
          bm25_score: fn.bm25Score(),
        })
        .topk(
            field("bm25_score").max(3).min(10).add(field("summary_distance").mul(0.5)),
            2,
            true
          )
        )
      ).rejects.toThrow(/invalid argument/);
  });

  test("query topk pow sqrt", async () => {
    const ctx = getContext();
    const collection = await createTestCollection(ctx);

    await expect(
      ctx.client.collection(collection.name).query(
        select({
          summary_distance: fn.vectorDistance("summary_embedding", Array(16).fill(2)),
          bm25_score: fn.bm25Score(),
        })
        .topk(
            (field("bm25_score").pow(1.5).add(field("summary_distance").pow(2))).sqrt(),
            2,
            true
          )
        )
      ).rejects.toThrow(/invalid argument/);
  });

  test("query topk exp precision", async () => {
    const ctx = getContext();
    const collection = await createTestCollection(ctx);

    await expect(
      ctx.client.collection(collection.name).query(
        select({})
          .filter(field("published_year").exp().ln().sub(1988).abs().lte(10e-6))
          .topk(field("published_year"), 2, true)
        )
      ).rejects.toThrow(/invalid argument/);
  });
}); 