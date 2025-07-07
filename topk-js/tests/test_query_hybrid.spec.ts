import { field, fn, match, select } from "../lib/query";
import { f32Vector, int, keywordIndex, text, vectorIndex } from "../lib/schema";
import { isSorted, newProjectContext, ProjectContext } from "./setup";

describe("Hybrid Queries", () => {
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
      published_year: int().required(),
      summary: text().required().index(keywordIndex()),
      summary_embedding: f32Vector({ dimension: 16 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
      nullable_embedding: f32Vector({ dimension: 16 })
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        published_year: 1960,
        summary: "A young girl confronts racial injustice in the Deep South through the eyes of her lawyer father.",
        summary_embedding: Array(16).fill(1.0),
        nullable_embedding: Array(16).fill(1.0),
        nullable_importance: 2.0,
      },
      {
        _id: "1984",
        title: "1984",
        published_year: 1949,
        summary: "A totalitarian regime uses surveillance and mind control to oppress its citizens.",
        summary_embedding: Array(16).fill(2.0),
        nullable_embedding: Array(16).fill(2.0),
      },
      {
        _id: "pride",
        title: "Pride and Prejudice",
        published_year: 1813,
        summary: "A witty exploration of love, social class, and marriage in 19th-century England.",
        summary_embedding: Array(16).fill(3.0),
      },
      {
        _id: "gatsby",
        title: "The Great Gatsby",
        published_year: 1925,
        summary: "A mysterious millionaire navigates love and wealth in the Roaring Twenties.",
        summary_embedding: Array(16).fill(4.0),
      },
      {
        _id: "catcher",
        title: "The Catcher in the Rye",
        published_year: 1951,
        summary: "A rebellious teenager struggles with alienation and identity in mid-20th-century America.",
        summary_embedding: Array(16).fill(5.0),
        nullable_embedding: Array(16).fill(5.0),
      },
      {
        _id: "moby",
        title: "Moby-Dick",
        published_year: 1851,
        summary: "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.",
        summary_embedding: Array(16).fill(6.0),
        nullable_importance: 5.0,
      },
      {
        _id: "hobbit",
        title: "The Hobbit",
        published_year: 1937,
        summary: "A reluctant hobbit embarks on a quest to help a group of dwarves reclaim their mountain home.",
        summary_embedding: Array(16).fill(7.0),
      },
      {
        _id: "harry",
        title: "Harry Potter and the Sorcerer's Stone",
        published_year: 1997,
        summary: "A young wizard discovers his magical heritage and attends a school for witchcraft and wizardry.",
        summary_embedding: Array(16).fill(8.0),
        nullable_embedding: Array(16).fill(8.0),
      },
      {
        _id: "lotr",
        title: "The Lord of the Rings: The Fellowship of the Ring",
        published_year: 1954,
        summary: "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
        summary_embedding: Array(16).fill(9.0),
      },
      {
        _id: "alchemist",
        title: "The Alchemist",
        published_year: 1988,
        summary: "A shepherd boy journeys to fulfill his destiny and discover the meaning of life.",
        summary_embedding: Array(16).fill(10.0),
      },
    ]);

    return collection;
  }

  test("test_query_hybrid_vector_bm25", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client.collection(collection.name).query(
      select({
        summary_distance: fn.vectorDistance("summary_embedding", Array(16).fill(2.0)),
        bm25_score: fn.bm25Score(),
      })
        .filter(field("summary").match("love", undefined, 30.0, false).or(field("summary").match("young", undefined, 10.0, false)))
        .topk(field("bm25_score").add(field("summary_distance").mul(100)), 2, true)
    );

    expect(result.length).toBe(2);
    expect(result.map((doc) => doc._id)).toEqual(["mockingbird", "pride"]);
  });

  test("test_query_hybrid_keyword_boost", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    // Multiply summary_distance by 0.1 if the summary matches "racial injustice", otherwise
    // multiply by 1.0 (leave unchanged).
    for (const scoreExpr of [
      field("summary_distance").mul(field("summary").matchAll("racial injustice").choose(0.1, 1.0)),
      field("summary_distance").boost(field("summary").matchAll("racial injustice"), 0.1),
    ]) {
      const result = await ctx.client.collection(collection.name).query(
        select({
          summary_distance: fn.vectorDistance("summary_embedding", Array(16).fill(2.3)),
        })
          .topk(scoreExpr, 3, true)
      );

      // Keyword boosting swaps the order of results so we expect [1984, mockingbird, pride]
      // instead of [1984, pride, mockingbird].
      expect(result.map((doc) => doc._id)).toEqual(["1984", "mockingbird", "pride"]);

      // We use a modified scoring expression so the results are not sorted by summary_distance.
      expect(isSorted(result, "summary_distance")).toBe(false);
    }
  });

  test("test_query_hybrid_coalesce_score", async () => {
    const ctx = getContext();
    const collection = await setupBooks(ctx);

    const result = await ctx.client.collection(collection.name).query(
      select({
        summary_score: fn.vectorDistance("summary_embedding", Array(16).fill(4.1)),
        nullable_score: fn.vectorDistance("nullable_embedding", Array(16).fill(4.1)),
      })
        .topk(field("summary_score").add(field("nullable_score").coalesce(0.0)), 3, true)
    );

    // Adding the nullable_score without coalescing would exclude "pride" and "gatsby" from
    // the result set, even though they are the closest candidates based on summary_score.
    expect(result.map((doc) => doc._id)).toEqual(["gatsby", "pride", "catcher"]);
  });
}); 