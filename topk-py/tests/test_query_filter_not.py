from topk_sdk.query import field, filter, not_

from . import ProjectContext
from .utils import dataset, doc_ids


def test_query_not(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(not_(field("_id").contains("gatsby"))).topk(
            field("published_year"), 100, False
        )
    )

    assert doc_ids(result) == {
        "harry",
        "lotr",
        "1984",
        "mockingbird",
        "moby",
        "alchemist",
        "catcher",
        "hobbit",
        "pride",
    }
