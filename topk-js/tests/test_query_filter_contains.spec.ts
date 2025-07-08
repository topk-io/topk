import { field, filter } from "../lib/query";
import { int, keywordIndex, text } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Logical Queries", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("query contains", async () => {
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
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        published_year: 1960,
      },
      { _id: "alchemist", title: "The Alchemist", published_year: 1988 },
      { _id: "harry", title: "Harry Potter", published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954 },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813 },
      { _id: "1984", title: "1984", published_year: 1949 },
      { _id: "hobbit", title: "The Hobbit", published_year: 1937 },
    ]);

    const results = await ctx.client
      .collection(collection.name)
      .query(
        filter(field("title").contains("he"))
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(results.map((doc) => doc._id))).toEqual(
      new Set(["catcher", "gatsby", "alchemist", "lotr", "hobbit"])
    );
  });
});
