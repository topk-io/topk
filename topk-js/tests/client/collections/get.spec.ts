import { newProjectContext, ProjectContext } from "../../setup";

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
