import * as path from "node:path";

import type { Answer, Progress } from "../index.js";
import { newProjectContext, ProjectContext } from "./setup";

function isProgressMessage(message: Answer | Progress): message is Progress {
  return "update" in message;
}

function isAnswerMessage(message: Answer | Progress): message is Answer {
  return "facts" in message && !isProgressMessage(message);
}

async function extractAnswer(
  stream: AsyncIterable<Answer | Progress>
): Promise<Answer | null> {
  for await (const message of stream) {
    if (isAnswerMessage(message)) return message;
  }
  return null;
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

  test("ask yields an answer", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const handle = await ctx.client
      .dataset(dataset.name)
      .upsertFile("doc1", { path: pdfPath }, {});

    await ctx.client.dataset(dataset.name).waitForHandle(handle);

    const answer = await extractAnswer(ctx.client.ask("summarize", [dataset.name]));

    expect(answer).not.toBeNull();
  }, 1000 * 60 * 6); // 6 minutes
});
