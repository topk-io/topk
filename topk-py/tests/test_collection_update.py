import pytest
from topk_sdk import error, query
from topk_sdk.query import field, fn, select

from . import ProjectContext
from .utils import dataset


def test_update_non_existent_collection(ctx: ProjectContext):
    with pytest.raises(error.CollectionNotFoundError):
        ctx.client.collection("missing").update([{"_id": "one"}], False)


def test_update_batch(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "1", "foo": "bar1"},
            {"_id": "2", "foo": "bar2"},
            {"_id": "3", "foo": "bar3"},
            {"_id": "4", "foo": "bar4"},
        ]
    )
    assert lsn == "1"

    lsn = ctx.client.collection(collection.name).update(
        [
            {"_id": "2", "foo": "bar2.2", "baz": "foo"},
            {"_id": "3", "foo": None},
            {"_id": "4", "foo": "bar4.2"},
            {"_id": "5", "foo": "bar5"},  # missing id
        ],
        False,
    )
    assert lsn == "2"

    docs = ctx.client.collection(collection.name).get(["1", "2", "3", "4", "5"], lsn=lsn)

    assert len(docs) == 4
    assert docs["1"] == {"_id": "1", "foo": "bar1"}
    assert docs["2"] == {"_id": "2", "foo": "bar2.2", "baz": "foo"}
    assert docs["3"] == {"_id": "3"}
    assert docs["4"] == {"_id": "4", "foo": "bar4.2"}


def test_update_missing_id(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    # Upsert some docs
    lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "1", "foo": "bar1"},
            {"_id": "2", "foo": "bar2"},
        ]
    )
    assert lsn == "1"

    # Update non-existent doc
    new_lsn = ctx.client.collection(collection.name).update(
        [{"_id": "3", "foo": "bar3"}], False
    )
    assert new_lsn == ""

    # Check that no changes were made
    docs = ctx.client.collection(collection.name).get(["1", "2", "3"], lsn=lsn)

    assert len(docs) == 2
    assert docs["1"] == {"_id": "1", "foo": "bar1"}
    assert docs["2"] == {"_id": "2", "foo": "bar2"}


def test_update_missing_id_with_fail_on_missing(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    # Upsert some docs
    lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "1", "foo": "bar1"},
            {"_id": "2", "foo": "bar2"},
        ]
    )
    assert lsn == "1"

    # Update non-existent doc
    with pytest.raises(error.DocumentValidationError) as exc_info:
        ctx.client.collection(collection.name).update(
            [{"_id": "3", "foo": "bar3"}], True
        )
    assert "DocumentNotFound" in str(exc_info.value)
    assert 'doc_id: "3"' in str(exc_info.value)


def test_update_vector_index_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    res = ctx.client.collection(collection.name).query(
        select(
            dist=fn.vector_distance("summary_embedding", [2.0] * 16)
        ).filter(field("_id") == "1984").limit(1)
    )

    assert len(res) == 1
    assert res[0]["dist"] == 0.0

    lsn = ctx.client.collection(collection.name).update(
        [{"_id": "1984", "summary_embedding": [8.0] * 16}], True
    )

    res = ctx.client.collection(collection.name).query(
        select(
            dist=fn.vector_distance("summary_embedding", [2.0] * 16)
        ).filter(field("_id") == "1984").limit(1),
        lsn=lsn,
    )

    assert len(res) == 1
    expected_dist = pow(6.0, 2) * 16.0
    assert res[0]["dist"] == expected_dist


def test_update_semantic_index_field(ctx: ProjectContext):
    collection = dataset.semantic.setup(ctx)
    result = ctx.client.collection(collection.name).query(
        select(sim=fn.semantic_similarity("title", "dummy")).topk(
            field("sim"), 1, True
        )
    )

    assert len(result) == 1
    id = result[0]["_id"]

    lsn = ctx.client.collection(collection.name).update(
        [{"_id": id, "title": "foobarbaz"}], True
    )

    updated = ctx.client.collection(collection.name).query(
        select(
            title=field("title"),
            sim=fn.semantic_similarity("title", "dummy"),
        )
        .filter(field("_id") == id)
        .limit(1),
        lsn=lsn,
    )

    assert len(updated) == 1
    assert updated[0]["_id"] == id
    assert updated[0]["title"] == "foobarbaz"
    assert updated[0]["sim"] != result[0]["sim"]


def test_update_invalid_data_type(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.DocumentValidationError):
        ctx.client.collection(collection.name).update(
            [{"_id": "1984", "title": 1984}], True
        )


def test_update_missing_required_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.DocumentValidationError) as exc_info:
        ctx.client.collection(collection.name).update(
            [{"_id": "1984", "title": None}], True
        )
    assert "MissingField" in str(exc_info.value) or "required" in str(exc_info.value).lower()

