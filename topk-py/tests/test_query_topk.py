from topk_sdk.query import field, select

from . import ProjectContext
from .utils import dataset


def test_query_topk_basic(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select("title").topk(field("published_year"), 5, True)
    )

    assert len(results) == 5
    # Should return oldest books first (ascending order)
    expected_ids = ["pride", "moby", "gatsby", "hobbit", "1984"]
    assert [doc["_id"] for doc in results] == expected_ids


def test_query_topk_ascending(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select("title").topk(field("published_year"), 5, False)
    )

    assert len(results) == 5
    # Should return newest books first (descending order)
    expected_ids = ["harry", "alchemist", "mockingbird", "lotr", "catcher"]
    assert [doc["_id"] for doc in results] == expected_ids


def test_query_topk_limit_greater_than_document_count(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select("title").topk(field("published_year"), 20, True)
    )

    # Should return all available documents
    assert len(results) == 10
    # First should be oldest
    assert results[0]["_id"] == "pride"
    # Last should be newest
    assert results[9]["_id"] == "harry"


def test_query_topk_empty_collection(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("empty_books"), schema={})

    results = ctx.client.collection(collection.name).query(
        select("title").topk(field("published_year"), 5, True)
    )

    assert len(results) == 0
