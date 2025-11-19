import pytest
from topk_sdk import data, error
from topk_sdk.query import field, filter, select, literal, not_

from . import ProjectContext
from .utils import dataset, doc_ids


def test_query_regexp_match(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").regexp_match('^cat')).limit(10)
    )

    assert doc_ids(result) == {"catcher"}


def test_query_regexp_match_with_flags(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("title").regexp_match("\\salchem", "i")).limit(10),
    )

    assert doc_ids(result) == {"alchemist"}