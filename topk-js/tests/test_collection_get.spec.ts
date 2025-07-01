import { int, keywordIndex, text } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Get", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("get from non existent collection", async () => {
    const ctx = getContext();
    await expect(
      ctx.client.collection("missing").get(["doc1"])
    ).rejects.toThrow();
  });

  test("get non existent document", async () => {
    const ctx = getContext();

    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", title: "The Catcher in the Rye", published_year: 1951 },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925 },
      { _id: "moby", title: "Moby Dick", published_year: 1851 },
    ]);

    let docs = await ctx.client.collection(collection.name).get(["missing"]);
    expect(docs).toEqual({});
  });

  test("get document", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    const expectedDoc = {
      _id: "lotr",
      title: "The Lord of the Rings: The Fellowship of the Ring",
      published_year: 1954,
    };

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "catcher",
        title: "The Catcher in the Rye",
        published_year: 1951,
      },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925 },
      { _id: "moby", title: "Moby Dick", published_year: 1851 },
      expectedDoc,
    ]);

    const docs = await ctx.client.collection(collection.name).get(["lotr"]);
    expect(docs).toEqual({ lotr: expectedDoc });
  });

  test("get multiple documents", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    const expectedDocs = {
      lotr: {
        _id: "lotr",
        title: "The Lord of the Rings: The Fellowship of the Ring",
        published_year: 1954,
      },
      moby: {
        _id: "moby",
        title: "Moby Dick",
        published_year: 1851,
      },
    };

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "catcher",
        title: "The Catcher in the Rye",
        published_year: 1951,
      },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925 },
      expectedDocs.moby,
      expectedDocs.lotr,
    ]);

    const docs = await ctx.client
      .collection(collection.name)
      .get(["lotr", "moby"]);
    expect(docs).toEqual(expectedDocs);
  });

  test("get document fields", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", title: "The Catcher in the Rye", published_year: 1951 },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925 },
      { _id: "moby", title: "Moby Dick", published_year: 1851 },
      {
        _id: "lotr",
        title: "The Lord of the Rings: The Fellowship of the Ring",
        published_year: 1954,
      },
    ]);

    const docs = await ctx.client
      .collection(collection.name)
      .get(["lotr"], ["title", "published_year"]);
    expect(docs).toEqual({
      lotr: {
        _id: "lotr",
        title: "The Lord of the Rings: The Fellowship of the Ring",
        published_year: 1954,
      },
    });
  });
});
