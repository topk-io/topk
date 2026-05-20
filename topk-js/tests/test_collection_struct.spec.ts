import { struct as dataStruct } from "../lib/data";
import { field, fn, select } from "../lib/query";
import {
  bool,
  float,
  int,
  list,
  semanticIndex,
  struct as schemaStruct,
  text,
} from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Struct", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("struct round trip", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      outer: schemaStruct({
        inner: schemaStruct({
          leaf: text(),
          sibling: text(),
        }),
      }),
    });

    const lsn = await ctx.client.collection(collection.name).upsert([
      {
        _id: "one",
        outer: dataStruct({
          inner: dataStruct({
            leaf: "v",
            sibling: "s",
          }),
        }),
      },
    ]);

    const docs = await ctx.client
      .collection(collection.name)
      .get(["one"], null, { lsn });

    expect(docs.one.outer.inner.leaf).toBe("v");
    expect(docs.one.outer.inner.sibling).toBe("s");
  });

  test("struct query with naked object", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      meta: schemaStruct({
        author: text(),
        year: int(),
        tag: text(),
      }),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "old",
        meta: { author: "alice", year: 1999, tag: "classic" },
      },
      {
        _id: "new",
        meta: { author: "bob", year: 2024, tag: "fresh" },
      },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        "meta.author": field("meta.author"),
        "meta.tag": field("meta.tag"),
      })
        .filter(field("meta.year").gt(2020))
        .topk(field("meta.year"), 10, true)
    );

    expect(results).toHaveLength(1);
    expect(results[0]._id).toBe("new");
    expect(results[0].meta.author).toBe("bob");
    expect(results[0].meta.tag).toBe("fresh");
    expect(results[0].meta.year).toBeUndefined();
  });

  test("struct semantic index on subfield", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      meta: schemaStruct({
        description: text().index(semanticIndex()),
      }),
    });

    const lsn = await ctx.client.collection(collection.name).upsert([
      {
        _id: "rust",
        meta: { description: "a systems programming language" },
      },
      {
        _id: "python",
        meta: dataStruct({ description: "a snake" }),
      },
    ]);

    const results = await ctx.client.collection(collection.name).query(
      select({
        sim: fn.semanticSimilarity("meta.description", "programming"),
      }).topk(field("sim"), 2, true),
      { lsn }
    );

    expect(results).toHaveLength(2);
    expect(results.every((doc) => doc.sim !== undefined)).toBe(true);
  });

  test("struct update deep merge", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      meta: schemaStruct({
        author: text(),
        title: text(),
      }),
    });

    await ctx.client.collection(collection.name).upsert([
      {
        _id: "one",
        meta: dataStruct({ author: "alice", title: "v1" }),
      },
    ]);

    const lsn = await ctx.client
      .collection(collection.name)
      .update([{ _id: "one", meta: { title: "v2" } }], true);

    const docs = await ctx.client
      .collection(collection.name)
      .get(["one"], null, { lsn });

    expect(docs.one.meta.title).toBe("v2");
    expect(docs.one.meta.author).toBe("alice");
  });

  test("deeply nested naked objects", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      outer: schemaStruct({
        middle: schemaStruct({
          leaf: text(),
        }),
      }),
    });

    const lsn = await ctx.client.collection(collection.name).upsert([
      {
        _id: "one",
        outer: { middle: { leaf: "deep" } },
      },
    ]);

    const docs = await ctx.client
      .collection(collection.name)
      .get(["one"], null, { lsn });

    expect(docs.one.outer.middle.leaf).toBe("deep");
  });

  test("naked object with all primitive types", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      meta: schemaStruct({
        s: text(),
        i: int(),
        f: float(),
        b: bool(),
      }),
    });

    const lsn = await ctx.client.collection(collection.name).upsert([
      {
        _id: "one",
        meta: { s: "hello", i: 42, f: 3.14, b: true },
      },
    ]);

    const docs = await ctx.client
      .collection(collection.name)
      .get(["one"], null, { lsn });

    expect(docs.one.meta.s).toBe("hello");
    expect(docs.one.meta.i).toBe(42);
    expect(docs.one.meta.f).toBeCloseTo(3.14, 2);
    expect(docs.one.meta.b).toBe(true);
  });

  test("naked object with list subfield", async () => {
    // A plain JS array inside a naked struct: the string-list parser fires
    // before the struct fallback, so ["a", "b"] becomes Value::List(String).
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      meta: schemaStruct({
        tags: list({ valueType: "text" }),
        count: int(),
      }),
    });

    const lsn = await ctx.client.collection(collection.name).upsert([
      {
        _id: "one",
        meta: { tags: ["rust", "systems"], count: 2 },
      },
    ]);

    const docs = await ctx.client
      .collection(collection.name)
      .get(["one"], null, { lsn });

    expect(docs.one.meta.tags).toEqual(["rust", "systems"]);
    expect(docs.one.meta.count).toBe(2);
  });

  test("naked object and explicit struct() are equivalent", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", {
      meta: schemaStruct({
        author: text(),
        year: int(),
      }),
    });

    const lsn = await ctx.client.collection(collection.name).upsert([
      { _id: "naked", meta: { author: "alice", year: 2024 } },
      { _id: "explicit", meta: dataStruct({ author: "alice", year: 2024 }) },
    ]);

    const docs = await ctx.client
      .collection(collection.name)
      .get(["naked", "explicit"], null, { lsn });

    expect(docs.naked.meta.author).toBe(docs.explicit.meta.author);
    expect(docs.naked.meta.year).toBe(docs.explicit.meta.year);
  });

  test("struct schema rejects dotted field name", async () => {
    const ctx = getContext();

    await expect(
      ctx.createCollection("test", {
        meta: schemaStruct({ "bad.name": text() }),
      })
    ).rejects.toThrow(/FieldNameContainsDot|struct path separator/);
  });
});
