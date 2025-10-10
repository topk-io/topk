import { field, select } from "../lib/query";
import { int, keywordIndex, text } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

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

  test("query sort limit by non primitive", async () => {
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
        .query(select({}).sort(field("title"), true).limit(3))
    ).rejects.toThrow();
  });

  test("query sort limit by non existing", async () => {
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
        .query(select({}).sort(field("non_existing_field"), true).limit(3))
    ).rejects.toThrow();
  });

  test("query sort limit zero", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await expect(
      ctx.client
        .collection(collection.name)
        .query(select({}).sort(field("published_year"), true).limit(0))
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
        .query(select({}).sort(field("num"), true).limit(100))
    ).rejects.toThrow();
  });
});
