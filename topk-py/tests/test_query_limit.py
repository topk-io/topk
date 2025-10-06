import pytest
from topk_sdk import error
from topk_sdk.query import field, fn, match, select

from . import ProjectContext
from .utils import dataset, doc_ids, doc_fields


def test_query_bare_limit(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(select().limit(100))

    assert len(result) == 10
    expected_ids = {str(doc["_id"]) for doc in dataset.books.docs()}
    assert doc_ids(result) == expected_ids


def test_query_limit_select_filter(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            _id=field("_id"),
            summary=field("summary"),
            is_recent=field("published_year") > 1980,
        )
        .filter(field("_id") <= "hobbit")
        .limit(3)
    )

    assert len(result) == 3
    assert doc_fields(result) == {"_id", "summary", "is_recent"}

    expected_ids = {"1984", "alchemist", "catcher", "gatsby", "harry", "hobbit"}
    for doc in result:
        assert doc["_id"] in expected_ids


def test_query_limit_with_bm25(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(bm25=fn.bm25_score()).filter(match("quest", all=True)).limit(10)
    )

    assert len(result) == 2
    assert doc_fields(result) == {"_id", "bm25"}
    assert {doc["_id"] for doc in result} == {"moby", "hobbit"}


def test_query_limit_vector_distance(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # skip_refine is implicitly true
    result_limit = ctx.client.collection(collection.name).query(
        select(title=field("title"))
        .select(summary_distance=fn.vector_distance("summary_embedding", [2.0] * 16))
        .limit(100)
    )

    # explicitly set skip_refine to true
    result_topk = ctx.client.collection(collection.name).query(
        select(title=field("title"))
        .select(
            summary_distance=fn.vector_distance("summary_embedding", [2.0] * 16, True)
        )
        .topk(field("summary_distance"), 100, True)
    )

    docs_limit = {doc["_id"]: doc for doc in result_limit}
    docs_topk = {doc["_id"]: doc for doc in result_topk}

    assert doc_fields(result_limit) == {"_id", "title", "summary_distance"}
    assert doc_fields(result_topk) == {"_id", "title", "summary_distance"}
    assert len(docs_limit) == len(docs_topk)

    # vector distance from limit should be the same as the vector distance from topk with skip_refine true
    assert docs_limit == docs_topk


def test_query_invalid_collectors(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    invalid_queries = [
        # topk + limit - multiple collectors
        select(title=field("title"))
        .topk(field("published_year"), 100, True)
        .limit(100),
        # limit + count - multiple collectors
        select(title=field("title")).limit(100).count(),
        # no collector
        select(title=field("title")).sort(field("published_year"), True),
        # multiple sorts
        select(title=field("title"))
        .sort(field("published_year"), True)
        .sort(field("published_year"), False),
        # topk + sort - effectively multiple sorts
        select(title=field("title"))
        .topk(field("published_year"), 100, True)
        .sort(field("published_year"), True),
    ]

    for q in invalid_queries:
        with pytest.raises(error.InvalidArgumentError):
            ctx.client.collection(collection.name).query(q)
