import { MatrixValueType } from '../lib/data';
import {
  binaryVector,
  bool,
  bytes,
  f32Vector,
  float,
  int,
  keywordIndex,
  matrix,
  multiVectorIndex,
  ngramIndex,
  text,
  u8Vector,
  vectorIndex,
} from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

async function listCollections(
  ctx: ProjectContext,
  options?: { prefix?: string }
) {
  const collections = [];
  for await (const collection of ctx.client.collections().list(options)) {
    collections.push(collection);
  }
  return collections;
}

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

  test("list collections", async () => {
    const ctx = getContext();

    const a = await ctx.createCollection("books", {});
    const collections1 = await listCollections(ctx);
    expect(collections1).toContainEqual(a);

    const b = await ctx.createCollection("books2", {});
    const collections2 = await listCollections(ctx);
    expect(collections2).toContainEqual(a);
    expect(collections2).toContainEqual(b);

    const c = await ctx.createCollection("books3", {});

    const collections3 = await listCollections(ctx);
    expect(collections3).toContainEqual(a);
    expect(collections3).toContainEqual(b);
    expect(collections3).toContainEqual(c);
  });

  test("list collections with prefix", async () => {
    const ctx = getContext();

    const names = (
      await Promise.all(
        ["a", "ab", "b"].map((name) => ctx.createCollection(name, {}))
      )
    ).map((c) => c.name);

    for (const [prefix, expected] of [
      [ctx.scopePrefix, names],
      [ctx.scope("a"), names.slice(0, 2)],
    ] as const) {
      const collections = await listCollections(ctx, { prefix });
      expect(collections.map((c) => c.name)).toEqual(expected);
    }
  });

  test("list collections with invalid prefix", async () => {
    const ctx = getContext();

    await expect(listCollections(ctx, { prefix: "-books" })).rejects.toThrow(
      "Invalid collection name"
    );
  });

  test("get collection", async () => {
    const ctx = getContext();

    // Test getting non-existent collection
    await expect(
      ctx.client.collections().get(ctx.scope("test"))
    ).rejects.toThrow("collection not found");

    // Create collection
    const collection = await ctx.createCollection("test", {});

    // Get collection
    const retrievedCollection = await ctx.client
      .collections()
      .get(ctx.scope("test"));
    expect(retrievedCollection).toEqual(collection);
  });

  test("create collection", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});
    const collections = await listCollections(ctx);
    expect(collections).toContainEqual(collection);
  });

  test("collection schema with exact keyword index round trip", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      title: text().required().index(keywordIndex("exact")),
    });

    const fetched = await ctx.client.collections().get(collection.name);
    const collections = await listCollections(ctx);
    const listed = collections.find(({ name }) => name === collection.name);

    expect(collection.schema.title.index).toMatchObject({
      type: "KeywordIndex",
      indexType: "exact",
    });
    expect(fetched.schema.title.index).toMatchObject({
      type: "KeywordIndex",
      indexType: "exact",
    });
    expect(listed?.schema.title.index).toMatchObject({
      type: "KeywordIndex",
      indexType: "exact",
    });
  });

  test("collection schema with ngram index round trip", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      title: text().required().index(ngramIndex()),
    });

    const fetched = await ctx.client.collections().get(collection.name);
    const collections = await listCollections(ctx);
    const listed = collections.find(({ name }) => name === collection.name);

    expect(collection.schema.title.index).toMatchObject({
      type: "NGramIndex",
    });
    expect(fetched.schema.title.index).toMatchObject({
      type: "NGramIndex",
    });
    expect(listed?.schema.title.index).toMatchObject({
      type: "NGramIndex",
    });
  });

  test("create collection with invalid schema", async () => {
    const ctx = getContext();

    await expect(
      ctx.createCollection("books", {
        title: "invalid",
      })
    ).rejects.toThrow("Value must be a FieldSpec");
  });

  test("create duplicate collection", async () => {
    const ctx = getContext();
    await ctx.createCollection("test", {});

    await expect(
      ctx.client.collections().create(ctx.scope("test"), {})
    ).rejects.toThrow("collection already exists");
  });

  test("collection schema", async () => {
    const ctx = getContext();

    const schema = {
      text: text(),
      int: int(),
      float: float(),
      bool: bool(),
      vector: f32Vector({ dimension: 1536 }),
      float_vector: f32Vector({ dimension: 1536 }),
      byte_vector: u8Vector({ dimension: 1536 }),
      binary_vector: binaryVector({ dimension: 1536 }),
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

  test("matrix schema", async () => {
    const ctx = getContext();

    const matrixValueTypes: Array<[MatrixValueType, string]> = [
      ["f32", "f32"],
      ["u8", "u8"],
      ["i8", "i8"],
      ["f16", "f16"],
      ["f8", "f8"],
    ];

    for (const [valueType, expectedValueType] of matrixValueTypes) {
      const schema = {
        token_embeddings: matrix({ dimension: 7, valueType }).index(
          multiVectorIndex({ metric: "maxsim" })
        ),
      };
      const collection = await ctx.createCollection(
        `test_matrix_${valueType}`,
        schema
      );
      expect(collection.schema.token_embeddings.dataType.type).toBe("Matrix");
      expect(
        collection.schema.token_embeddings.dataType.type === "Matrix" &&
          collection.schema.token_embeddings.dataType.dimension
      ).toBe(7);
      expect(
        collection.schema.token_embeddings.dataType.type === "Matrix" &&
          collection.schema.token_embeddings.dataType.valueType
      ).toBe(expectedValueType);
      expect(collection.schema.token_embeddings.required).toBe(false);
    }
  });

  test("incorrect schema", async () => {
    const ctx = getContext();

    await expect(
      ctx.client.collections().create(ctx.scope("books"), {
        name: text().index(vectorIndex({ metric: "cosine" })),
      })
    ).rejects.toThrow(
      /InvalidIndex { field: \"name\", index: \"vector\", data_type: \"text\" }/
    );
  });

  test("delete collection", async () => {
    const ctx = getContext();

    const collectionsBeforeCreate = await listCollections(ctx);
    expect(collectionsBeforeCreate.map((c) => c.name)).not.toContain(
      ctx.scope("books")
    );

    await ctx.createCollection("books", {});

    const collectionsAfterCreate = await listCollections(ctx);
    expect(collectionsAfterCreate.map((c) => c.name)).toContain(
      ctx.scope("books")
    );

    await ctx.client.collections().delete(ctx.scope("books"));

    ctx.collectionsCreated = ctx.collectionsCreated.filter(
      (name) => name !== ctx.scope("books")
    );

    const collectionsAfterDelete = await listCollections(ctx);
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
