import * as data from "../lib/data";
import {
  binaryVector,
  bool,
  bytes,
  f32SparseVector,
  f32Vector,
  float,
  int,
  list,
  text,
  u8SparseVector,
  u8Vector,
  vectorIndex,
} from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Upsert", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("upsert to non-existent collection", async () => {
    const ctx = getContext();
    await expect(
      ctx.client.collection("missing").upsert([{ _id: "one" }])
    ).rejects.toThrow(/collection not found/);
  });

  test("upsert basic", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    const lsn = await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "one" }]);
    expect(lsn).toBe("1");
  });

  test("upsert batch", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    const lsn = await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "one" }, { _id: "two" }]);
    expect(lsn).toBe("1");
  });

  test("upsert sequential", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    let lsn = await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "one" }]);
    expect(lsn).toBe("1");

    lsn = await ctx.client.collection(collection.name).upsert([{ _id: "two" }]);
    expect(lsn).toBe("2");

    lsn = await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "three" }]);
    expect(lsn).toBe("3");
  });

  test("upsert with no documents", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    await expect(
      ctx.client.collection(collection.name).upsert([])
    ).rejects.toThrow(/NoDocuments/);
  });

  test("upsert with missing id field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    await expect(
      ctx.client.collection(collection.name).upsert([{}])
    ).rejects.toThrow(/MissingId { doc_offset: 0 }/);
  });

  test("upsert with missing name field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      name: text().required(),
    });

    await expect(
      ctx.client.collection(collection.name).upsert([{ _id: "one" }])
    ).rejects.toThrow(/MissingField { doc_id: \"one\", field: \"name\" }/);
  });

  test("upsert max doc size", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    try {
      await ctx.client.collection(collection.name).upsert([
        { _id: "one", payload: "x".repeat(500 * 1024) }, // 500KB, too large
        { _id: "two", payload: "xxx" }, // ok
        { _id: "three", payload: "x".repeat(130 * 1024) }, // 130KB, too large
        { _id: "four", payload: "x".repeat(126 * 1024) }, // 126KB (plus overhead), ok
      ]);
      fail("expected DocumentValidationError");
    } catch (e: any) {
      const s = String(e);
      expect((s.match(/DocumentTooLarge/g) || []).length).toBe(2);
      expect(s).toMatch(/doc_id: \"one\"/);
      expect(s).toMatch(/doc_id: \"three\"/);
      expect(s).not.toMatch(/doc_id: \"two\"/);
      expect(s).not.toMatch(/doc_id: \"four\"/);
      expect(s).toMatch(/max_size_bytes: 131072/);
    }
  });

  test("upsert with invalid document - null field value", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
      title: text().required(),
      f32_embedding: f32Vector({ dimension: 3 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await expect(
      ctx.client
        .collection(ctx.scope("books"))
        .upsert([{ _id: "doc1", title: null, f32_embedding: [1, 2, 3] }])
    ).rejects.toThrow(
      /TopkError\(DocumentValidationError\(ValidationErrorBag\(\[MissingField \{ doc_id: \"doc1\", field: \"title\" \}\]\)\)\)/
    );
  });

  test("upsert with invalid document - missing required field", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
      title: text().required(),
      f32_embedding: f32Vector({ dimension: 3 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await expect(
      ctx.client
        .collection(ctx.scope("books"))
        .upsert([{ _id: "doc1", f32_embedding: [1, 2, 3] }])
    ).rejects.toThrow(/MissingField { doc_id: \"doc1\", field: \"title\" }/);
  });

  test("upsert with invalid document - wrong vector dimension", async () => {
    const ctx = getContext();

    await ctx.createCollection("books", {
      title: text().required(),
      f32_embedding: f32Vector({ dimension: 3 })
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
    });

    await expect(
      ctx.client
        .collection(ctx.scope("books"))
        .upsert([{ _id: "doc1", title: "one", f32_embedding: [1, 2] }])
    ).rejects.toThrow(
      /InvalidVectorDimension { doc_id: \"doc1\", field: \"f32_embedding\", expected_dimension: 3, got_dimension: 2 }/
    );
  });

  test.each([
    [true, bool()],
    ["hello", text()],
    [1, int()],
    [1.1, float()],
    [Buffer.from("hello"), bytes()],
  ])("upsert primitives - %s", async (value, dataType) => {
    const ctx = getContext();

    await ctx.createCollection("test", {
      field: dataType,
    });

    const lsn = await ctx.client
      .collection(ctx.scope("test"))
      .upsert([{ _id: "x", field: value }]);

    const obj = await ctx.client
      .collection(ctx.scope("test"))
      .get(["x"], null, { lsn });

    expect(obj["x"].field).toEqual(value);
  });

  test("upsert vectors", async () => {
    const ctx = getContext();

    await ctx.createCollection("test", {
      f32_vector: f32Vector({ dimension: 3 }),
      u8_vector: u8Vector({ dimension: 3 }),
      binary_vector: binaryVector({ dimension: 3 }),
    });

    const lsn = await ctx.client.collection(ctx.scope("test")).upsert([
      {
        _id: "x",
        f32_vector: [1, 2, 3],
        u8_vector: data.u8Vector([4, 5, 6]),
        binary_vector: data.binaryVector([7, 8, 9]),
      },
    ]);

    const obj = await ctx.client
      .collection(ctx.scope("test"))
      .get(["x"], null, { lsn });

    expect(obj["x"].f32_vector).toEqual([1, 2, 3]);
    expect(obj["x"].u8_vector).toEqual([4, 5, 6]);
    expect(obj["x"].binary_vector).toEqual([7, 8, 9]);
  });

  test("upsert sparse vectors", async () => {
    const ctx = getContext();

    await ctx.createCollection("test", {
      f32_sparse_vector: f32SparseVector(),
      u8_sparse_vector: u8SparseVector(),
    });

    const lsn = await ctx.client.collection(ctx.scope("test")).upsert([
      {
        _id: "x",
        f32_sparse_vector: { 1: 1.2, 2: 2.3, 3: 3.4 },
        u8_sparse_vector: data.u8SparseVector({ 1: 4, 2: 5, 3: 6 }),
      },
    ]);

    const obj = await ctx.client
      .collection(ctx.scope("test"))
      .get(["x"], null, { lsn });

    expect(normalizeF32SparseValues(obj["x"].f32_sparse_vector)).toEqual({
      1: 1.2,
      2: 2.3,
      3: 3.4,
    });
    expect(obj["x"].u8_sparse_vector).toEqual({ 1: 4, 2: 5, 3: 6 });
  });

  test("upsert empty float list", async () => {
    const ctx = getContext();

    await ctx.createCollection("test", {
      f32_list: list({ valueType: "float" }),
    });

    const lsn = await ctx.client.collection(ctx.scope("test")).upsert([
      { _id: "x", f32_list: [] },
    ]);

    const obj = await ctx.client
      .collection(ctx.scope("test"))
      .get(["x"], null, { lsn });

    expect(obj["x"].f32_list).toEqual([]);
  });

  test("upsert empty float list with helper", async () => {
    const ctx = getContext();

    await ctx.createCollection("test", {
      f32_list: list({ valueType: "float" }),
    });

    const lsn = await ctx.client.collection(ctx.scope("test")).upsert([
      { _id: "x", f32_list: data.f32List([]) },
    ]);

    const obj = await ctx.client
      .collection(ctx.scope("test"))
      .get(["x"], null, { lsn });

    expect(obj["x"].f32_list).toEqual([]);
  });

  test("upsert empty integer list raises error", async () => {
    const ctx = getContext();

    await ctx.createCollection("test", {
      i32_list: list({ valueType: "integer" }),
    });

    await expect(
      ctx.client.collection(ctx.scope("test")).upsert([
        { _id: "x", i32_list: [] },
      ])
    ).rejects.toThrow(/field: "i32_list", expected_type: "list<integer>", got_value: "list<f32>"/);
  });

  test("upsert empty integer list with helper", async () => {
    const ctx = getContext();

    await ctx.createCollection("test", {
      i32_list: list({ valueType: "integer" }),
    });

    const lsn = await ctx.client.collection(ctx.scope("test")).upsert([
      { _id: "x", i32_list: data.i32List([]) },
    ]);

    const obj = await ctx.client
      .collection(ctx.scope("test"))
      .get(["x"], null, { lsn });

    expect(obj["x"].i32_list).toEqual([]);
  });

  test("upsert empty string list raises error", async () => {
    const ctx = getContext();

    await ctx.createCollection("test", {
      string_list: list({ valueType: "text" }),
    });

    await expect(
      ctx.client.collection(ctx.scope("test")).upsert([
        { _id: "x", string_list: [] },
      ])
    ).rejects.toThrow(/field: "string_list", expected_type: "list<string>", got_value: "list<f32>"/);
  });

  test("upsert empty string list with helper", async () => {
    const ctx = getContext();

    await ctx.createCollection("test", {
      string_list: list({ valueType: "text" }),
    });

    const lsn = await ctx.client.collection(ctx.scope("test")).upsert([
      { _id: "x", string_list: data.stringList([]) },
    ]);

    const obj = await ctx.client
      .collection(ctx.scope("test"))
      .get(["x"], null, { lsn });

    expect(obj["x"].string_list).toEqual([]);
  });
});

function normalizeF32SparseValues(obj: Record<string, number>, precision = 5) {
  const factor = 10 ** precision;
  return Object.fromEntries(
    Object.entries(obj).map(([k, v]) => [k, Math.round(v * factor) / factor])
  );
}
