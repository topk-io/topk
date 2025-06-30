import { field, select } from "../../lib/query";
import { int, keywordIndex, text } from "../../lib/schema";
import { newProjectContext, ProjectContext } from "../setup";

describe("Query Validation", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("query topk by non primitive", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1", title: "Book 1", published_year: 2020 },
      { _id: "2", title: "Book 2", published_year: 2021 },
      { _id: "3", title: "Book 3", published_year: 2022 },
    ]);

    await expect(
      ctx.client
        .collection(collection.name)
        .query(select({}).topk(field("title"), 3, true))
    ).rejects.toThrow();
  });

  test("query topk by non existing", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1", title: "Book 1", published_year: 2020 },
      { _id: "2", title: "Book 2", published_year: 2021 },
      { _id: "3", title: "Book 3", published_year: 2022 },
    ]);

    await expect(
      ctx.client
        .collection(collection.name)
        .query(select({}).topk(field("non_existing_field"), 3, true))
    ).rejects.toThrow();
  });

  test("query topk limit zero", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await expect(
      ctx.client
        .collection(collection.name)
        .query(select({}).topk(field("published_year"), 0, true))
    ).rejects.toThrow();
  });

  test("union u32 and binary", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {});

    const lsn = await ctx.client.collection(collection.name).upsert([
      { _id: "1", num: 1 },
      { _id: "11", num: new Uint8Array([1, 2, 3]) },
    ]);

    await ctx.client.collection(collection.name).count({ lsn });

    await expect(
      ctx.client
        .collection(collection.name)
        .query(select({}).topk(field("num"), 100, true))
    ).rejects.toThrow();
  });
});
