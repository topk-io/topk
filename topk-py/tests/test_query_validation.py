import pytest
from topk_sdk import error
from topk_sdk.query import field, select

from . import ProjectContext
from .utils import dataset


def test_query_topk_by_non_primitive(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError) as exc_info:
        ctx.client.collection(collection.name).query(
            select("title").topk(field("title"), 3, True)
        )
    assert "Input to SortWithLimit must produce primitive type, not String" in str(
        exc_info.value
    )


def test_query_topk_by_non_existing(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError) as exc_info:
        ctx.client.collection(collection.name).query(
            select("title").topk(field("non_existing_field"), 3, True)
        )
    assert "Input to SortWithLimit must produce primitive type, not Null" in str(
        exc_info.value
    )


def test_query_topk_limit_zero(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError) as exc_info:
        ctx.client.collection(collection.name).query(
            select("title").topk(field("published_year"), 0, True)
        )
    assert "Invalid argument: TopK k must be > 0" in str(exc_info.value)


def test_union_u32_and_binary(ctx: ProjectContext):
    # create collection
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    # upsert documents
    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "1", "num": 1}, {"_id": "11", "num": bytes([1, 2, 3])}]
    )

    # wait for writes to be flushed
    ctx.client.collection(collection.name).count(lsn=lsn)

    with pytest.raises(error.InvalidArgumentError) as exc_info:
        ctx.client.collection(collection.name).query(
            select("title").topk(field("num"), 100, True)
        )
    assert "Input to SortWithLimit must produce primitive type" in str(exc_info.value)
