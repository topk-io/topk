import { bm25Score, field, match, select, semanticSimilarity, vectorDistance } from '../query';
import { binaryVector, f32Vector, keywordIndex, semanticIndex, text, u8Vector, VectorDistanceMetric, vectorIndex } from '../schema';
import { binaryVector as binaryVectorValue, u8Vector as u8VectorValue } from '../index';
import { newProjectContext, ProjectContext } from "./setup";

describe("Documents", () => {
  let ctx: ProjectContext;
  let collectionsCreated: string[] = [];

  beforeEach(async () => {
    ctx = newProjectContext();
    collectionsCreated = [];
  });

  afterEach(async () => {
    // Only delete collections created within the test
    for (const collectionName of collectionsCreated) {
      try {
        await ctx.client.collections().delete(collectionName);
      } catch (error) {
        console.error(`Error deleting collection ${collectionName}:`, error);
      }
    }
  });

  // Helper function to create a collection and track it
  const createCollection = async (name: string, schema: any = {}) => {
    const collection = await ctx.client.collections().create(name, schema);
    collectionsCreated.push(name);
    return collection;
  };

  test("get", async () => {
    await createCollection(ctx.scope("books"));

    // get non-existent document
    await expect(
      ctx.client.collection(ctx.scope("books")).get("one")
    ).rejects.toThrow();

    // upsert document
    const lsn = await ctx.client
      .collection(ctx.scope("books"))
      .upsert([{ _id: "one", name: "one", rank: 1 }]);
    expect(lsn).toBe(1);

    // get document
    const doc = await ctx.client.collection(ctx.scope("books")).get("one");
    expect(doc).toEqual({ _id: "one", name: "one", rank: 1 });
  });

  test("upsert", async () => {
    await createCollection(ctx.scope("books"));

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "one", name: "one", rank: 1 },
      { _id: "two", name: "two", extra: 1, rank: 2 },
    ]);
    expect(lsn).toBe(1);

    const docs = await ctx.client.collection(ctx.scope("books")).query(
      select({ name: field("name") })
        .filter(field("name").eq("two"))
        .top_k(field("rank"), 10),
      lsn
    );

    expect(docs).toEqual([{ _id: "two", name: "two" }]);
  });

  test("keyword_search", async () => {
    await createCollection(ctx.scope("books"), {
      title: text().required().index(keywordIndex()),
    });

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", title: "red purple green" },
      { _id: "doc2", title: "yellow purple pink" },
      { _id: "doc3", title: "orange red blue" },
      { _id: "doc4", title: "green yellow purple" },
      { _id: "doc5", title: "pink orange red" },
      { _id: "doc6", title: "black green yellow" },
      { _id: "doc7", title: "purple pink orange" },
      { _id: "doc8", title: "red yello green" },
      { _id: "doc9", title: "yellow purple pink" },
      { _id: "doc10", title: "orange red blue" },
    ]);
    expect(lsn).toBe(1);

    const docs = await ctx.client.collection(ctx.scope("books")).query(
      select({
        text_score: bm25Score(),
      })
        .filter(match("red").or(match("blue")))
        .top_k(field("text_score"), 5),
      lsn
    );

    const docIds = new Set(docs.map((d) => d._id));
    expect(docIds).toEqual(new Set(["doc1", "doc10", "doc3", "doc5", "doc8"]));
  });

  test("vector_search_f32", async () => {
    await createCollection(ctx.scope("books"), {
      f32_embedding: f32Vector(3)
        .required()
        .index(vectorIndex({ metric: VectorDistanceMetric.Euclidean })),
    });

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", f32_embedding: [1.0, 2.0, 3.0] },
      { _id: "doc2", f32_embedding: [4.0, 5.0, 6.0] },
      { _id: "doc3", f32_embedding: [7.0, 8.0, 9.0] },
    ]);
    expect(lsn).toBe(1);

    let docs = await ctx.client.collection(ctx.scope("books")).query(
      select({
        vector_distance: vectorDistance("f32_embedding", [7.0, 8.0, 9.0]),
      }).top_k(field("vector_distance"), 2, true),
      lsn
    );
    docs.sort((a, b) => a.vector_distance - b.vector_distance);

    expect(docs.map((d) => d._id)).toEqual(["doc3", "doc2"]);
  });

  test("vector_search_u8", async () => {
    await createCollection(ctx.scope("books"), {
      u8_embedding: u8Vector(3)
        .required()
        .index(vectorIndex({ metric: VectorDistanceMetric.Euclidean })),
    });

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", u8_embedding: u8VectorValue([1, 2, 3]) },
      { _id: "doc2", u8_embedding: u8VectorValue([4, 5, 6]) },
      { _id: "doc3", u8_embedding: u8VectorValue([7, 8, 9]) },
    ]);
    expect(lsn).toBe(1);

    let docs = await ctx.client.collection(ctx.scope("books")).query(
      select({
        vector_distance: vectorDistance("u8_embedding", u8VectorValue([7, 8, 9])),
      }).top_k(field("vector_distance"), 2, true),
      lsn
    );
    docs.sort((a, b) => a.vector_distance - b.vector_distance);

    expect(docs.map((d) => d._id)).toEqual(["doc3", "doc2"]);
  });

  test("vector_search_binary", async () => {
    await createCollection(ctx.scope("books"), {
      binary_embedding: binaryVector(3)
        .required()
        .index(vectorIndex({ metric: VectorDistanceMetric.Hamming })),
    });

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", binary_embedding: binaryVectorValue([0, 0, 1]) },
      { _id: "doc2", binary_embedding: binaryVectorValue([0, 1, 1]) },
      { _id: "doc3", binary_embedding: binaryVectorValue([1, 1, 1]) },
    ]);
    expect(lsn).toBe(1);

    let docs = await ctx.client.collection(ctx.scope("books")).query(
      select({
        vector_distance: vectorDistance("binary_embedding", binaryVectorValue([1, 1, 1])),
      }).top_k(field("vector_distance"), 2, true),
      lsn
    );
    docs.sort((a, b) => a.vector_distance - b.vector_distance);

    expect(docs.map((d) => d._id)).toEqual(["doc3", "doc2"]);
  });

  test("semantic_search", async () => {
    await createCollection(ctx.scope("books"), {
      title: text()
        .required()
        .index(semanticIndex({ model: "dummy" })),
    });

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", title: "red purple green" },
      { _id: "doc2", title: "yellow purple pink" },
      { _id: "doc3", title: "orange red blue" },
      { _id: "doc4", title: "green yellow purple" },
      { _id: "doc5", title: "pink orange green" },
      { _id: "doc6", title: "black green yellow" },
      { _id: "doc7", title: "purple pink orange" },
      { _id: "doc8", title: "green yello green" },
      { _id: "doc9", title: "yellow purple pink" },
    ]);
    expect(lsn).toBe(1);

    const docs = await ctx.client
      .collection(ctx.scope("books"))
      .query(
        select({ sim: semanticSimilarity("title", "redish") }).top_k(
          field("sim"),
          2
        ),
        lsn
      );

    // NOTE: since we are using the dummy model, the similarity score is randomized, so we can't check ordering
    expect(docs.length).toBe(2);
  });

  test("delete", async () => {
    await createCollection(ctx.scope("books"));

    let lsn = await ctx.client
      .collection(ctx.scope("books"))
      .upsert([{ _id: "doc1", name: "one" }]);
    expect(lsn).toBe(1);

    lsn = await ctx.client.collection(ctx.scope("books")).delete(["doc1"]);
    expect(lsn).toBe(2);

    const docs = await ctx.client.collection(ctx.scope("books")).query(
      select({ name: field("name") })
        .filter(field("name").eq("one"))
        .count(),
      lsn
    );
    expect(docs).toEqual([{ _count: 0 }]);
  });

  test("count", async () => {
    await createCollection(ctx.scope("books"));

    let lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", name: "one" },
      { _id: "doc2", name: "two" },
    ]);
    expect(lsn).toBe(1);

    let count = await ctx.client.collection(ctx.scope("books")).count(lsn);
    expect(count).toBe(2);

    lsn = await ctx.client.collection(ctx.scope("books")).delete(["doc1"]);
    expect(lsn).toBe(2);

    count = await ctx.client.collection(ctx.scope("books")).count(lsn);
    expect(count).toBe(1);
  });

  test("rerank", async () => {
    await createCollection(ctx.scope("books"), {
      summary: text()
        .required()
        .index(semanticIndex({ model: "dummy" })),
    });

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", summary: "book about a boy that goes to the woods" },
      { _id: "doc2", summary: "capitalism 101" },
      { _id: "doc3", summary: "walking into the sea" },
      { _id: "doc4", summary: "bears like to stay by the bushes" },
      { _id: "doc5", summary: "dad takes his son for a stroll in the nature" },
    ]);

    const docs = await ctx.client.collection(ctx.scope("books")).query(
      select({
        name: field("name"),
        summary_sim: semanticSimilarity("summary", "male walks around trees"),
      })
        .top_k(field("summary_sim"), 2)
        .rerank(),
      lsn
    );

    // NOTE: since we are using the dummy model, the similarity score is randomized, so we can't check ordering
    expect(docs.length).toBe(2);
  });
});
