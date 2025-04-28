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

  test("list collections", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});
    const collections = await ctx.client.collections().list();
    expect(collections).toContainEqual(collection);
  });

  test("create collection", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});
    const collections = await ctx.client.collections().list();
    expect(collections).toContainEqual(collection);
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

  test("delete non-existent collection", async () => {
    const ctx = getContext();
    await expect(
      ctx.client.collections().delete(ctx.scope("test"))
    ).rejects.toThrow("collection not found");
  });

  test("delete collection", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});
    await ctx.client.collections().delete(ctx.scope("test"));
    ctx.collectionsCreated = ctx.collectionsCreated.filter(
      (name) => name !== ctx.scope("test")
    );

    const collections = await ctx.client.collections().list();
    expect(collections).not.toContainEqual(collection);
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
});
