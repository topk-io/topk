from topk_sdk.query import field, filter

from . import ProjectContext
from .utils import dataset, doc_ids


def test_query_contains(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").contains("atch")).topk(field("published_year"), 100, False)
    )

    assert doc_ids(result) == {"catcher"}


def test_query_contains_empty(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").contains("")).topk(field("published_year"), 100, False)
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


def test_query_contains_no_match(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").contains("rubbish")).topk(
            field("published_year"), 100, False
        )
    )

    assert len(result) == 0
