from topk_sdk.query import field, filter

from . import ProjectContext
from .utils import dataset


def test_query_filter_basic(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        filter(field("published_year") <= 1950).topk(
            field("published_year"), 100, False
        )
    )

    assert [doc["_id"] for doc in results] == [
        "1984",
        "hobbit",
        "gatsby",
        "moby",
        "pride",
    ]


def test_query_filter_with_lte_operator(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        filter(field("published_year").lte(1950)).topk(
            field("published_year"), 100, False
        )
    )

    assert [doc["_id"] for doc in results] == [
        "1984",
        "hobbit",
        "gatsby",
        "moby",
        "pride",
    ]


def test_query_filter_with_gte_operator(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        filter(field("published_year") >= 1950).topk(field("published_year"), 100, True)
    )

    # The actual count depends on the dataset, so we'll check the count and some key documents
    assert len(results) == 5
    doc_ids = [doc["_id"] for doc in results]
    assert "catcher" in doc_ids
    assert "1984" in doc_ids


def test_query_filter_with_eq_operator(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        filter(field("published_year") == 1951).topk(field("published_year"), 100, True)
    )

    assert [doc["_id"] for doc in results] == ["catcher"]


def test_query_filter_with_ne_operator(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        filter(field("published_year") != 1951).topk(field("published_year"), 100, True)
    )

    assert len(results) == 9
    assert "catcher" not in [doc["_id"] for doc in results]
