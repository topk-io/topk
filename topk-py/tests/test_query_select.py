from topk_sdk.data import f32_vector, f64_list, i32_list, i64_list, u8_vector, u32_list
from topk_sdk.query import field, fn, literal, match, select

from . import ProjectContext
from .utils import dataset, doc_ids


def test_query_select_literal(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(literal=literal(1.0))
        .filter(field("title") == "1984")
        .topk(field("published_year"), 100, True)
    )

    assert results == [{"_id": "1984", "literal": 1.0}]


def test_query_select_non_existing_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(literal=field("non_existing_field"))
        .filter(field("title") == "1984")
        .topk(field("published_year"), 100, True)
    )

    assert results == [{"_id": "1984"}]


def test_query_topk_limit(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select("title").topk(field("published_year"), 3, True)
    )
    assert len(results) == 3

    results = ctx.client.collection(collection.name).query(
        select("title").topk(field("published_year"), 2, True)
    )
    assert len(results) == 2

    results = ctx.client.collection(collection.name).query(
        select("title").topk(field("published_year"), 1, True)
    )
    assert len(results) == 1


def test_query_topk_asc(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select("published_year").topk(field("published_year"), 3, True)
    )

    assert results == [
        {"_id": "pride", "published_year": 1813},
        {"_id": "moby", "published_year": 1851},
        {"_id": "gatsby", "published_year": 1925},
    ]


def test_query_topk_desc(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select("published_year").topk(field("published_year"), 3, False)
    )

    assert results == [
        {"_id": "harry", "published_year": 1997},
        {"_id": "alchemist", "published_year": 1988},
        {"_id": "mockingbird", "published_year": 1960},
    ]


def test_query_select_bm25_score(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(bm25_score=fn.bm25_score())
        .filter(match("pride"))
        .topk(field("bm25_score"), 100, True)
    )

    assert doc_ids(results) == {"pride"}


def test_query_select_vector_distance(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance("summary_embedding", [2.0] * 16)
        ).topk(field("summary_distance"), 3, True)
    )

    assert doc_ids(results) == {"1984", "mockingbird", "pride"}


def test_query_select_null_field(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    ctx.client.collection(collection.name).upsert(
        [{"_id": "1984", "a": None}, {"_id": "pride"}]
    )

    results = ctx.client.collection(collection.name).query(
        select(a=field("a"), b=literal(1)).topk(field("b"), 100, True)
    )

    # Assert that `a` is null for all documents, even when not specified when upserting
    assert {doc.get("a") for doc in results} == {None, None}


def test_query_select_text_match(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(
            match_surveillance=field("summary").match_all("surveillance control mind"),
            match_love=field("summary").match_any("love class marriage"),
        )
        .filter((field("title") == "1984") | (field("_id") == "pride"))
        .topk(field("published_year"), 100, True)
    )

    assert results == [
        {"_id": "pride", "match_surveillance": False, "match_love": True},
        {"_id": "1984", "match_surveillance": True, "match_love": False},
    ]


def test_query_select_union(ctx: ProjectContext):
    # create collection
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    # upsert documents with different types in the same field
    lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "0", "rank": 0, "mixed": None},
            {"_id": "1", "rank": 1, "mixed": 1},
            {"_id": "2", "rank": 2, "mixed": 2},
            {"_id": "3", "rank": 3, "mixed": 3},
            {"_id": "4", "rank": 4, "mixed": 4},
            {"_id": "5", "rank": 5, "mixed": 5.0},
            {"_id": "6", "rank": 6, "mixed": 6.0},
            {"_id": "7", "rank": 7, "mixed": True},
            {"_id": "8", "rank": 8, "mixed": "hello"},
            {"_id": "9", "rank": 9, "mixed": u8_vector([1, 2, 3])},
            {"_id": "10", "rank": 10, "mixed": f32_vector([1.0, 2.0, 3.0])},
            {"_id": "11", "rank": 11, "mixed": bytes([1, 2, 3])},
            {"_id": "12", "rank": 12, "mixed": u32_list([17, 6, 1997])},
            {"_id": "13", "rank": 13, "mixed": i32_list([17, 6, 1997])},
            {"_id": "14", "rank": 14, "mixed": i64_list([17, 6, 1997])},
            {"_id": "15", "rank": 15, "mixed": f64_list([17.5, 6.5, 1997.5])},
            {"_id": "16", "rank": 16, "mixed": ["foo", "bar"]},
        ]
    )

    # wait for writes to be flushed
    ctx.client.collection(collection.name).count(lsn=lsn)

    results = ctx.client.collection(collection.name).query(
        select("mixed").topk(field("rank"), 100, True)
    )

    # Verify we have all the documents
    assert results == [
        {"_id": "0", "mixed": None},
        {"_id": "1", "mixed": 1},
        {"_id": "2", "mixed": 2},
        {"_id": "3", "mixed": 3},
        {"_id": "4", "mixed": 4},
        {"_id": "5", "mixed": 5.0},
        {"_id": "6", "mixed": 6.0},
        {"_id": "7", "mixed": True},
        {"_id": "8", "mixed": "hello"},
        {"_id": "9", "mixed": [1, 2, 3]},
        {"_id": "10", "mixed": [1.0, 2.0, 3.0]},
        {"_id": "11", "mixed": bytes([1, 2, 3])},
        {"_id": "12", "mixed": [17, 6, 1997]},
        {"_id": "13", "mixed": [17, 6, 1997]},
        {"_id": "14", "mixed": [17, 6, 1997]},
        {"_id": "15", "mixed": [17.5, 6.5, 1997.5]},
        {"_id": "16", "mixed": ["foo", "bar"]},
    ]
