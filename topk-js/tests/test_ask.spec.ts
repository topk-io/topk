import * as path from "node:path";

import type { Answer, Progress } from "../index.js";
import { newProjectContext, ProjectContext } from "./setup";

function isProgressMessage(message: Answer | Progress): message is Progress {
  return "update" in message;
}

function isAnswerMessage(message: Answer | Progress): message is Answer {
  return "facts" in message && !isProgressMessage(message);
}

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

  test("ask returns an answer", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const response = await ctx.client
      .dataset(dataset.name)
      .upsertFile("doc1", { path: pdfPath }, {});
    await ctx.client.dataset(dataset.name).waitForHandle(response.handle);

    const result = await ctx.client.ask("summarize", [dataset.name]);

    expect(isAnswerMessage(result)).toBe(true);
  }, 1000 * 60 * 6); // 6 minutes

  test("askStream yields an answer", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const response = await ctx.client
      .dataset(dataset.name)
      .upsertFile("doc1", { path: pdfPath }, {});
    await ctx.client.dataset(dataset.name).waitForHandle(response.handle);

    const stream = ctx.client.askStream("summarize", [dataset.name]);

    let answerReceived = false;
    for await (const message of stream) {
      if (isAnswerMessage(message)) {
        answerReceived = true;
        break;
      }
    }

    expect(answerReceived).toBe(true);
  }, 1000 * 60 * 6); // 6 minutes
});
