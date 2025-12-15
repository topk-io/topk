import { u32List, f32List, i32List } from "../lib/data";
import { field, filter, select } from "../lib/query";
import { int } from "../lib/schema";
import { newProjectContext, ProjectContext } from "./setup";

describe("test_query_filter_unions", () => {
    const contexts: ProjectContext[] = [];

    function getContext(): ProjectContext {
        const ctx = newProjectContext();
        contexts.push(ctx);
        return ctx;
    }

    afterAll(async () => {
        await Promise.all(contexts.map((ctx) => ctx.deleteCollections()));
    });

    async function setupMinimalBooks(ctx: ProjectContext) {
        const collection = await ctx.createCollection("books", {
            published_year: int(),
        });

        await ctx.client.collection(collection.name).upsert([
            { _id: "mockingbird", user_ratings: u32List([9, 7, 10, 8]), published_year: 1960 },
            { _id: "1984", user_ratings: f32List([5.0, 3.5, 4.5, 4.0, -1.0]), published_year: 1949 },
            { _id: "pride", user_ratings: ["romantic", "classic", "good"], published_year: 1813 },
            { _id: "gatsby", user_ratings: "good book", published_year: 1925 },
            { _id: "catcher", user_ratings: f32List([4.5, 3.0, 3.5, 4.0, -1.0]), published_year: 1951 },
            { _id: "moby", user_ratings: i32List([-5, 2, -1, 1]), published_year: 1851 },
            { _id: "hobbit", user_ratings: u32List([5, 3, 4, 5, 2]), published_year: 1937 },
            { _id: "harry", user_ratings: 10, published_year: 1997 },
            { _id: "lotr", user_ratings: ["epic", "legendary", "good"], published_year: 1954 },
            { _id: "alchemist", user_ratings: u32List([8, 10, 9, 7]), published_year: 1988 },
        ]);
        return collection;
    }

    test("test_query_union_eq", async () => {
        const ctx = getContext();
        const collection = await setupMinimalBooks(ctx);

        const result = await ctx.client
            .collection(collection.name)
            .query(
                filter(field("user_ratings").eq(10)).topk(field("published_year"), 100, true)
            );

        expect(new Set(result.map((doc) => doc._id))).toEqual(new Set(["harry"]));
    });

    test("test_query_union_starts_with", async () => {
        const ctx = getContext();
        const collection = await setupMinimalBooks(ctx);

        const results = await ctx.client
            .collection(collection.name)
            .query(
                select({ _id: field("_id"), user_ratings: field("user_ratings") })
                    .filter(field("user_ratings").startsWith("good"))
                    .topk(field("published_year"), 100, true)
            );

        expect(new Set(results.map((doc) => doc._id))).toEqual(new Set(["gatsby", "pride", "lotr"]));
    });

    test("test_query_union_contains", async () => {
        const ctx = getContext();
        const collection = await setupMinimalBooks(ctx);

        for (const filterExpr of [
            field("user_ratings").contains(3),
            field("user_ratings").contains(3.0),
        ]) {
            const results = await ctx.client
                .collection(collection.name)
                .query(
                    select({ user_ratings: field("user_ratings") })
                        .filter(filterExpr)
                        .topk(field("published_year"), 100, true)
                );

            expect(new Set(results.map((doc) => doc._id))).toEqual(
                new Set(["catcher", "hobbit"])
            );
        }
    });

    test("test_query_union_contains_both_string_and_list", async () => {
        const ctx = getContext();
        const collection = await setupMinimalBooks(ctx);

        const results = await ctx.client
            .collection(collection.name)
            .query(
                select({ _id: field("_id"), user_ratings: field("user_ratings") })
                    .filter(field("user_ratings").contains("good"))
                    .topk(field("published_year"), 100, true)
            );

        expect(new Set(results.map((doc) => doc._id))).toEqual(
            new Set(["gatsby", "lotr", "pride"])
        );
    });
});
