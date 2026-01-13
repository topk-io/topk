import typing

from topk_sdk import data
from topk_sdk.schema import (
    binary_vector,
    f32_sparse_vector,
    f32_vector,
    i8_vector,
    int as int_type,
    keyword_index,
    matrix,
    multi_vector_index,
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
            "published_year": int_type().required(),
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
                "codes": [
                    "ISBN 978-0-06-093546-7",
                    "ISBN 0-06-093546-4",
                    "LCCN 60007854",
                    "UPC 025192354670",
                ],
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
                "tags": [
                    "dystopia",
                    "surveillance",
                    "totalitarianism",
                    "mind control",
                    "oppression",
                ],
                "codes": [
                    "1984",
                    "ISBN 978-0-452-28423-4",
                    "ISBN 0-452-28423-6",
                    "OCLC 70775700",
                    "DOI 10.1000/182",
                    "EAN 9780452284234",
                ],
                "reprint_years": data.u32_list(
                    [1950, 1954, 1956, 1961, 1984, 1990, 2003]
                ),
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
                "codes": [
                    "ISBN 978-0-7432-7356-5",
                    "ISBN 0-7432-7356-7",
                    "OCLC 60393320",
                    "ASIN B000FC0SIS",
                    "UPC 074327356709",
                    "LCCN 2002114274",
                ],
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
                "tags": [
                    "alienation",
                    "identity",
                    "rebellion",
                    "mid-20th-century",
                    "america",
                ],
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
                "codes": [
                    "ISBN 978-0-547-92821-0",
                    "ISBN 0-547-92821-2",
                    "OCLC 434394005",
                    "LCCN 2004558654",
                    "Barcode 0618346252",
                ],
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
                "codes": [
                    "ISBN 978-0-06-231500-7",
                    "ASIN 0062315005",
                    "OCLC 804616251",
                    "DOI 10.1234/alchemist",
                    "EAN 9780062315007",
                    "UPC 006231500719",
                    "LCCN 88675123",
                ],
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


class multi_vec:
    @staticmethod
    def cast(value_type: str, matrix: typing.List[typing.List[float]]) -> data.Matrix:
        """Cast f32 matrix data to a different value type.

        Similar to the Rust implementation, this function converts f32 matrix values
        to the target value type.
        """
        if value_type == "f32":
            return data.matrix(matrix, value_type="f32")
        elif value_type == "f16":
            return data.matrix(matrix, value_type="f16")
        elif value_type == "f8":
            return data.matrix(matrix, value_type="f8")
        elif value_type == "u8":
            # Convert f32 to u8: (abs(v) * 64.0).round() as u8
            u8_rows = [
                [int(round(abs(v) * 64.0)) for v in row] for row in matrix
            ]
            return data.matrix(u8_rows, value_type="u8")
        elif value_type == "i8":
            # Convert f32 to i8: (v * 64.0).round() and clip to i8 range
            i8_rows = [
                [multi_vec.clip_number_to_i8(int(round(v * 64.0))) for v in row] for row in matrix
            ]

            return data.matrix(i8_rows, value_type="i8")
        else:
            raise ValueError(f"Unsupported value_type: {value_type}")

    @staticmethod
    def clip_number_to_i8(number: int) -> int:
        return max(min(number, 127), -128)

    @staticmethod
    def setup(ctx: ProjectContext, value_type: str):
        collection = ctx.client.collections().create(
            ctx.scope(f"multi_vec_{value_type}"),
            schema=multi_vec.schema(value_type),
        )

        lsn = ""
        docs_list = multi_vec.docs(value_type)
        # Upsert in chunks of 4
        for i in range(0, len(docs_list), 4):
            chunk = docs_list[i : i + 4]
            lsn = ctx.client.collection(collection.name).upsert(chunk)

        count = ctx.client.collection(collection.name).count(lsn=lsn)
        assert count == len(docs_list)

        return collection

    @staticmethod
    def schema(value_type: str):
        return {
            "title": text().required().index(keyword_index()),
            "published_year": int_type().required(),
            "token_embeddings": matrix(7, value_type).index(  # type: ignore
                multi_vector_index("maxsim")
            ),
        }

    @staticmethod
    def docs(value_type: str):
        # !!! IMPORTANT !!!
        # Do not change the values of existing fields.
        # If you need to test new behavior which is not already covered by existing fields, add a new field.
        base_docs = [
            {
                "_id": "doc_0",
                "title": "To Kill a Mockingbird",
                "published_year": 1960,
                "token_embeddings": multi_vec.cast(
                    value_type,
                    [[0.9719, 0.132, 0.5612, -1.1843, -0.2115, 0.1455, -1.6471], [-0.1054, 1.6053, -0.0901, 0.5288, -0.6347, 0.9521, -0.8853]],
                ),
            },
            {
                "_id": "doc_1",
                "title": "1984",
                "published_year": 1949,
                "token_embeddings": multi_vec.cast(
                    value_type,
                    [
                        [0.4364, -0.4954, 0.3665, 1.5041, -1.4773, -0.701, -0.9732],
                        [-1.2239, 1.7501, 0.4089, 2.0643, -1.3925, 0.4711, -0.6247],
                    ],
                ),
            },
            {
                "_id": "doc_2",
                "title": "Pride and Prejudice",
                "published_year": 1813,
                "token_embeddings": multi_vec.cast(
                    value_type,
                    [[-2.6447, 0.3202, -0.5956, 0.6756, 1.0693, -1.0891, 1.0181]],
                ),
            },
            {
                "_id": "doc_3",
                "title": "The Great Gatsby",
                "published_year": 1925,
                "token_embeddings": multi_vec.cast(
                    value_type,
                    [
                        [0.1643, -0.2945, 1.3312, -0.3341, -0.3304, -0.029, -0.4426],
                        [-0.0975, -0.3696, -0.4106, -0.451, 0.4149, 0.8296, 0.3084],
                        [0.68, -0.182, -0.2652, -0.9707, -0.3433, 0.9671, -1.9293],
                    ],
                ),
            },
            {
                "_id": "doc_4",
                "title": "The Catcher in the Rye",
                "published_year": 1951,
                "token_embeddings": multi_vec.cast(
                    value_type,
                    [
                        [0.8748, 0.9163, 1.5845, -1.303, 1.7739, 0.9365, 1.2679],
                        [-0.6695, 0.5488, -1.0841, 0.3331, 0.5206, -1.2897, 0.6149],
                    ],
                ),
            },
            {
                "_id": "doc_5",
                "title": "Moby-Dick",
                "published_year": 1851,
                "token_embeddings": multi_vec.cast(
                    value_type,
                    [
                        [-0.6367, -0.5482, -1.2782, 1.0357, 1.044, -1.7687, 0.1703],
                        [-1.379, 0.0448, -0.7917, -1.693, -0.6001, 0.0598, 1.5035],
                        [1.968, -0.8128, 0.7871, -1.2036, -0.6445, -0.0684, 0.3407],
                    ],
                ),
            },
            {
                "_id": "doc_6",
                "title": "The Hobbit",
                "published_year": 1937,
                "token_embeddings": multi_vec.cast(
                    value_type,
                    [
                        [-0.4733, 0.5792, 0.1226, 0.4607, -0.3138, -0.2211, -0.1725],
                        [1.0828, -0.9416, 0.0848, 1.5135, 1.0625, 0.5481, 0.1558],
                        [0.71, -1.3281, 0.5986, -2.2235, -0.1252, -0.5943, 0.6521],
                    ],
                ),
            },
            {
                "_id": "doc_7",
                "title": "Harry Potter and the Sorcerer's Stone",
                "published_year": 1997,
                "token_embeddings": multi_vec.cast(
                    value_type,
                    [
                        [-0.4046, -0.1552, 2.632, -0.5471, -0.1942, -0.731, -1.1103],
                        [0.5813, 0.247, 0.0275, 0.0063, -2.4539, -0.2918, 1.1274],
                        [1.0666, 0.5535, 1.184, 0.5897, 1.2976, 1.2298, 2.6738],
                    ],
                ),
            },
            {
                "_id": "doc_8",
                "title": "The Lord of the Rings: The Fellowship of the Ring",
                "published_year": 1954,
                "token_embeddings": multi_vec.cast(
                    value_type,
                    [
                        [-0.2822, -0.4862, 2.0163, -1.4105, 2.1853, 0.583, 0.7119],
                        [-1.7254, 0.3599, 0.2296, 0.1091, -0.6483, 0.3901, -0.9539],
                        [-0.5296, -0.3046, 1.5027, 0.7712, -1.071, 0.7371, 0.1228],
                        [1.7048, 0.182, 0.3116, 0.7806, 0.2414, -0.7322, -0.1204],
                    ],
                ),
            },
            {
                "_id": "doc_9",
                "title": "The Alchemist",
                "published_year": 1988,
            },
        ]
        return base_docs
