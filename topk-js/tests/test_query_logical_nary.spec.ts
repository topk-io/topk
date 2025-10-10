import { field, filter, not, select, all, any } from "../lib/query";
import { int, keywordIndex, list, text } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Logical N-ary Queries", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("any codes vec", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      codes: list({ valueType: "text" }),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", title: "1984", published_year: 1949, codes: ["DOI 10.1000/182"] },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954, codes: ["Barcode 0618346252"] },
      { _id: "mockingbird", title: "To Kill a Mockingbird", published_year: 1960, codes: ["UPC 025192354670"] },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925, codes: ["Other"] },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      filter(
        any([
          field("codes").contains("DOI 10.1000/182"),
          field("codes").contains("Barcode 0618346252"),
          field("codes").contains("UPC 025192354670"),
        ])
      ).limit(100)
    );

    const ids = result.map((doc) => doc._id).sort();
    expect(ids).toEqual(["1984", "lotr", "mockingbird"]);
  });

  test("all codes vec", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      tags: list({ valueType: "text" }),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "harry", title: "Harry Potter", published_year: 1997, tags: ["wizard", "school", "magic"] },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954, tags: ["wizard", "magic"] },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925, tags: ["romance"] },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      filter(
        all([
          field("tags").contains("wizard"),
          field("tags").contains("school"),
          field("tags").contains("magic"),
        ])
      ).limit(100)
    );

    const ids = result.map((doc) => doc._id).sort();
    expect(ids).toEqual(["harry"]);
  });

  test("select any flag", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      codes: list({ valueType: "text" }),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "1984", title: "1984", published_year: 1949, codes: ["DOI 10.1000/182"] },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813, codes: ["OCLC 934546789"] },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954, codes: ["Other"] },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        has_code: any([
          field("codes").contains("DOI 10.1000/182"),
          field("codes").contains("OCLC 934546789"),
        ]),
      })
        .filter(field("_id").eq("1984").or(field("_id").eq("pride")).or(field("_id").eq("lotr")))
        .limit(100)
    );

    results.sort((d1, d2) => d1._id.localeCompare(d2._id));

    expect(results).toEqual([
      { _id: "1984", has_code: true },
      { _id: "lotr", has_code: false },
      { _id: "pride", has_code: true },
    ]);
  });

  test("select all flag", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      codes: list({ valueType: "text" }),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925, codes: ["UPC 074327356709", "ASIN B000FC0SIS"] },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813, codes: ["UPC 074327356709"] },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        all_match: all([
          field("codes").contains("UPC 074327356709"),
          field("codes").contains("ASIN B000FC0SIS"),
        ]),
      }).limit(100)
    );

    results.sort((d1, d2) => d1._id.localeCompare(d2._id));

    expect(results.length).toBe(2);
    expect(results[0]._id).toBe("gatsby");
    expect(results[0].all_match).toBe(true);
    expect(results[1]._id).toBe("pride");
    expect(results[1].all_match).toBe(false);
  });

  test("nested any all", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      tags: list({ valueType: "text" }),
      codes: list({ valueType: "text" }),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925, codes: ["UPC 074327356709", "ASIN B000FC0SIS"], tags: ["romance"] },
      { _id: "harry", title: "Harry Potter", published_year: 1997, tags: ["wizard", "magic"], codes: ["Other"] },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954, tags: ["wizard", "magic"], codes: ["Other"] },
    ]);

    const expr = any([
      all([field("tags").contains("wizard"), field("tags").contains("magic")]),
      all([
        field("codes").contains("UPC 074327356709"),
        field("codes").contains("ASIN B000FC0SIS"),
      ]),
    ]);

    const result = await ctx.client.collection(collection.name).query(
      filter(expr).limit(100)
    );

    const ids = result.map((doc) => doc._id).sort();
    expect(ids).toEqual(["gatsby", "harry", "lotr"]);
  });

  test("non-nested any and all", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      tags: list({ valueType: "text" }),
      codes: list({ valueType: "text" }),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "harry", title: "Harry Potter", published_year: 1997, tags: ["wizard", "magic"], codes: ["Barcode 0618346252"] },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954, tags: ["wizard", "magic"], codes: ["UPC 043970818909"] },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925, tags: ["romance"], codes: ["Other"] },
    ]);

    const codesAny = any([
      field("codes").contains("Barcode 0618346252"),
      field("codes").contains("UPC 043970818909"),
    ]);

    const tagsAll = all([
      field("tags").contains("wizard"),
      field("tags").contains("magic"),
    ]);

    const result = await ctx.client.collection(collection.name).query(
      filter(codesAny.and(tagsAll)).limit(100)
    );

    const ids = result.map((doc) => doc._id).sort();
    expect(ids).toEqual(["harry", "lotr"]);
  });

  test("any mixed exprs", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      tags: list({ valueType: "text" }),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925, tags: ["romance"] },
      { _id: "moby", title: "Moby Dick", published_year: 1851, tags: ["adventure"] },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813, tags: ["romance"] },
      { _id: "1984", title: "1984", published_year: 1949, tags: ["dystopian"] },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      filter(
        any([
          field("title").startsWith("The Great"),
          field("tags").contains("romance"),
          field("published_year").lt(1900),
        ])
      ).limit(100)
    );

    const ids = result.map((doc) => doc._id).sort();
    expect(ids).toEqual(["gatsby", "moby", "pride"]);
  });

  test("all mixed exprs", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      tags: list({ valueType: "text" }),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "alchemist", title: "The Alchemist", published_year: 1988, tags: ["fantasy"] },
      { _id: "catcher", title: "The Catcher in the Rye", published_year: 1951, tags: ["coming-of-age"] },
      { _id: "hobbit", title: "The Hobbit", published_year: 1937, tags: ["fantasy"] },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954, tags: ["fantasy"] },
      { _id: "pride", title: "Pride and Prejudice", published_year: 1813, tags: ["romance"] },
    ]);

    const result = await ctx.client.collection(collection.name).query(
      filter(
        all([
          field("published_year").gt(1900),
          field("title").contains("The"),
          not(field("tags").contains("romance")),
        ])
      ).limit(100)
    );

    const ids = result.map((doc) => doc._id).sort();
    expect(ids).toEqual(["alchemist", "catcher", "hobbit", "lotr"]);
  });

  test("all large arity", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      tags: list({ valueType: "text" }),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "harry", title: "Harry Potter", published_year: 1997, tags: ["wizard"] },
      { _id: "lotr", title: "The Lord of the Rings", published_year: 1954, tags: ["wizard"] },
      { _id: "gatsby", title: "The Great Gatsby", published_year: 1925, tags: ["romance"] },
    ]);

    const expr = all(Array(32).fill(field("tags").contains("wizard")));

    const result = await ctx.client.collection(collection.name).query(
      filter(expr).limit(100)
    );

    const ids = result.map((doc) => doc._id).sort();
    expect(ids).toEqual(["harry", "lotr"]);
  });

  test("all max arity", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      published_year: int(),
      tags: list({ valueType: "text" }),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "harry", title: "Harry Potter", published_year: 1997, tags: ["wizard"] },
    ]);

    const expr = all(Array(33).fill(field("tags").contains("wizard")));

    await expect(
      ctx.client.collection(collection.name).query(
        filter(expr).limit(100)
      )
    ).rejects.toThrow(/N-ary expression has too many operands/);
  });
});
