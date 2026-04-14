import { newProjectContext, ProjectContext } from "./setup";

describe("Context Streams", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.cleanup()));
  });

  test("askStream implements async iterable protocol and validates datasets", async () => {
    const ctx = getContext();
    const stream = ctx.client.askStream("summarize", [], undefined, "research");

    const iter = stream[Symbol.asyncIterator]();
    expect(typeof iter.next).toBe("function");
    expect(typeof iter.return).toBe("function");
    await expect(iter.next()).rejects.toThrow(/provide at least one dataset/i);
  });

  test("searchStream implements async iterable protocol and validates datasets", async () => {
    const ctx = getContext();
    const stream = ctx.client.searchStream("summarize", [], 3);

    const iter = stream[Symbol.asyncIterator]();
    expect(typeof iter.next).toBe("function");
    expect(typeof iter.return).toBe("function");
    await expect(iter.next()).rejects.toThrow(/provide at least one dataset/i);
  });
});
