import { i32List, u32List } from '../lib/data';
import { field, filter, literal, not, select } from "../lib/query";
import { int, keywordIndex, list, text } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("Logical Queries Regexp Filter", () => {
    const contexts: ProjectContext[] = [];

    function getContext(): ProjectContext {
        const ctx = newProjectContext();
        contexts.push(ctx);
        return ctx;
    }

    function getDocuments(): Record<string, any>[] {
        return [
            { _id: "catcher", title: "The Catcher in the Rye" },
            { _id: "gatsby", title: "The Great Gatsby" },
            { _id: "moby", title: "Moby Dick" },
            { _id: "mockingbird", title: "To Kill a Mockingbird" },
            { _id: "alchemist", title: "The Alchemist" },
            { _id: "harry", title: "Harry Potter" },
            { _id: "lotr", title: "The Lord of the Rings" },
            { _id: "pride", title: "Pride and Prejudice" },
            { _id: "1984", title: "1984" },
            { _id: "hobbit", title: "The Hobbit" },
        ];
    }

    afterAll(async () => {
        await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
    });

    test("query regexp match", async () => {
        const ctx = getContext();
        const collection = await ctx.createCollection("books", {});

        await ctx.client.collection(collection.name).upsert(getDocuments());

        const results = await ctx.client
            .collection(collection.name)
            .query(
                filter(field("_id").regexpMatch("^cat")).limit(10)
            );

        expect(results.map((doc) => doc._id)).toEqual(["catcher"]);
    });

    test("query regexp match with flags", async () => {
        const ctx = getContext();
        const collection = await ctx.createCollection("books", {});

        await ctx.client.collection(collection.name).upsert(getDocuments());

        const results = await ctx.client
            .collection(collection.name)
            .query(
                filter(field("title").regexpMatch("\\salchem", "i")).limit(10)
            );

        expect(results.map((doc) => doc._id)).toEqual(["alchemist"]);
    });
});