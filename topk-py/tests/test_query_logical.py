import pytest
from topk_sdk.query import field, filter

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
