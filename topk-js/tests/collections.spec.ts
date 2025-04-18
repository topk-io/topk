import {
  binaryVector,
  bool,
  bytes,
  f32Vector,
  float,
  int,
  semanticIndex,
  text,
  u8Vector,
  vectorIndex,
} from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Collections", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("create collection", async () => {
    const ctx = getContext();

    const schema = {
      title: text(),
      title_embedding: f32Vector(1536)
        .required()
        .index(vectorIndex({ metric: "euclidean" })),
      summary: text()
        .required()
        .index(semanticIndex({ model: "dummy", embeddingType: "float32" })),
      published_year: int().required(),
    };

    const collection = await ctx.createCollection("books", schema);

    expect(collection.name).toBe(ctx.scope("books"));

    expect(collection.schema.title.dataType.type).toBe("Text");
    expect(collection.schema.title.required).toBe(false);

    expect(collection.schema.title_embedding.dataType.type).toBe("F32Vector");
    expect(
      collection.schema.title_embedding.dataType.type === "F32Vector" &&
        collection.schema.title_embedding.dataType.dimension
    ).toBe(1536);
    expect(collection.schema.title_embedding.required).toBe(true);
    expect(collection.schema.title_embedding.index.type).toBe("VectorIndex");
    expect(
      collection.schema.title_embedding.index.type === "VectorIndex" &&
        collection.schema.title_embedding.index.metric
    ).toBe("euclidean");

    expect(collection.schema.summary.dataType.type).toBe("Text");
    expect(collection.schema.summary.required).toBe(true);

    expect(collection.schema.published_year.dataType.type).toBe("Integer");
    expect(collection.schema.published_year.required).toBe(true);
  });

  test("create collection with all data types", async () => {
    const ctx = getContext();

    const schema = {
      text: text(),
      int: int(),
      float: float(),
      bool: bool(),
      vector: f32Vector(1536),
      float_vector: f32Vector(1536),
      byte_vector: u8Vector(1536),
      binary_vector: binaryVector(1536),
      bytes: bytes(),
    };

    const collection = await ctx.createCollection("books", schema);

    expect(collection.name).toBe(ctx.scope("books"));

    expect(collection.schema.text.dataType.type).toBe("Text");
    expect(collection.schema.text.required).toBe(false);

    expect(collection.schema.int.dataType.type).toBe("Integer");
    expect(collection.schema.int.required).toBe(false);

    expect(collection.schema.float.dataType.type).toBe("Float");
    expect(collection.schema.float.required).toBe(false);

    expect(collection.schema.bool.dataType.type).toBe("Boolean");
    expect(collection.schema.bool.required).toBe(false);

    expect(collection.schema.vector.dataType.type).toBe("F32Vector");
    expect(
      collection.schema.vector.dataType.type === "F32Vector" &&
        collection.schema.vector.dataType.dimension
    ).toBe(1536);
    expect(collection.schema.vector.required).toBe(false);

    expect(collection.schema.float_vector.dataType.type).toBe("F32Vector");
    expect(
      collection.schema.float_vector.dataType.type === "F32Vector" &&
        collection.schema.float_vector.dataType.dimension
    ).toBe(1536);
    expect(collection.schema.float_vector.required).toBe(false);

    expect(collection.schema.byte_vector.dataType.type).toBe("U8Vector");
    expect(
      collection.schema.byte_vector.dataType.type === "U8Vector" &&
        collection.schema.byte_vector.dataType.dimension
    ).toBe(1536);
    expect(collection.schema.byte_vector.required).toBe(false);

    expect(collection.schema.binary_vector.dataType.type).toBe("BinaryVector");
    expect(
      collection.schema.binary_vector.dataType.type === "BinaryVector" &&
        collection.schema.binary_vector.dataType.dimension
    ).toBe(1536);
    expect(collection.schema.binary_vector.required).toBe(false);

    expect(collection.schema.bytes.dataType.type).toBe("Bytes");
    expect(collection.schema.bytes.required).toBe(false);
  });

  test("incorrect schema", async () => {
    const ctx = getContext();

    await expect(
      ctx.client.collections().create(ctx.scope("books"), {
        name: text().index(vectorIndex({ metric: "cosine" })),
      })
    ).rejects.toThrow("invalid collection schema");
  });

  test("list collections", async () => {
    const ctx = getContext();

    const a = await ctx.createCollection("books", {});
    const collections1 = await ctx.client.collections().list();
    expect(collections1).toContainEqual(a);

    const b = await ctx.createCollection("books2", {});
    const collections2 = await ctx.client.collections().list();
    expect(collections2).toContainEqual(a);
    expect(collections2).toContainEqual(b);

    const c = await ctx.createCollection("books3", {});
    const collections3 = await ctx.client.collections().list();
    expect(collections3).toContainEqual(a);
    expect(collections3).toContainEqual(b);
    expect(collections3).toContainEqual(c);
  });

  test("get collections", async () => {
    const ctx = getContext();

    // get non-existent collection
    await expect(
      ctx.client.collections().get(ctx.scope("foo"))
    ).rejects.toThrow();

    // create collection
    await ctx.createCollection("foo", {});

    // get collection
    const collection = await ctx.client.collections().get(ctx.scope("foo"));

    // assert collection
    expect(collection.name).toBe(ctx.scope("foo"));
  });

  test("delete collection", async () => {
    const ctx = getContext();

    const collectionsBeforeCreate = await ctx.client.collections().list();
    expect(collectionsBeforeCreate.map((c) => c.name)).not.toContain(
      ctx.scope("books")
    );

    await ctx.createCollection("books", {});

    const collectionsAfterCreate = await ctx.client.collections().list();
    expect(collectionsAfterCreate.map((c) => c.name)).toContain(
      ctx.scope("books")
    );

    await ctx.client.collections().delete(ctx.scope("books"));

    ctx.collectionsCreated = ctx.collectionsCreated.filter(
      (name) => name !== ctx.scope("books")
    );

    const collectionsAfterDelete = await ctx.client.collections().list();
    expect(collectionsAfterDelete.map((c) => c.name)).not.toContain(
      ctx.scope("books")
    );
  });

  test("delete non-existent collection", async () => {
    const ctx = getContext();

    await expect(
      ctx.client.collections().delete(ctx.scope("books"))
    ).rejects.toThrow("collection not found");
  });
});
