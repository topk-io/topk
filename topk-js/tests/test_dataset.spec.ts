import * as path from "node:path";

import { newProjectContext, ProjectContext } from "./setup";

describe("Dataset", () => {
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

  test("upsert file to non-existent dataset", async () => {
    const ctx = getContext();

    await expect(
      ctx.client.dataset(ctx.scope("missing")).upsertFile("doc1", { path: pdfPath }, {})
    ).rejects.toThrow(/dataset not found/i);
  });

  test("upsert file from path", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const response = await ctx.client
      .dataset(dataset.name)
      .upsertFile("doc1", { path: pdfPath }, { title: "test" });

    expect(response.handle.length).toBeGreaterThan(0);
  });

  test("upsert file from inline bytes", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const response = await ctx.client.dataset(dataset.name).upsertFile(
      "doc1",
      {
        data: Buffer.from("# Test Document\n\nThis is a test markdown file."),
        fileName: "test.md",
        mimeType: "text/markdown",
      },
      { title: "test markdown" }
    );

    expect(response.handle.length).toBeGreaterThan(0);
  });

  test("update metadata", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    await ctx.client.dataset(dataset.name).upsertFile("doc1", { path: pdfPath }, {});

    const response = await ctx.client
      .dataset(dataset.name)
      .updateMetadata("doc1", { title: "Updated Title" });

    expect(response.handle.length).toBeGreaterThan(0);
  });

  test("delete document", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    await ctx.client.dataset(dataset.name).upsertFile("doc1", { path: pdfPath }, {});

    const response = await ctx.client.dataset(dataset.name).delete("doc1");
    expect(response.handle.length).toBeGreaterThan(0);
  });

  test("check handle", async () => {
    const ctx = getContext();
    const dataset = await ctx.createDataset("test");

    const response = await ctx.client
      .dataset(dataset.name)
      .upsertFile("doc1", { path: pdfPath }, {});

    await expect(
      ctx.client.dataset(dataset.name).checkHandle(response.handle)
    ).resolves.toBe(false);
  });

  test("listStream implements AsyncIterator", async () => {
    const ctx = getContext();
    const stream = ctx.client.dataset(ctx.scope("missing")).listStream();

    // napi-rs async iterators return a dedicated iterator from @@asyncIterator (not `this`).
    expect(typeof stream[Symbol.asyncIterator]).toBe("function");
    const iter = stream[Symbol.asyncIterator]();
    expect(typeof iter.next).toBe("function");
    await expect(iter.next()).rejects.toThrow(/dataset not found/i);
  });
});
