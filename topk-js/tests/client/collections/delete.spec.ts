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
