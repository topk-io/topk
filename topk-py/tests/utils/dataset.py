from topk_sdk import data
from topk_sdk.schema import (
    binary_vector,
    f32_sparse_vector,
    f32_vector,
    i8_vector,
    int,
    keyword_index,
    semantic_index,
    text,
    u8_sparse_vector,
    u8_vector,
    vector_index,
    list,
)

from .. import ProjectContext, AsyncProjectContext


class books:
    @staticmethod
    def setup(ctx: ProjectContext):

        collection = ctx.client.collections().create(
            ctx.scope("books"),
            schema=books.schema(),
        )

        # Upsert sample books
        ctx.client.collection(collection.name).upsert(books.docs())

        return collection

    @staticmethod
    async def setup_async(ctx: AsyncProjectContext):

        collection = await ctx.client.collections().create(
            ctx.scope("books"),
            schema=books.schema(),
        )

        await ctx.client.collection(collection.name).upsert(books.docs())

        return collection


    @staticmethod
    def schema():
        return {
            "title": text().required().index(keyword_index()),
            "published_year": int().required(),
            "summary": text().required().index(keyword_index()),
            "summary_embedding": f32_vector(16)
            .required()
            .index(vector_index(metric="euclidean")),
            "nullable_embedding": f32_vector(16).index(
                vector_index(metric="euclidean")
            ),
            "scalar_embedding": u8_vector(16).index(vector_index(metric="euclidean")),
            "scalar_i8_embedding": i8_vector(16).index(
                vector_index(metric="euclidean")
            ),
            "binary_embedding": binary_vector(2).index(vector_index(metric="hamming")),
            "sparse_f32_embedding": f32_sparse_vector().index(
                vector_index(metric="dot_product")
            ),
            "sparse_u8_embedding": u8_sparse_vector().index(
                vector_index(metric="dot_product")
            ),
            "tags": list(value_type="text").index(keyword_index()),
            "codes": list(value_type="text"),
        }


    @staticmethod
    def docs():
        return [
            {
                "_id": "mockingbird",
                "title": "To Kill a Mockingbird",
                "published_year": 1960,
                "summary": "A young girl confronts racial injustice in the Deep South through the eyes of her lawyer father.",
                "summary_embedding": [1.0] * 16,
                "nullable_embedding": [1.0] * 16,
                "scalar_embedding": data.u8_vector([1] * 16),
                "scalar_i8_embedding": data.i8_vector([-100] * 16),
                "binary_embedding": data.binary_vector([0, 1]),
                "sparse_f32_embedding": data.f32_sparse_vector(
                    {0: 1.0, 1: 2.0, 2: 3.0}
                ),
                "sparse_u8_embedding": data.u8_sparse_vector({0: 1, 1: 2, 2: 3}),
                "tags": ["racism", "injustice", "girl", "father", "lawyer"],
                "codes": ["ISBN 978-0-06-093546-7", "ISBN 0-06-093546-4", "LCCN 60007854", "UPC 025192354670"],
                "reprint_years": data.u32_list([1966, 1988, 1999, 2002, 2015]),
                "user_ratings": data.u32_list([9, 7, 10, 8]),
            },
            {
                "_id": "1984",
                "title": "1984",
                "published_year": 1949,
                "summary": "A totalitarian regime uses surveillance and mind control to oppress its citizens.",
                "summary_embedding": [2.0] * 16,
                "nullable_embedding": [2.0] * 16,
                "scalar_embedding": data.u8_vector([2] * 16),
                "scalar_i8_embedding": data.i8_vector([-50] * 16),
                "binary_embedding": data.binary_vector([0, 3]),
                "sparse_f32_embedding": data.f32_sparse_vector(
                    {2: 1.0, 3: 2.0, 4: 3.0}
                ),
                "sparse_u8_embedding": data.u8_sparse_vector({2: 1, 3: 2, 4: 3}),
                "tags": ["dystopia", "surveillance", "totalitarianism", "mind control", "oppression"],
                "codes": ["1984", "ISBN 978-0-452-28423-4", "ISBN 0-452-28423-6", "OCLC 70775700", "DOI 10.1000/182", "EAN 9780452284234"],
                "reprint_years": data.u32_list([1950, 1954, 1956, 1961, 1984, 1990, 2003]),
                "user_ratings": data.f32_list([5.0, 3.5, 4.5, 4.0, -1.0]),
            },
            {
                "_id": "pride",
                "title": "Pride and Prejudice",
                "published_year": 1813,
                "summary": "A witty exploration of love, social class, and marriage in 19th-century England.",
                "summary_embedding": [3.0] * 16,
                "scalar_i8_embedding": data.i8_vector([0] * 16),
                "sparse_f32_embedding": data.f32_sparse_vector(
                    {3: 1.0, 4: 2.0, 5: 3.0}
                ),
                "sparse_u8_embedding": data.u8_sparse_vector({3: 1, 4: 2, 5: 3}),
                "tags": ["pride", "love", "romance", "class", "marriage", "prejudice"],
                "codes": ["ISBN 978-0-14-143951-8", "OCLC 934546789"],
                "reprint_years": data.u32_list([1966, 1972, 1985]),
                "user_ratings": ["romantic", "classic", "good"],
            },
            {
                "_id": "gatsby",
                "title": "The Great Gatsby",
                "published_year": 1925,
                "summary": "A mysterious millionaire navigates love and wealth in the Roaring Twenties.",
                "summary_embedding": [4.0] * 16,
                "scalar_i8_embedding": data.i8_vector([50] * 16),
                "sparse_f32_embedding": data.f32_sparse_vector(
                    {4: 1.0, 5: 2.0, 6: 3.0}
                ),
                "sparse_u8_embedding": data.u8_sparse_vector({4: 1, 5: 2, 6: 3}),
                "tags": ["love", "romance", "wealth", "marriage"],
                "codes": ["ISBN 978-0-7432-7356-5", "ISBN 0-7432-7356-7", "OCLC 60393320", "ASIN B000FC0SIS", "UPC 074327356709", "LCCN 2002114274"],
                "reprint_years": data.u32_list([1953, 1996, 2004]),
                "user_ratings": "good book",
            },
            {
                "_id": "catcher",
                "title": "The Catcher in the Rye",
                "published_year": 1951,
                "summary": "A rebellious teenager struggles with alienation and identity in mid-20th-century America.",
                "summary_embedding": [5.0] * 16,
                "nullable_embedding": [5.0] * 16,
                "scalar_embedding": data.u8_vector([5] * 16),
                "scalar_i8_embedding": data.i8_vector([100] * 16),
                "binary_embedding": data.binary_vector([0, 7]),
                "sparse_f32_embedding": data.f32_sparse_vector(
                    {5: 1.0, 6: 2.0, 7: 3.0}
                ),
                "sparse_u8_embedding": data.u8_sparse_vector({5: 1, 6: 2, 7: 3}),
                "tags": ["alienation", "identity", "rebellion", "mid-20th-century", "america"],
                "codes": ["ISBN 978-0-316-76917-4", "LCCN 51011564", "OCLC 287628"],
                "reprint_years": data.u32_list([1964, 1979, 1991, 2001, 2010]),
                "user_ratings": data.f32_list([4.5, 3.0, 3.5, 4.0, -1.0]),
            },
            {
                "_id": "moby",
                "title": "Moby-Dick",
                "published_year": 1851,
                "summary": "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.",
                "summary_embedding": [6.0] * 16,
                "sparse_f32_embedding": data.f32_sparse_vector(
                    {6: 1.0, 7: 2.0, 8: 3.0}
                ),
                "sparse_u8_embedding": data.u8_sparse_vector({6: 1, 7: 2, 8: 3}),
                "nullable_importance": 5.0,
                "tags": ["whale", "obsession", "tragedy", "sailing", "ocean"],
                "codes": data.string_list([]),
                "reprint_years": data.u32_list([]),
                "user_ratings": data.i32_list([-5, 2, -1, 1]),
            },
            {
                "_id": "hobbit",
                "title": "The Hobbit",
                "published_year": 1937,
                "summary": "A reluctant hobbit embarks on a quest to help a group of dwarves reclaim their mountain home.",
                "summary_embedding": [7.0] * 16,
                "sparse_f32_embedding": data.f32_sparse_vector(
                    {7: 1.0, 8: 2.0, 9: 3.0}
                ),
                "sparse_u8_embedding": data.u8_sparse_vector({7: 1, 8: 2, 9: 3}),
                "tags": ["hobbit", "dwarf", "quest", "home", "adventure"],
                "codes": data.string_list([]),
                "user_ratings": data.u32_list([5, 3, 4, 5, 2]),
            },
            {
                "_id": "harry",
                "title": "Harry Potter and the Sorcerer's Stone",
                "published_year": 1997,
                "summary": "A young wizard discovers his magical heritage and attends a school for witchcraft and wizardry.",
                "summary_embedding": [8.0] * 16,
                "nullable_embedding": [8.0] * 16,
                "scalar_embedding": data.u8_vector([8] * 16),
                "binary_embedding": data.binary_vector([0, 15]),
                "sparse_f32_embedding": data.f32_sparse_vector(
                    {8: 1.0, 9: 2.0, 10: 3.0}
                ),
                "sparse_u8_embedding": data.u8_sparse_vector({8: 1, 9: 2, 10: 3}),
                "tags": ["wizard", "magic", "sorcerer", "school", "witchcraft"],
                "codes": ["ISBN 978-0-439-70818-0", "UPC 043970818909"],
                "reprint_years": data.u32_list([1998, 1999, 2001, 2004, 2013, 2020]),
                "user_ratings": 10,
            },
            {
                "_id": "lotr",
                "title": "The Lord of the Rings: The Fellowship of the Ring",
                "published_year": 1954,
                "summary": "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
                "summary_embedding": [9.0] * 16,
                "scalar_i8_embedding": data.i8_vector([-100] * 16),
                "sparse_f32_embedding": data.f32_sparse_vector(
                    {9: 1.0, 10: 2.0, 11: 3.0}
                ),
                "sparse_u8_embedding": data.u8_sparse_vector({9: 1, 10: 2, 11: 3}),
                "tags": ["lord of the rings", "fellowship", "magic", "wizard", "elves"],
                "codes": ["ISBN 978-0-547-92821-0", "ISBN 0-547-92821-2", "OCLC 434394005", "LCCN 2004558654", "Barcode 0618346252"],
                "user_ratings": ["epic", "legendary", "good"],
            },
            {
                "_id": "alchemist",
                "title": "The Alchemist",
                "published_year": 1988,
                "summary": "A shepherd boy journeys to fulfill his destiny and discover the meaning of life.",
                "summary_embedding": [10.0] * 16,
                "sparse_f32_embedding": data.f32_sparse_vector(
                    {10: 1.0, 11: 2.0, 12: 3.0}
                ),
                "sparse_u8_embedding": data.u8_sparse_vector({10: 1, 11: 2, 12: 3}),
                "tags": ["journey", "destiny", "meaning of life", "alchemy", "soul"],
                "codes": ["ISBN 978-0-06-231500-7", "ASIN 0062315005", "OCLC 804616251", "DOI 10.1234/alchemist", "EAN 9780062315007", "UPC 006231500719", "LCCN 88675123"],
                "reprint_years": data.u32_list([1993, 2005, 2014]),
                "user_ratings": data.u32_list([8, 10, 9, 7]),
            },
        ]


class semantic:
    @staticmethod
    def setup(ctx: ProjectContext):
        schema = {
            "title": text().required().index(semantic_index(model="dummy")),
            "summary": text().required().index(semantic_index(model="dummy")),
        }

        collection = ctx.client.collections().create(
            ctx.scope("semantic"),
            schema=schema,
        )

        # Upsert sample books
        ctx.client.collection(collection.name).upsert(semantic.docs())

        return collection

    @staticmethod
    def docs():
        return [
            {
                "_id": "mockingbird",
                "title": "To Kill a Mockingbird",
                "published_year": 1960,
                "summary": "A young girl confronts racial injustice in the Deep South through the eyes of her lawyer father.",
                "nullable_importance": 2.0,
            },
            {
                "_id": "1984",
                "title": "1984",
                "published_year": 1949,
                "summary": "A totalitarian regime uses surveillance and mind control to oppress its citizens.",
            },
            {
                "_id": "pride",
                "title": "Pride and Prejudice",
                "published_year": 1813,
                "summary": "A witty exploration of love, social class, and marriage in 19th-century England.",
            },
            {
                "_id": "gatsby",
                "title": "The Great Gatsby",
                "published_year": 1925,
                "summary": "A mysterious millionaire navigates love and wealth in the Roaring Twenties.",
            },
            {
                "_id": "catcher",
                "title": "The Catcher in the Rye",
                "published_year": 1951,
                "summary": "A rebellious teenager struggles with alienation and identity in mid-20th-century America.",
            },
            {
                "_id": "moby",
                "title": "Moby-Dick",
                "published_year": 1851,
                "summary": "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.",
            },
            {
                "_id": "hobbit",
                "title": "The Hobbit",
                "published_year": 1937,
                "summary": "A reluctant hobbit embarks on a quest to help a group of dwarves reclaim their mountain home.",
            },
            {
                "_id": "harry",
                "title": "Harry Potter and the Sorcerer's Stone",
                "published_year": 1997,
                "summary": "A young wizard discovers his magical heritage and attends a school for witchcraft and wizardry.",
            },
            {
                "_id": "lotr",
                "title": "The Lord of the Rings: The Fellowship of the Ring",
                "published_year": 1954,
                "summary": "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
            },
            {
                "_id": "alchemist",
                "title": "The Alchemist",
                "published_year": 1988,
                "summary": "A shepherd boy journeys to fulfill his destiny and discover the meaning of life.",
            },
        ]
