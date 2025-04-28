import pytest
from topk_sdk.data import f32_vector, u8_vector
from topk_sdk.query import field, select

from . import ProjectContext


def test_query_select_union(ctx: ProjectContext):
    # create collection
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    # upsert documents with different types in the same field
    lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "0", "rank": 0, "mixed": None},
            {"_id": "1", "rank": 1, "mixed": 1},
            {"_id": "2", "rank": 2, "mixed": 2},
            {"_id": "3", "rank": 3, "mixed": 3},
            {"_id": "4", "rank": 4, "mixed": 4},
            {"_id": "5", "rank": 5, "mixed": 5.0},
            {"_id": "6", "rank": 6, "mixed": 6.0},
            {"_id": "7", "rank": 7, "mixed": True},
            {"_id": "8", "rank": 8, "mixed": "hello"},
            {"_id": "9", "rank": 9, "mixed": u8_vector([1, 2, 3])},
            {"_id": "10", "rank": 10, "mixed": f32_vector([1.0, 2.0, 3.0])},
            {"_id": "11", "rank": 11, "mixed": [1, 2, 3]},
            {"_id": "12", "rank": 12, "mixed": [1.0, 2.0, 3.0]},
            {"_id": "13", "rank": 13, "mixed": bytes([1, 2, 3])},
        ]
    )

    # wait for writes to be flushed
    ctx.client.collection(collection.name).count(lsn=lsn)

    results = ctx.client.collection(collection.name).query(
        select("mixed").topk(field("rank"), 100, True)
    )

    # Verify we have all the documents
    assert results == [
        {"_id": "0", "mixed": None},
        {"_id": "1", "mixed": 1},
        {"_id": "2", "mixed": 2},
        {"_id": "3", "mixed": 3},
        {"_id": "4", "mixed": 4},
        {"_id": "5", "mixed": 5.0},
        {"_id": "6", "mixed": 6.0},
        {"_id": "7", "mixed": True},
        {"_id": "8", "mixed": "hello"},
        {"_id": "9", "mixed": [1, 2, 3]},
        {"_id": "10", "mixed": [1.0, 2.0, 3.0]},
        {"_id": "11", "mixed": [1, 2, 3]},
        {"_id": "12", "mixed": [1.0, 2.0, 3.0]},
        {"_id": "13", "mixed": bytes([1, 2, 3])},
    ]
