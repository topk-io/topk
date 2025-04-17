import pytest
from topk_sdk.query import field, filter

from . import ProjectContext
from .utils import dataset, doc_ids


def test_query_lte(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("published_year") <= 1950).top_k(
            field("published_year"), 100, True
        )
    )

    assert doc_ids(result) == {"1984", "pride", "hobbit", "moby", "gatsby"}


def test_query_and(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            (field("published_year") <= 1950) & (field("published_year") >= 1948)
        ).top_k(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"1984"}
