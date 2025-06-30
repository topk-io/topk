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
});
