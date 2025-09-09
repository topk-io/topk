import pytest
from topk_sdk import error
from topk_sdk.query import field, filter, fn, not_, select, literal, match, min, max, abs

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

def test_query_abs(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(abs_year=abs(field("published_year") - 1990)).topk(
            field("abs_year"), 3, True
        )
    )

    # The 3 books closest to 1990
    assert result == [
        {"_id": "alchemist", "abs_year": 2},
        {"_id": "harry", "abs_year": 7},
        {"_id": "mockingbird", "abs_year": 30},
    ]

def test_query_topk_min_max(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(bm25_score=fn.bm25_score())
        .select(clamped_bm25_score=max(min(field("bm25_score"), 2.0), 1.6))
        .filter(match("millionaire love consequences dwarves", field="summary", weight=1.0, all=False))
        .topk(field("clamped_bm25_score"), 5, False)
    )

    assert len(result) == 4

    assert result[0]["_id"] == "gatsby"
    assert result[0]["clamped_bm25_score"] == 2.0

    assert result[1]["_id"] == "hobbit"
    clamped_score_1 = result[1]["clamped_bm25_score"]
    assert 1.6 <= clamped_score_1 <= 2.0

    assert result[2]["_id"] == "moby"
    clamped_score_2 = result[2]["clamped_bm25_score"]
    assert 1.6 <= clamped_score_2 <= 2.0

    assert result[3]["_id"] == "pride"
    assert result[3]["clamped_bm25_score"] == 1.6

def test_query_gt_and_lte_string(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").gt("moby") & (field("_id").lte("pride"))).topk(
            field("published_year"), 100, True
        )
    )

    assert doc_ids(result) == {"mockingbird", "pride"}

def test_query_min_string(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select("title", min_string=field("title").min("Oz"))
        .topk(field("published_year"), 2, True)
    )

    assert result == [
        {"_id": "pride", "title": "Pride and Prejudice", "min_string": "Oz"},
        {"_id": "moby", "title": "Moby-Dick", "min_string": "Moby-Dick"},
    ]
