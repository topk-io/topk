import pytest
from topk_sdk.query import field, filter, not_, select, literal

from . import ProjectContext
from .utils import dataset, doc_ids


def test_query_lte(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("published_year") <= 1950).topk(
            field("published_year"), 100, True
        )
    )

    assert doc_ids(result) == {"1984", "pride", "hobbit", "moby", "gatsby"}


def test_query_and(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            (field("published_year") <= 1950) & (field("published_year") >= 1948)
        ).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"1984"}

def test_query_is_null(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("nullable_embedding").is_null()).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"pride", "gatsby", "moby", "hobbit", "lotr", "alchemist"}

def test_query_is_not_null(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("nullable_embedding").is_not_null()).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"mockingbird", "1984", "catcher", "harry"}

def test_query_not(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(not_(field("_id").contains("gatsby"))).topk(
            field("published_year"), 100, False
        )
    )

    assert doc_ids(result) == {
        "harry",
        "lotr",
        "1984",
        "mockingbird",
        "moby",
        "alchemist",
        "catcher",
        "hobbit",
        "pride",
    }

def test_query_choose_literal(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            love_score=field("summary")
            .match_all("love")
            .choose(literal(2.0), literal(0.1))
        )
        .filter(field("love_score") > 1.0)
        .topk(field("love_score"), 10, False)
    )

    assert doc_ids(result) == {"pride", "gatsby"}

def test_query_choose_literal_and_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            love_score=field("summary")
            .match_all("love")
            .choose(field("published_year"), literal(10))
        )
        .topk(field("love_score"), 2, False)
    )

    assert result == [
        {"_id": "gatsby", "love_score": 1925},
        {"_id": "pride", "love_score": 1813},
    ]

def test_query_choose_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            love_score=field("summary")
            .match_all("love")
            .choose(field("published_year"), field("published_year") / 10)
        )
        .topk(field("love_score"), 3, False)
    )

    assert result == [
        {"_id": "gatsby", "love_score": 1925},
        {"_id": "pride", "love_score": 1813},
        {"_id": "harry", "love_score": 199},
    ]

def test_query_coalesce_nullable(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(importance=field("nullable_importance").coalesce(1.0))
        .filter(field("published_year") < 1900)
        .topk(field("published_year"), 3, False)
    )

    assert result == [
        {"_id": "moby", "importance": 5.0},
        {"_id": "pride", "importance": 1.0},
    ]

def test_query_coalesce_missing(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(importance=field("missing_field").coalesce(1.0))
        .filter(field("published_year") < 1900)
        .topk(field("published_year"), 3, False)
    )

    assert result == [
        {"_id": "moby", "importance": 1.0},
        {"_id": "pride", "importance": 1.0},
    ]

def test_query_coalesce_non_nullable(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(coalesced_year=field("published_year").coalesce(0))
        .filter(field("published_year") < 1900)
        .topk(field("published_year"), 3, False)
    )

    assert result == [
        {"_id": "moby", "coalesced_year": 1851},
        {"_id": "pride", "coalesced_year": 1813},
    ]
