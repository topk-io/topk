from topk_sdk.query import field, filter

from . import ProjectContext
from .utils import dataset, doc_ids


def test_query_starts_with(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").starts_with("cat")).limit(100)
    )

    assert [doc["_id"] for doc in result] == ["catcher"]


def test_query_starts_with_empty(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").starts_with("")).limit(100)
    )

    assert doc_ids(result) == {
        "gatsby",
        "catcher",
        "moby",
        "mockingbird",
        "alchemist",
        "harry",
        "lotr",
        "pride",
        "1984",
        "hobbit",
    }


def test_query_starts_with_non_existent_prefix(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").starts_with("foobarbaz")).limit(100)
    )

    assert len(result) == 0
