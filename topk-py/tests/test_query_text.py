import pytest
from topk_sdk import error
from topk_sdk.query import field, filter, fn, match, select

from . import ProjectContext
from .utils import dataset, doc_ids


def test_query_text_filter_single_term_disjunctive(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(match("love", field="summary")).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"pride", "gatsby"}


def test_query_text_filter_single_term_conjunctive(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(match("love", field="summary")).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"gatsby", "pride"}


def test_query_text_filter_two_terms_disjunctive(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(match("LOVE", "summary") | match("ring", "title")).topk(
            field("published_year"), 100, True
        )
    )

    assert doc_ids(result) == {"pride", "gatsby", "lotr"}


def test_query_text_filter_two_terms_conjunctive(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(match("LOVE", field="summary") & match("class", field="summary")).topk(
            field("published_year"), 100, True
        )
    )

    assert doc_ids(result) == {"pride"}


def test_query_text_filter_stop_word(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(match("the", field="summary")).topk(field("published_year"), 100, True)
    )

    assert len(result) == 0


def test_query_select_bm25_without_text_queries(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
            select(bm25_score=fn.bm25_score())
            .filter(field("_id") == "pride")
            .topk(field("bm25_score"), 100, True)
        )


def test_query_text_matches_single_term(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    for match_expr in [
        filter(field("summary").match_any("love")),
        filter(field("summary").match_all("love")),
    ]:
        result = ctx.client.collection(collection.name).query(
            match_expr.topk(field("published_year"), 100, True)
        )

        assert doc_ids(result) == {"pride", "gatsby"}


def test_query_text_match_all_two_terms(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("summary").match_all("love class")).topk(
            field("published_year"), 100, True
        )
    )

    assert doc_ids(result) == {"pride"}


def test_query_text_match_any_two_terms(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("summary").match_any("love ring")).topk(
            field("published_year"), 100, True
        )
    )

    assert doc_ids(result) == {"pride", "gatsby", "lotr"}


def test_query_text_matches_with_logical_expr(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter((field("summary").match_all("love class")) | (field("published_year") == 1925))
        .topk(field("published_year"), 10, True)
    )

    assert doc_ids(result) == {"pride", "gatsby"}


def test_query_text_matches_on_invalid_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
            filter(field("published_year").match_all("love class")).count()
        )
