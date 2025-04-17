import pytest
from topk_sdk.query import field, fn, literal, match, select, top_k

from . import ProjectContext
from .utils import dataset


def test_query_select_literal(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(literal=literal(1.0))
        .filter(field("title") == "1984")
        .top_k(field("published_year"), 100, True)
    )

    assert len(results) == 1
    assert results[0]["_id"] == "1984"
    assert results[0]["literal"] == 1.0


def test_query_select_non_existing_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(literal=field("non_existing_field"))
        .filter(field("title") == "1984")
        .top_k(field("published_year"), 100, True)
    )

    assert len(results) == 1
    assert results[0]["_id"] == "1984"
    assert "literal" not in results[0]


def test_query_topk_limit(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        top_k(field("published_year"), 3, True)
    )
    assert len(results) == 3

    results = ctx.client.collection(collection.name).query(
        top_k(field("published_year"), 2, True)
    )
    assert len(results) == 2

    results = ctx.client.collection(collection.name).query(
        top_k(field("published_year"), 1, True)
    )
    assert len(results) == 1


def test_query_topk_asc(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select("published_year").top_k(field("published_year"), 3, True)
    )

    assert len(results) == 3
    # The results should be sorted by published_year ascending
    assert results[0]["_id"] == "pride"
    assert results[0]["published_year"] == 1813
    assert results[1]["published_year"] > results[0]["published_year"]
    assert results[2]["published_year"] > results[1]["published_year"]


def test_query_topk_desc(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select("published_year").top_k(field("published_year"), 3, False)
    )

    assert len(results) == 3
    # The results should be sorted by published_year descending
    assert results[0]["_id"] == "harry"
    assert results[0]["published_year"] == 1997
    assert results[1]["published_year"] < results[0]["published_year"]
    assert results[2]["published_year"] < results[1]["published_year"]


def test_query_select_bm25_score(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(bm25_score=fn.bm25_score())
        .filter(match("pride"))
        .top_k(field("bm25_score"), 100, True)
    )

    assert len(results) == 1
    assert results[0]["_id"] == "pride"
    assert "bm25_score" in results[0]
    # We can't assert the exact score as it may change
    assert isinstance(results[0]["bm25_score"], float)


def test_query_select_vector_distance(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance("summary_embedding", [2.0] * 16)
        ).top_k(field("summary_distance"), 3, True)
    )

    assert len(results) == 3
    # We expect these specific documents in the result set
    result_ids = {doc["_id"] for doc in results}
    assert result_ids == {"1984", "mockingbird", "pride"}


def test_query_select_null_field(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    ctx.client.collection(collection.name).upsert(
        [{"_id": "1984", "a": None}, {"_id": "pride"}]
    )

    results = ctx.client.collection(collection.name).query(
        select(a=field("a"), b=literal(1)).top_k(field("b"), 100, True)
    )

    # Assert that `a` is null for all documents, even when not specified when upserting
    for doc in results:
        assert doc.get("a") is None
