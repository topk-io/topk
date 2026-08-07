import pytest
from topk_sdk.query import field, literal, select

from . import ProjectContext
from .utils import dataset, doc_ids_ordered


def test_query_sort_default_asc(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(_id=field("_id")).sort(field("published_year")).limit(3)
    )

    assert doc_ids_ordered(result) == ["pride", "moby", "gatsby"]


@pytest.mark.parametrize(
    "asc,expected",
    [
        (True, ["pride", "moby", "gatsby"]),
        (False, ["harry", "alchemist", "mockingbird"]),
    ],
)
def test_query_sort_asc(ctx: ProjectContext, asc: bool, expected: list[str]):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(_id=field("_id")).sort(field("published_year"), asc).limit(3)
    )

    assert doc_ids_ordered(result) == expected


def test_query_sort_asc_kwarg(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(_id=field("_id")).sort(field("published_year"), asc=False).limit(3)
    )

    assert doc_ids_ordered(result) == ["harry", "alchemist", "mockingbird"]


def test_query_sort_single_expr_list(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(_id=field("_id")).sort([(field("published_year"), "asc")]).limit(3)
    )

    assert doc_ids_ordered(result) == ["pride", "moby", "gatsby"]


def test_query_sort_multiple_exprs(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(_id=field("_id"))
        .sort([(literal(1), "desc"), (field("published_year"), "asc")])
        .limit(4)
    )

    assert doc_ids_ordered(result) == ["pride", "moby", "gatsby", "hobbit"]


def test_query_sort_multiple_exprs_with_asc_arg():
    with pytest.raises(ValueError):
        select(_id=field("_id")).sort([(field("published_year"), "asc")], True)  # type: ignore


def test_query_sort_multiple_exprs_with_asc_kwarg():
    with pytest.raises(ValueError):
        select(_id=field("_id")).sort([(field("published_year"), "asc")], asc=True)  # type: ignore


def test_query_sort_invalid_order():
    with pytest.raises((TypeError, ValueError)):
        select(_id=field("_id")).sort([(field("published_year"), "up")])  # type: ignore


def test_query_sort_missing_order():
    with pytest.raises((TypeError, ValueError)):
        select(_id=field("_id")).sort([field("published_year")])  # type: ignore
