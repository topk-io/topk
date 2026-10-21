import { readFileSync } from "fs";
import { join } from "path";
import { image as dataImage, Image } from "../lib/data";
import { field, select } from "../lib/query";
import { image } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

const IMAGES = join(__dirname, "../../assets/test/img");

describe("Image", () => {
  const contexts: ProjectContext[] = [];

  function getContext(): ProjectContext {
    const ctx = newProjectContext();
    contexts.push(ctx);
    return ctx;
  }

  afterAll(async () => {
    await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
  });

  test("image round trip", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", { image: image() });

    const jpeg = readFileSync(join(IMAGES, "cat.jpg"));
    const png = readFileSync(join(IMAGES, "form.png"));

    const lsn = await ctx.client.collection(collection.name).upsert([
      { _id: "jpeg", image: dataImage(jpeg) },
      { _id: "png", image: dataImage(png) },
    ]);

    const docs = await ctx.client
      .collection(collection.name)
      .get(["jpeg", "png"], null, { lsn });

    expect(docs.jpeg.image).toBeInstanceOf(Image);
    expect(docs.jpeg.image.format).toBe("jpeg");
    expect(docs.jpeg.image.data).toEqual(jpeg);
    expect(docs.jpeg.image.data).toBe(docs.jpeg.image.data);

    expect(docs.png.image).toBeInstanceOf(Image);
    expect(docs.png.image.format).toBe("png");
    expect(docs.png.image.data).toEqual(png);
  });

  test("image query", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", { image: image() });

    const lsn = await ctx.client.collection(collection.name).upsert([
      { _id: "cat", rank: 1, image: dataImage(readFileSync(join(IMAGES, "cat.jpg"))) },
      { _id: "form", rank: 2, image: dataImage(readFileSync(join(IMAGES, "form.png"))) },
    ]);

    // Images can't be selected, they are fetched by the ids returned from the query
    const result = await ctx.client
      .collection(collection.name)
      .query(
        select({ rank: field("rank") }).sort(field("rank"), true).limit(10),
        { lsn }
      );
    const ids = result.map((doc) => doc._id);
    expect(ids).toEqual(["cat", "form"]);

    const docs = await ctx.client
      .collection(collection.name)
      .get(ids, null, { lsn });
    expect(ids.map((id) => docs[id].image.format)).toEqual(["jpeg", "png"]);
  });

  test("image re-upsert", async () => {
    const ctx = getContext();
    const collection = await ctx.createCollection("test", { image: image() });

    const png = readFileSync(join(IMAGES, "form.png"));

    let lsn = await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "form", image: dataImage(png) }]);
    const docs = await ctx.client
      .collection(collection.name)
      .get(["form"], null, { lsn });

    // Images read from TopK can be written back as-is
    lsn = await ctx.client
      .collection(collection.name)
      .upsert([{ _id: "copy", image: docs.form.image }]);
    const copy = await ctx.client
      .collection(collection.name)
      .get(["copy"], null, { lsn });

    expect(copy.copy.image.format).toBe("png");
    expect(copy.copy.image.data).toEqual(png);
  });
});
