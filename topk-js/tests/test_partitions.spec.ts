import { field, select } from "../lib/query";
import { newProjectContext, ProjectContext } from "./setup";

async function collectPartitions(
  client: ProjectContext["client"],
  collectionName: string,
  prefix?: string
): Promise<string[]> {
  const names: string[] = [];
  for await (const partition of client
    .collection(collectionName)
    .listPartitions(prefix)) {
    names.push(partition.name);
  }
  return names;
}

describe("Partitions", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("partition upsert isolation", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    const defaultLsn = await ctx.client.collection(collection.name).upsert([
      { _id: "shared", partition: "default" },
      { _id: "only-default", partition: "default" },
    ]);
    expect(defaultLsn).toBe("1");

    const p1Lsn = await ctx.client.collection(collection.name, "p1").upsert([
      { _id: "shared", partition: "p1" },
      { _id: "only-p1", partition: "p1" },
    ]);
    expect(p1Lsn).toBe("1");

    const p2Lsn = await ctx.client.collection(collection.name, "p2").upsert([
      { _id: "shared", partition: "p2" },
    ]);
    expect(p2Lsn).toBe("1");

    const defaultDocs = await ctx.client
      .collection(collection.name)
      .get(["shared", "only-default", "only-p1"], null, { lsn: defaultLsn });
    expect(Object.keys(defaultDocs).sort()).toEqual(["only-default", "shared"]);
    expect(defaultDocs.shared.partition).toBe("default");

    const p1Docs = await ctx.client
      .collection(collection.name, "p1")
      .get(["shared", "only-default", "only-p1"], null, { lsn: p1Lsn });
    expect(Object.keys(p1Docs).sort()).toEqual(["only-p1", "shared"]);
    expect(p1Docs.shared.partition).toBe("p1");

    const p2Docs = await ctx.client
      .collection(collection.name, "p2")
      .get(["shared"], null, { lsn: p2Lsn });
    expect(Object.keys(p2Docs)).toEqual(["shared"]);
    expect(p2Docs.shared.partition).toBe("p2");
  });

  test("list partitions empty", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    const partitions = await collectPartitions(ctx.client, collection.name);
    expect(partitions).toEqual([]);
  });

  test("list partitions", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    await ctx.client
      .collection(collection.name, "partition-a")
      .upsert([{ _id: "doc-a" }]);
    await ctx.client
      .collection(collection.name, "partition-b")
      .upsert([{ _id: "doc-b" }]);

    const partitions = (
      await collectPartitions(ctx.client, collection.name)
    ).sort();
    expect(partitions).toEqual(["partition-a", "partition-b"]);
  });

  test("list partitions with prefix", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    for (const name of ["foo", "foot", "bar"]) {
      await ctx.client
        .collection(collection.name, name)
        .upsert([{ _id: "doc" }]);
    }

    const partitions = await collectPartitions(
      ctx.client,
      collection.name,
      "foo"
    );
    expect(new Set(partitions)).toEqual(new Set(["foo", "foot"]));
  });

  test("list partitions excludes default", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    await ctx.client.collection(collection.name).upsert([
      { _id: "doc", partition: "default" },
    ]);
    await ctx.client
      .collection(collection.name, "named-partition")
      .upsert([{ _id: "doc" }]);

    const partitions = await collectPartitions(ctx.client, collection.name);
    expect(partitions).toEqual(["named-partition"]);
  });

  test("delete partition", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    await ctx.client
      .collection(collection.name, "test-partition")
      .upsert([
        { _id: "doc1", value: "one" },
        { _id: "doc2", value: "two" },
      ]);

    let partitions = await collectPartitions(ctx.client, collection.name);
    expect(partitions).toEqual(["test-partition"]);

    await ctx.client
      .collection(collection.name)
      .deletePartition("test-partition");

    partitions = await collectPartitions(ctx.client, collection.name);
    expect(partitions).toEqual([]);

    await expect(
      ctx.client.collection(collection.name, "test-partition").count()
    ).rejects.toThrow(/partition not found/i);
  });

  test("delete partition does not affect other partitions", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    const p1Lsn = await ctx.client
      .collection(collection.name, "partition-a")
      .upsert([{ _id: "doc-a", partition: "partition-a" }]);
    const p2Lsn = await ctx.client
      .collection(collection.name, "partition-b")
      .upsert([{ _id: "doc-a", partition: "partition-b" }]);

    await ctx.client
      .collection(collection.name)
      .deletePartition("partition-a");

    const partitions = await collectPartitions(ctx.client, collection.name);
    expect(partitions).toEqual(["partition-b"]);

    const p2Docs = await ctx.client
      .collection(collection.name, "partition-b")
      .get(["doc-a"], null, { lsn: p2Lsn });
    expect(p2Docs["doc-a"].partition).toBe("partition-b");
    expect(p1Lsn).toBe("1");
    expect(p2Lsn).toBe("1");
  });

  test("upsert creates partition", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    const lsn = await ctx.client
      .collection(collection.name, "new-partition")
      .upsert([{ _id: "one", value: "created" }]);
    expect(lsn).toBe("1");

    const docs = await ctx.client
      .collection(collection.name, "new-partition")
      .get(["one"], null, { lsn });
    expect(docs.one.value).toBe("created");

    const count = await ctx.client
      .collection(collection.name, "new-partition")
      .count({ lsn });
    expect(count).toBe(1);
  });

  test("query non existent partition", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    await expect(
      ctx.client.collection(collection.name, "missing-partition").count()
    ).rejects.toThrow(/partition not found/i);

    await expect(
      ctx.client.collection(collection.name, "missing-partition").get(["doc"])
    ).rejects.toThrow(/partition not found/i);
  });

  test("partition with invalid name", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    await expect(
      ctx.client.collection(collection.name, "$foo&bar").upsert([
        { _id: "one", value: "created" },
      ])
    ).rejects.toThrow();
  });

  test("partition query filter", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test");

    const p1Lsn = await ctx.client.collection(collection.name, "p1").upsert([
      { _id: "doc1", partition: "p1", region: "us" },
      { _id: "doc2", partition: "p1", region: "eu" },
      { _id: "doc3", partition: "p1", region: "us" },
    ]);
    await ctx.client.collection(collection.name, "p2").upsert([
      { _id: "doc1", partition: "p2", region: "us" },
      { _id: "doc2", partition: "p2", region: "us" },
    ]);

    const p1Results = await ctx.client
      .collection(collection.name, "p1")
      .query(
        select({
          partition: field("partition"),
        })
          .filter(field("region").eq("us"))
          .limit(10),
        { lsn: p1Lsn }
      );
    expect(new Set(p1Results.map((doc) => doc._id))).toEqual(
      new Set(["doc1", "doc3"])
    );
    expect(p1Results.every((doc) => doc.partition === "p1")).toBe(true);

    const defaultResults = await ctx.client.collection(collection.name).query(
      select({
        partition: field("partition"),
      })
        .filter(field("region").eq("us"))
        .limit(10)
    );
    expect(defaultResults).toEqual([]);
  });
});
