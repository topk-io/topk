import { i32List, u32List } from '../lib/data';
import { field, filter, literal, not, select } from "../lib/query";
import { int, keywordIndex, list, text } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Logical Queries Contains", () => {
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

  test("string contains literal no match", async () => {
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

    const result = await ctx.client
      .collection(collection.name)
      .query(
        filter(field("_id").contains("rubbish"))
          .topk(field("published_year"), 100, false)
      );

    expect(result.length).toBe(0);
  });

  test("string contains literal empty", async () => {
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

    const result = await ctx.client
      .collection(collection.name)
      .query(
        filter(field("_id").contains(""))
          .topk(field("published_year"), 100, false)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set([
        "gatsby",
        "catcher",
        "moby",
        "mockingbird",
        "alchemist",
        "harry",
        "lotr",
        "pride",
        "1984",
        "hobbit",
      ])
    );
  });

  test("string contains literal with keyword index", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      summary: text().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", title: "The Catcher in the Rye", summary: "A story about a young man's journey of healing", published_year: 1951 },
      { _id: "gatsby", title: "The Great Gatsby", summary: "A tale of love and the American Dream", published_year: 1925 },
      { _id: "moby", title: "Moby Dick", summary: "A captain's quest to hunt a white whale", published_year: 1851 },
      { _id: "mockingbird", title: "To Kill a Mockingbird", summary: "A story about racial injustice in the South", published_year: 1960 },
      { _id: "alchemist", title: "The Alchemist", summary: "A shepherd's journey and his destiny", published_year: 1988 },
      { _id: "harry", title: "Harry Potter", summary: "A boy wizard's adventures at Hogwarts", published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", summary: "A hobbit's quest to destroy the One Ring", published_year: 1954 },
      { _id: "pride", title: "Pride and Prejudice", summary: "A romance between Elizabeth and Darcy", published_year: 1813 },
      { _id: "1984", title: "1984", summary: "A dystopian tale of totalitarian control", published_year: 1949 },
      { _id: "hobbit", title: "The Hobbit", summary: "A hobbit's unexpected journey to help dwarves", published_year: 1937 },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        filter(field("summary").contains("to h"))
          .topk(field("published_year"), 100, false)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set([
        "moby",
        "hobbit",
      ])
    );
  });

  test("string contains field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      summary: text().index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", title: "The Catcher in the Rye", summary: "A story about a young man's journey to heal", published_year: 1951 },
      { _id: "gatsby", title: "The Great Gatsby", summary: "A tale of love and the American Dream", published_year: 1925 },
      { _id: "moby", title: "Moby Dick", summary: "A captain's quest hunting a white whale", published_year: 1851 },
      { _id: "mockingbird", title: "To Kill a Mockingbird", summary: "A story about racial injustice in the South", published_year: 1960 },
      { _id: "alchemist", title: "The Alchemist", summary: "A shepherd's journey and his destiny", published_year: 1988 },
      { _id: "harry", title: "Harry Potter", summary: "A boy wizard's adventures at Hogwarts", published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", summary: "A hobbit's quest to destroy the One Ring", published_year: 1954 },
      { _id: "pride", title: "Pride and Prejudice", summary: "A romance between Elizabeth and Darcy", published_year: 1813 },
      { _id: "1984", title: "1984", summary: "A dystopian tale of totalitarian control", published_year: 1949 },
      { _id: "hobbit", title: "The Hobbit", summary: "A hobbit's unexpected journey to help dwarves", published_year: 1937 },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        filter(field("title").contains(field("_id")))
          .topk(field("published_year"), 100, false)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["1984"])
    );
  });

  test("string in field", async () => {
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

    const result = await ctx.client
      .collection(collection.name)
      .query(
        filter(field("_id").in(field("title")))
          .topk(field("published_year"), 100, false)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["1984"])
    );
  });

  test("string in field self", async () => {
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

    const result = await ctx.client
      .collection(collection.name)
      .query(
        filter(not(field("title").in(field("title"))))
          .topk(field("published_year"), 100, false)
      );

    expect(result.length).toBe(0);
  });

  test("list match any with keyword index", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      tags: list({ valueType: "text" }).index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", title: "The Catcher in the Rye", tags: ["adolescence", "alienation", "phoniness"], published_year: 1951 },
      { _id: "gatsby", title: "The Great Gatsby", tags: ["love", "romance", "wealth", "marriage"], published_year: 1925 },
      { _id: "moby", title: "Moby Dick", tags: ["adventure", "obsession", "nature"], published_year: 1851 },
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        tags: ["racism", "justice", "childhood"],
        published_year: 1960,
      },
      { _id: "alchemist", title: "The Alchemist", tags: ["journey", "dreams", "destiny"], published_year: 1988 },
      { _id: "harry", title: "Harry Potter", tags: ["magic", "friendship", "good vs evil"], published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", tags: ["fantasy", "friendship", "good vs evil"], published_year: 1954 },
      { _id: "pride", title: "Pride and Prejudice", tags: ["pride", "love", "romance", "class", "marriage", "prejudice"], published_year: 1813 },
      { _id: "1984", title: "1984", tags: ["dystopia", "surveillance", "totalitarianism"], published_year: 1949 },
      { _id: "hobbit", title: "The Hobbit", tags: ["adventure", "journey", "fantasy"], published_year: 1937 },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title"),
          tags: field("tags")
        })
          .filter(field("tags").matchAny("love"))
          .topk(field("published_year"), 100, true)
      );

    expect(result).toEqual([
      {
        _id: "pride",
        title: "Pride and Prejudice",
        tags: ["pride", "love", "romance", "class", "marriage", "prejudice"]
      },
      {
        _id: "gatsby",
        title: "The Great Gatsby",
        tags: ["love", "romance", "wealth", "marriage"]
      },
    ]);
  });

  test("list match any/all without keyword index", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      codes: list({ valueType: "text" }), // No keyword index
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "hobbit", title: "The Hobbit", codes: ["ISBN 0-547-92821-2", "ASIN B007978NPG"], published_year: 1937 },
    ]);

    const filterExpressions = [
      field("codes").matchAny("ISBN 0-547-92821-2"),
      field("codes").matchAll("ISBN 0-547-92821-2"),
    ];

    for (const filterExpr of filterExpressions) {
      await expect(
        ctx.client.collection(collection.name).query(
          select({
            title: field("title"),
            codes: field("codes")
          })
            .filter(filterExpr)
            .topk(field("published_year"), 100, true)
        )
      ).rejects.toThrow();
    }
  });

  test("list contains with keyword index", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      tags: list({ valueType: "text" }).index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "pride",
        title: "Pride and Prejudice",
        tags: ["pride", "love", "romance", "class", "marriage", "prejudice"],
        published_year: 1813
      },
      {
        _id: "gatsby",
        title: "The Great Gatsby",
        tags: ["love", "romance", "wealth", "marriage"],
        published_year: 1925
      },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title"),
          tags: field("tags")
        })
          .filter(field("tags").contains("love"))
          .topk(field("published_year"), 100, true)
      );

    expect(result).toEqual([
      {
        _id: "pride",
        title: "Pride and Prejudice",
        tags: ["pride", "love", "romance", "class", "marriage", "prejudice"]
      },
      {
        _id: "gatsby",
        title: "The Great Gatsby",
        tags: ["love", "romance", "wealth", "marriage"]
      },
    ]);
  });

  test("list contains literal", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required(),
      codes: list({ valueType: "text" }),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "lotr",
        title: "The Lord of the Rings: The Fellowship of the Ring",
        codes: [
          "ISBN 978-0-547-92821-0",
          "ISBN 0-547-92821-2",
          "OCLC 434394005",
          "LCCN 2004558654",
          "Barcode 0618346252",
        ],
        published_year: 1954
      },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title"),
          codes: field("codes")
        })
          .filter(field("codes").contains("ISBN 0-547-92821-2"))
          .topk(field("published_year"), 100, true)
      );

    expect(result).toEqual([
      {
        _id: "lotr",
        title: "The Lord of the Rings: The Fellowship of the Ring",
        codes: [
          "ISBN 978-0-547-92821-0",
          "ISBN 0-547-92821-2",
          "OCLC 434394005",
          "LCCN 2004558654",
          "Barcode 0618346252",
        ],
      }
    ]);
  });

  test("list contains int literal", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required(),
      reprint_years: list({ valueType: "integer" }),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        reprint_years: i32List([1962, 1975, 1988, 1999, 2006]),
        published_year: 1960,
      },
      {
        _id: "harry",
        title: "Harry Potter",
        reprint_years: i32List([1999, 2001, 2003]),
        published_year: 1997,
      },
      {
        _id: "gatsby",
        title: "The Great Gatsby",
        reprint_years: i32List([1945, 1953, 1974]),
        published_year: 1925,
      },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title"),
          reprint_years: field("reprint_years")
        })
          .filter(field("reprint_years").contains(1999))
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["mockingbird", "harry"])
    );
  });

  test("list contains int literal different type", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required(),
      reprint_years: list({ valueType: "integer" }),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        reprint_years: i32List([1962, 1975, 1988, 1999, 2006]),
        published_year: 1960,
      },
      {
        _id: "harry",
        title: "Harry Potter",
        reprint_years: i32List([1999, 2001, 2003]),
        published_year: 1997,
      },
      {
        _id: "gatsby",
        title: "The Great Gatsby",
        reprint_years: i32List([1945, 1953, 1974]),
        published_year: 1925,
      },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title"),
          reprint_years: field("reprint_years")
        })
          .filter(field("reprint_years").contains(literal(1999)))
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["mockingbird", "harry"])
    );
  });

  test("list contains int field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required(),
      reprint_years: list({ valueType: "integer" }),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        reprint_years: i32List([1962, 1975, 1988, 1999, 2006]),
        published_year: 1960,
      },
      {
        _id: "harry",
        title: "Harry Potter",
        reprint_years: i32List([1998, 2001, 2003]),
        published_year: 1997,
      },
      {
        _id: "gatsby",
        title: "The Great Gatsby",
        reprint_years: i32List([1945, 1953, 1974]),
        published_year: 1925,
      },
      {
        _id: "1984",
        title: "1984",
        reprint_years: i32List([1950, 1961, 1984]),
        published_year: 1949,
      },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title"),
          reprint_years: field("reprint_years")
        })
          .filter(field("reprint_years").contains(field("published_year").add(1)))
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["harry", "1984"])
    );
  });

  test("list in int field", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required(),
      reprint_years: list({ valueType: "integer" }),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        reprint_years: i32List([1962, 1975, 1988, 1999, 2006]),
        published_year: 1960,
      },
      {
        _id: "harry",
        title: "Harry Potter",
        reprint_years: i32List([1998, 2001, 2003]),
        published_year: 1997,
      },
      {
        _id: "gatsby",
        title: "The Great Gatsby",
        reprint_years: i32List([1945, 1953, 1974]),
        published_year: 1925,
      },
      {
        _id: "1984",
        title: "1984",
        reprint_years: i32List([1950, 1961, 1984]),
        published_year: 1949,
      },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title"),
          reprint_years: field("reprint_years")
        })
          .filter(field("published_year").add(1).in(field("reprint_years")))
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["harry", "1984"])
    );
  });

  test("list contains string field with keyword index", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      tags: list({ valueType: "text" }).index(keywordIndex()),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", title: "The Catcher in the Rye", tags: ["adolescence", "alienation", "phoniness"], published_year: 1951 },
      { _id: "gatsby", title: "The Great Gatsby", tags: ["love", "romance", "wealth", "marriage"], published_year: 1925 },
      { _id: "moby", title: "Moby Dick", tags: ["adventure", "obsession", "nature"], published_year: 1851 },
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        tags: ["racism", "justice", "childhood"],
        published_year: 1960,
      },
      { _id: "alchemist", title: "The Alchemist", tags: ["journey", "dreams", "destiny"], published_year: 1988 },
      { _id: "harry", title: "Harry Potter", tags: ["magic", "friendship", "good vs evil"], published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", tags: ["fantasy", "friendship", "good vs evil"], published_year: 1954 },
      { _id: "pride", title: "Pride and Prejudice", tags: ["pride", "love", "romance", "class", "marriage", "prejudice"], published_year: 1813 },
      { _id: "1984", title: "1984", tags: ["dystopia", "surveillance", "totalitarianism"], published_year: 1949 },
      { _id: "hobbit", title: "The Hobbit", tags: ["adventure", "journey", "hobbit", "fantasy"], published_year: 1937 },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title"),
          tags: field("tags")
        })
          .filter(field("tags").contains(field("_id")))
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["pride", "hobbit"])
    );
  });

  test("list contains string field without keyword index", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      codes: list({ valueType: "text" }), // No keyword index
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "catcher", title: "The Catcher in the Rye", codes: ["ISBN-13: 978-0316769174", "ASIN: B000FC0PDA"], published_year: 1951 },
      { _id: "gatsby", title: "The Great Gatsby", codes: ["ISBN-13: 978-0743273565", "ASIN: B000FC2X1Y"], published_year: 1925 },
      { _id: "moby", title: "Moby Dick", codes: ["ISBN-13: 978-0142437247", "ASIN: B000FC2LSO"], published_year: 1851 },
      {
        _id: "mockingbird",
        title: "To Kill a Mockingbird",
        codes: ["ISBN-13: 978-0061120084", "ASIN: B000FC2L8C"],
        published_year: 1960,
      },
      { _id: "alchemist", title: "The Alchemist", codes: ["ISBN-13: 978-0062315007", "ASIN: B000FCKC4C"], published_year: 1988 },
      { _id: "harry", title: "Harry Potter", codes: ["ISBN-13: 978-0439708180", "ASIN: B000FCKGXA"], published_year: 1997 },
      { _id: "lotr", title: "The Lord of the Rings", codes: ["ISBN-13: 978-0544003415", "ASIN: B007978NPG"], published_year: 1954 },
      { _id: "pride", title: "Pride and Prejudice", codes: ["ISBN-13: 978-0141439518", "ASIN: B000FC2JD8"], published_year: 1813 },
      { _id: "1984", title: "1984", codes: ["ISBN-13: 978-0451524935", "ASIN: B003JTHWKU", "1984"], published_year: 1949 },
      { _id: "hobbit", title: "The Hobbit", codes: ["ISBN-13: 978-0547928227", "ASIN: B007978NPG"], published_year: 1937 },
    ]);

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title"),
          codes: field("codes")
        })
          .filter(field("codes").contains(field("_id")))
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["1984"])
    );
  });

  test("list contains invalid types", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required().index(keywordIndex()),
      codes: list({ valueType: "text" }),
      reprint_years: list({ valueType: "integer" }),
      published_year: int(),
    });

    await ctx.client.collection(collection.name).upsert([
      { _id: "test", title: "Test Book", codes: ["ISBN-123"], reprint_years: i32List([2000]), published_year: 1990 },
    ]);

    const filterExpressions = [
      field("codes").contains(978),
      field("codes").contains(field("published_year")),
      field("reprint_years").contains(field("title")),
      field("published_year").contains(field("reprint_years")),
    ];

    for (const filterExpr of filterExpressions) {
      await expect(
        await ctx.client.collection(collection.name).query(
          select({
            title: field("title"),
            codes: field("codes")
          })
            .filter(filterExpr)
            .topk(field("published_year"), 100, true)
        )
      ).toHaveLength(0);
    }

    expect(() => {
      (field("codes") as any).contains([
        "ISBN 978-0-547-92821-0",
        "ISBN 0-547-92821-2",
      ]);
    }).toThrow();

    expect(() => {
      // @ts-expect-error
      field("codes").contains(true);
    }).toThrow();
  });

  test("string in", async () => {
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

    const result = await ctx.client
      .collection(collection.name)
      .query(
        filter(field("_id").in("harryhobbitlotr"))
          .topk(field("published_year"), 100, false)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["harry", "hobbit", "lotr"])
    );
  });

  test("in list literal int u32", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required(),
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

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title")
        })
          .filter(field("published_year").in(u32List([1999, 1988, 1997])))
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["alchemist", "harry"])
    );
  });

  test("in list literal string", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("books", {
      title: text().required(),
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

    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({
          title: field("title")
        })
          .filter(field("title").in([
            "The Great Gatsby",
            "The Catcher in the Rye",
            "The Lord of the Rings: NOT THIS ONE",
            "The",
            "something 123",
          ]))
          .topk(field("published_year"), 100, true)
      );

    expect(new Set(result.map((doc) => doc._id))).toEqual(
      new Set(["gatsby", "catcher"])
    );
  });
});
