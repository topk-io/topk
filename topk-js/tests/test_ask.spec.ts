import * as path from "node:path";

import type { Answer, Reason, Search } from "../index.js";
import { newProjectContext, ProjectContext } from "./setup";

/** Stream chunks are plain objects from native code, not class instances — use shape guards. */
function isReasonMessage(message: Answer | Search | Reason): message is Reason {
  return "thought" in message;
}

function isSearchMessage(message: Answer | Search | Reason): message is Search {
  return "objective" in message;
}

function isAnswerMessage(message: Answer | Search | Reason): message is Answer {
  return "facts" in message && !isSearchMessage(message) && !isReasonMessage(message);
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

  test.failing("ask returns an answer", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const response = await ctx.client
      .dataset(dataset.name)
      .upsertFile("doc1", { path: pdfPath }, {});
    await ctx.client.dataset(dataset.name).waitForHandle(response.handle);

    const result = await ctx.client.ask("summarize", [dataset.name]);

    expect(isAnswerMessage(result)).toBe(true);
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
      if (isAnswerMessage(message)) {
        answerReceived = true;
        break;
      }
    }

    expect(answerReceived).toBe(true);
  });
});
