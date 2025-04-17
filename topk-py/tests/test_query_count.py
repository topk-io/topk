import pytest
from topk_sdk import error
from topk_sdk.query import field, filter

from . import ProjectContext
from .utils import dataset


def test_query_non_existent_collection(ctx: ProjectContext):
    with pytest.raises(error.CollectionNotFoundError):
        ctx.client.collection("missing").count()


def test_query_count(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).count()
    assert result == 10


def test_query_count_with_filter(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("published_year") <= 1950).count()
    )

    assert result[0]["_count"] == 5


def test_query_count_with_delete(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).count()
    assert result == 10

    lsn = ctx.client.collection(collection.name).delete(["lotr"])

    result = ctx.client.collection(collection.name).count(lsn=lsn)
    assert result == 9
