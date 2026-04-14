import * as path from "node:path";

import { newProjectContext, ProjectContext } from "./setup";

describe("Ask", () => {
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

  test.failing("ask returns an answer with facts", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const response = await ctx.client
      .dataset(dataset.name)
      .upsertFile("doc1", { path: pdfPath }, {});
    await ctx.client.dataset(dataset.name).waitForHandle(response.handle);

    const result = await ctx.client.ask("summarize", [dataset.name]);

    expect(result.facts.length).toBeGreaterThan(0);
  });

  test.failing("askStream yields an answer", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const response = await ctx.client
      .dataset(dataset.name)
      .upsertFile("doc1", { path: pdfPath }, {});
    await ctx.client.dataset(dataset.name).waitForHandle(response.handle);

    const stream = ctx.client.askStream("summarize", [dataset.name]);

    let answerReceived = false;
    for await (const message of stream) {
      // Answer has `facts` but no `objective`; Search has `objective`; Reason has `thought`
      if ("facts" in message && !("objective" in message)) {
        answerReceived = true;
        break;
      }
    }

    expect(answerReceived).toBe(true);
  });
});
