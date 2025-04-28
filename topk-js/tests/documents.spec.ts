import {
  binaryVector as binaryVectorValue,
  u8Vector as u8VectorValue,
} from "../lib/data";
import { field, fn, match, select } from "../lib/query";
import {
  binaryVector,
  f32Vector,
  keywordIndex,
  semanticIndex,
  text,
  u8Vector,
  vectorIndex,
} from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Documents", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("get", async () => {
    const ctx = getContext();

    await ctx.createCollection("books");

    // upsert document
    const lsn = await ctx.client
      .collection(ctx.scope("books"))
      .upsert([{ _id: "one", name: "one", rank: 1 }]);
    expect(lsn).toBe("1");

    // get document
    const doc = await ctx.client.collection(ctx.scope("books")).get(["one"]);

    expect(doc).toEqual({ one: { _id: "one", name: "one", rank: 1 } });
  });

  test("get non-existent document", async () => {
    const ctx = getContext();

    await ctx.createCollection("books");

    const docs = await ctx.client.collection(ctx.scope("books")).get(["one"]);
    await expect(docs).toEqual({});
  });

  test("upsert", async () => {
    const ctx = getContext();

    await ctx.createCollection("books");

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "one", name: "one", rank: 1 },
      { _id: "two", name: "two", extra: 1, rank: 2 },
    ]);
    expect(lsn).toBe("1");

    const docs = await ctx.client.collection(ctx.scope("books")).query(
      select({ name: field("name") })
        .filter(field("name").eq("two"))
        .topk(field("rank"), 10),
      { lsn }
    );

    expect(docs).toEqual([{ _id: "two", name: "two" }]);
  });

  test("keyword_search", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
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
    expect(lsn).toBe("1");

    const docs = await ctx.client.collection(ctx.scope("books")).query(
      select({
        text_score: fn.bm25Score(),
      })
        .filter(match("red").or(match("blue")))
        .topk(field("text_score"), 5),
      { lsn }
    );

    const docIds = new Set(docs.map((d) => d._id));
    expect(docIds).toEqual(new Set(["doc1", "doc10", "doc3", "doc5", "doc8"]));
  });

  test("vector_search_f32", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
      f32_embedding: f32Vector({ dimension: 3 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", f32_embedding: [1.0, 2.0, 3.0] },
      { _id: "doc2", f32_embedding: [4.0, 5.0, 6.0] },
      { _id: "doc3", f32_embedding: [7.0, 8.0, 9.0] },
    ]);
    expect(lsn).toBe("1");

    let docs = await ctx.client.collection(ctx.scope("books")).query(
      select({
        vector_distance: fn.vectorDistance("f32_embedding", [7.0, 8.0, 9.0]),
      }).topk(field("vector_distance"), 2, true),
      { lsn }
    );
    docs.sort((a, b) => a.vector_distance - b.vector_distance);

    expect(docs.map((d) => d._id)).toEqual(["doc3", "doc2"]);
  });

  test("vector_search_u8", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
      u8_embedding: u8Vector({ dimension: 3 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", u8_embedding: u8VectorValue([1, 2, 3]) },
      { _id: "doc2", u8_embedding: u8VectorValue([4, 5, 6]) },
      { _id: "doc3", u8_embedding: u8VectorValue([7, 8, 9]) },
    ]);
    expect(lsn).toBe("1");

    let docs = await ctx.client.collection(ctx.scope("books")).query(
      select({
        vector_distance: fn.vectorDistance(
          "u8_embedding",
          u8VectorValue([7, 8, 9])
        ),
      }).topk(field("vector_distance"), 2, true),
      { lsn }
    );
    docs.sort((a, b) => a.vector_distance - b.vector_distance);

    expect(docs.map((d) => d._id)).toEqual(["doc3", "doc2"]);
  });

  test("vector_search_binary", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
      binary_embedding: binaryVector({ dimension: 3 })
        .required()
        .index(vectorIndex({ metric: "hamming" })),
    });

    const lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", binary_embedding: binaryVectorValue([0, 0, 1]) },
      { _id: "doc2", binary_embedding: binaryVectorValue([0, 1, 1]) },
      { _id: "doc3", binary_embedding: binaryVectorValue([1, 1, 1]) },
    ]);
    expect(lsn).toBe("1");

    let docs = await ctx.client.collection(ctx.scope("books")).query(
      select({
        vector_distance: fn.vectorDistance(
          "binary_embedding",
          binaryVectorValue([1, 1, 1])
        ),
      }).topk(field("vector_distance"), 2, true),
      { lsn }
    );
    docs.sort((a, b) => a.vector_distance - b.vector_distance);

    expect(docs.map((d) => d._id)).toEqual(["doc3", "doc2"]);
  });

  test("semantic_search", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
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
    expect(lsn).toBe("1");

    const docs = await ctx.client
      .collection(ctx.scope("books"))
      .query(
        select({ sim: fn.semanticSimilarity("title", "redish") }).topk(
          field("sim"),
          2
        ),
        { lsn }
      );

    // NOTE: since we are using the dummy model, the similarity score is randomized, so we can't check ordering
    expect(docs.length).toBe(2);
  });

  test("delete", async () => {
    const ctx = getContext();

    await ctx.createCollection("books");

    let lsn = await ctx.client
      .collection(ctx.scope("books"))
      .upsert([{ _id: "doc1", name: "one" }]);
    expect(lsn).toBe("1");

    lsn = await ctx.client.collection(ctx.scope("books")).delete(["doc1"]);
    expect(lsn).toBe("2");

    const docs = await ctx.client.collection(ctx.scope("books")).query(
      select({ name: field("name") })
        .filter(field("name").eq("one"))
        .count(),
      { lsn }
    );
    expect(docs).toEqual([{ _count: 0 }]);
  });

  test("count", async () => {
    const ctx = getContext();

    await ctx.createCollection("books");

    let lsn = await ctx.client.collection(ctx.scope("books")).upsert([
      { _id: "doc1", name: "one" },
      { _id: "doc2", name: "two" },
    ]);
    expect(lsn).toBe("1");

    let count = await ctx.client.collection(ctx.scope("books")).count({ lsn });
    expect(count).toBe(2);

    lsn = await ctx.client.collection(ctx.scope("books")).delete(["doc1"]);
    expect(lsn).toBe("2");

    count = await ctx.client.collection(ctx.scope("books")).count({ lsn });
    expect(count).toBe(1);
  });

  test("rerank", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
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
        summary_sim: fn.semanticSimilarity(
          "summary",
          "male walks around trees"
        ),
      })
        .topk(field("summary_sim"), 2)
        .rerank(),
      { lsn }
    );

    // NOTE: since we are using the dummy model, the similarity score is randomized, so we can't check ordering
    expect(docs.length).toBe(2);
  });
});
