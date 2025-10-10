import { field, select } from "../lib/query";
import { newProjectContext, ProjectContext } from "./setup";

describe("delete", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  it("should throw error when deleting from non-existent collection", async () => {
    const ctx = getContext();
    const collection = ctx.client.collection("non-existent");
    await expect(collection.delete(["doc1"])).rejects.toThrow(
      "collection not found"
    );
  });

  it("should delete existing document", async () => {
    const ctx = getContext();

    const collection = await ctx.createCollection("books", {});

    await ctx.client.collection(collection.name).upsert([
      { _id: "doc1", title: "Book 1", published_year: 1950 },
      { _id: "doc2", title: "Book 2", published_year: 1960 },
    ]);

    const lsn = await ctx.client.collection(collection.name).delete(["doc1"]);
    expect(lsn).toBe("2");

    const remaining = await ctx.client
      .collection(collection.name)
      .query(select({ _id: field("_id") }).limit(100));

    expect(remaining).toEqual([{ _id: "doc2" }]);
  });

  it("should delete existing document with filter", async () => {
    const ctx = getContext();

    const collection = await ctx.createCollection("books", {});

    await ctx.client.collection(collection.name).upsert([
      { _id: "doc1", title: "Book 1", published_year: 1950 },
      { _id: "doc2", title: "Book 2", published_year: 1960 },
    ]);

    const lsn = await ctx.client.collection(collection.name).delete(field("published_year").lt(1955));
    expect(lsn).toBe("2");

    const remaining = await ctx.client
      .collection(collection.name)
      .query(select({ _id: field("_id") }).limit(100));

    expect(remaining).toEqual([{ _id: "doc2" }]);
  });

  it("should ignore non-existent document", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {});
    const lsn = await ctx.client
      .collection(collection.name)
      .delete(["non-existent"]);
    expect(lsn).toBe("1");
  });
});
