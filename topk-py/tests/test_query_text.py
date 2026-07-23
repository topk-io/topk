import pytest
from topk_sdk import error
from topk_sdk.query import field, filter, fn, match, match_tokens, select, should
from topk_sdk.schema import keyword_index, text

from . import ProjectContext
from .utils import dataset, doc_ids, doc_ids_ordered


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


def test_query_text_filter_match_tokens_strings_only(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(match_tokens(["love", "class"], field="summary", all=True))
    )

    assert doc_ids(result) == {"pride"}


def test_query_text_filter_match_tokens_mixed_strings_and_tuples(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(match_tokens(["love", ("class", 1.0)], field="summary", all=True))
    )

    assert doc_ids(result) == {"pride"}


def test_query_text_filter_match_tokens_with_weights(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            match_tokens(
                [("wealth", 2.0), "love"],
                field="summary",
            )
        )
    )

    assert doc_ids(result) == {"gatsby", "pride"}


def test_query_text_filter_stop_word(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(match("the", field="summary")).topk(field("published_year"), 100, True)
    )

    assert len(result) == 0


def test_query_text_should_does_not_filter(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(bm25=fn.bm25_score())
        .filter(should("love", field="summary"))
        .sort(field("bm25"), False)
        .limit(100)
    )

    assert len(result) == 10
    assert doc_ids(result[:2]) == {"pride", "gatsby"}


def test_query_text_should_boosts_bm25_score(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # the should term only affects ranking - the result set is gated by "love" alone
    for boost, expected in [
        ("wealth", ["gatsby", "pride"]),
        ("marriage", ["pride", "gatsby"]),
    ]:
        result = ctx.client.collection(collection.name).query(
            select(bm25=fn.bm25_score())
            .filter(match("love", field="summary") & should(boost, field="summary"))
            .sort(field("bm25"), False)
            .limit(100)
        )

        assert doc_ids_ordered(result) == expected


def test_query_text_exact_keyword(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("exact_keyword"),
        schema={"tag": text().required().index(keyword_index(type="exact"))},
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "nyc", "tag": "New York City"},
            {"_id": "camel", "tag": "CamelCase"},
        ]
    )

    for token, expected in [
        ("New York City", {"nyc"}),
        ("York", set[str]()),
        ("new york city", set[str]()),
        ("CamelCase", {"camel"}),
        ("camelcase", set[str]()),
    ]:
        result = ctx.client.collection(collection.name).query(
            filter(match(token, field="tag")).limit(10),
            lsn=lsn,
        )

        assert doc_ids(result) == expected


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


def test_query_text_match_all_two_terms_tokenized(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("tags").match_all(["love", "class"])).topk(
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


def test_query_text_match_any_two_terms_tokenized(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("tags").match_any(["love", "elves"])).topk(
            field("published_year"), 100, True
        )
    )

    assert doc_ids(result) == {"pride", "gatsby", "lotr"}


def test_query_text_matches_with_logical_expr(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            (field("summary").match_all("love class"))
            | (field("published_year") == 1925)
        ).topk(field("published_year"), 10, True)
    )

    assert doc_ids(result) == {"pride", "gatsby"}


def test_query_text_matches_on_invalid_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
            filter(field("published_year").match_all("love class")).count()
        )


def test_invalid_truthiness():
    error_msg = "Using `and` or `or` keywords with Text expressions is not supported. Please use `&` or `|` instead."

    # `and`
    with pytest.raises(TypeError) as e:
        match("foo") and match("bar")
    assert error_msg in str(e.value)

    # `or`
    with pytest.raises(TypeError) as e:
        match("foo") or match("bar")
    assert error_msg in str(e.value)
