import { newProjectContext, ProjectContext } from "./setup";

describe("Datasets", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.cleanup()));
  });

  test("list datasets", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const datasets = await ctx.client.datasets().list();
    expect(datasets).toContainEqual(dataset);
  });

  test("get dataset", async () => {
    const ctx = getContext();

    await expect(ctx.client.datasets().get(ctx.scope("missing"))).rejects.toThrow(
      /dataset not found/i
    );

    const dataset = await ctx.createDataset("test");
    await expect(ctx.client.datasets().get(dataset.name)).resolves.toEqual(dataset);
  });

  test("create duplicate dataset", async () => {
    const ctx = getContext();
    await ctx.createDataset("test");

    await expect(ctx.client.datasets().create(ctx.scope("test"))).rejects.toThrow(
      /dataset already exists/i
    );
  });

  test("delete non-existent dataset", async () => {
    const ctx = getContext();

    await expect(ctx.client.datasets().delete(ctx.scope("missing"))).rejects.toThrow(
      /dataset not found/i
    );
  });

  test("delete dataset", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    await ctx.client.datasets().delete(dataset.name);
    ctx.datasetsCreated = ctx.datasetsCreated.filter((name) => name !== dataset.name);

    const datasets = await ctx.client.datasets().list();
    expect(datasets).not.toContainEqual(dataset);
  });
});
