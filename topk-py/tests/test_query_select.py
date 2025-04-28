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
