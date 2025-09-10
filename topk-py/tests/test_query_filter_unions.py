from topk_sdk.query import field, filter, select

from . import ProjectContext
from .utils import dataset, doc_ids


def test_query_union_eq(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("user_ratings") == 10).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"harry"}


def test_query_union_starts_with(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select("_id", "user_ratings")
        .filter(field("user_ratings").starts_with("good"))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(results) == {"gatsby"}


def test_query_union_contains(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    for filter_expr in [
        field("user_ratings").contains(3),
        field("user_ratings").contains(3.0),
    ]:
        results = ctx.client.collection(collection.name).query(
            select(user_ratings=field("user_ratings"))
            .filter(filter_expr)
            .topk(field("published_year"), 100, True)
        )

        assert doc_ids(results) == {"catcher", "hobbit"}


def test_query_union_contains_both_string_and_list(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(_id=field("_id"), user_ratings=field("user_ratings"))
        .filter(field("user_ratings").contains("good"))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(results) == {"gatsby", "lotr", "pride"}
