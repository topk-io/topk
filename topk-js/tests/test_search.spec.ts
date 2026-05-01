import * as path from "node:path";

import { newProjectContext, ProjectContext } from "./setup";

describe("Search", () => {
  const contexts: ProjectContext[] = [];
  const pdfPath = path.join(__dirname, "..", "..", "tests", "pdfko.pdf");

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.cleanup()));
  });

  test("search yields results", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const response = await ctx.client
      .dataset(dataset.name)
      .upsertFile("doc1", { path: pdfPath }, {});
    await ctx.client.dataset(dataset.name).waitForHandle(response.handle);

    const stream = ctx.client.search("technical", [dataset.name], 10);

    const results = [];
    for await (const result of stream) {
      results.push(result);
    }

    expect(results.length).toBeGreaterThan(0);
    expect(results.every((r) => typeof r.docId === "string")).toBe(true);
  }, 1000 * 60 * 6); // 6 minutes
});
