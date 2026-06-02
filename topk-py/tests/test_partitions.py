import pytest

from topk_sdk.error import CollectionNotFoundError
from topk_sdk.query import field, select

from . import ProjectContext


def test_partition_upsert_isolation(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    default_lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "shared", "partition": "default"},
            {"_id": "only-default", "partition": "default"},
        ]
    )
    assert default_lsn == "1"

    p1_lsn = ctx.client.collection(collection.name, "p1").upsert(
        [
            {"_id": "shared", "partition": "p1"},
            {"_id": "only-p1", "partition": "p1"},
        ]
    )
    assert p1_lsn == "1"

    p2_lsn = ctx.client.collection(collection.name, partition="p2").upsert(
        [{"_id": "shared", "partition": "p2"}]
    )
    assert p2_lsn == "1"

    default_docs = ctx.client.collection(collection.name).get(
        ["shared", "only-default", "only-p1"], lsn=default_lsn
    )
    assert set(default_docs.keys()) == {"shared", "only-default"}
    assert default_docs["shared"]["partition"] == "default"

    p1_docs = ctx.client.collection(collection.name, "p1").get(
        ["shared", "only-default", "only-p1"], lsn=p1_lsn
    )
    assert set(p1_docs.keys()) == {"shared", "only-p1"}
    assert p1_docs["shared"]["partition"] == "p1"

    p2_docs = ctx.client.collection(collection.name, "p2").get(["shared"], lsn=p2_lsn)
    assert set(p2_docs.keys()) == {"shared"}
    assert p2_docs["shared"]["partition"] == "p2"


def test_list_partitions_empty(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    partitions = list(ctx.client.collection(collection.name).list_partitions())
    assert partitions == []


def test_list_partitions(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    ctx.client.collection(collection.name, "partition-a").upsert([{"_id": "doc-a"}])
    ctx.client.collection(collection.name, "partition-b").upsert([{"_id": "doc-b"}])

    partitions = sorted(
        ctx.client.collection(collection.name).list_partitions(),
        key=lambda p: p.name,
    )
    assert [p.name for p in partitions] == ["partition-a", "partition-b"]
    assert all(p.created_at for p in partitions)


def test_list_partitions_with_prefix(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    for name in ["foo", "foot", "bar"]:
        ctx.client.collection(collection.name, name).upsert([{"_id": "doc"}])

    partitions = ctx.client.collection(collection.name).list_partitions(prefix="foo")
    names = {p.name for p in partitions}
    assert names == {"foo", "foot"}


def test_list_partitions_excludes_default(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    ctx.client.collection(collection.name).upsert(
        [{"_id": "doc", "partition": "default"}]
    )
    ctx.client.collection(collection.name, "named-partition").upsert([{"_id": "doc"}])

    partitions = list(ctx.client.collection(collection.name).list_partitions())
    assert len(partitions) == 1
    assert partitions[0].name == "named-partition"


def test_delete_partition(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    ctx.client.collection(collection.name, "test-partition").upsert(
        [
            {"_id": "doc1", "value": "one"},
            {"_id": "doc2", "value": "two"},
        ]
    )

    partitions = list(ctx.client.collection(collection.name).list_partitions())
    assert len(partitions) == 1
    assert partitions[0].name == "test-partition"

    ctx.client.collection(collection.name).delete_partition("test-partition")

    partitions = list(ctx.client.collection(collection.name).list_partitions())
    assert partitions == []

    with pytest.raises(CollectionNotFoundError):
        ctx.client.collection(collection.name, "test-partition").count()


def test_delete_partition_does_not_affect_other_partitions(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    p1_lsn = ctx.client.collection(collection.name, "partition-a").upsert(
        [{"_id": "doc-a", "partition": "partition-a"}]
    )
    p2_lsn = ctx.client.collection(collection.name, "partition-b").upsert(
        [{"_id": "doc-a", "partition": "partition-b"}]
    )

    ctx.client.collection(collection.name).delete_partition("partition-a")

    partitions = list(ctx.client.collection(collection.name).list_partitions())
    assert len(partitions) == 1
    assert partitions[0].name == "partition-b"

    p2_docs = ctx.client.collection(collection.name, "partition-b").get(
        ["doc-a"], lsn=p2_lsn
    )
    assert p2_docs["doc-a"]["partition"] == "partition-b"
    assert p1_lsn == "1"
    assert p2_lsn == "1"


def test_upsert_creates_partition(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    lsn = ctx.client.collection(collection.name, "new-partition").upsert(
        [{"_id": "one", "value": "created"}]
    )
    assert lsn == "1"

    docs = ctx.client.collection(collection.name, "new-partition").get(["one"], lsn=lsn)
    assert docs["one"]["value"] == "created"

    count = ctx.client.collection(collection.name, "new-partition").count(lsn=lsn)
    assert count == 1


def test_query_non_existent_partition(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    with pytest.raises(CollectionNotFoundError):
        ctx.client.collection(collection.name, "missing-partition").count()

    with pytest.raises(CollectionNotFoundError):
        ctx.client.collection(collection.name, "missing-partition").get(["doc"])


def test_partition_with_invalid_name(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    with pytest.raises(Exception):
        ctx.client.collection(collection.name, "$foo&bar").upsert(
            [{"_id": "one", "value": "created"}]
        )


def test_partition_query_filter(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    p1_lsn = ctx.client.collection(collection.name, "p1").upsert(
        [
            {"_id": "doc1", "partition": "p1", "region": "us"},
            {"_id": "doc2", "partition": "p1", "region": "eu"},
            {"_id": "doc3", "partition": "p1", "region": "us"},
        ]
    )
    ctx.client.collection(collection.name, "p2").upsert(
        [
            {"_id": "doc1", "partition": "p2", "region": "us"},
            {"_id": "doc2", "partition": "p2", "region": "us"},
        ]
    )

    p1_results = ctx.client.collection(collection.name, "p1").query(
        select(partition=field("partition"))
        .filter(field("region").eq("us"))
        .limit(10),
        lsn=p1_lsn,
    )
    assert {doc["_id"] for doc in p1_results} == {"doc1", "doc3"}
    assert all(doc["partition"] == "p1" for doc in p1_results)

    default_results = ctx.client.collection(collection.name).query(
        select(partition=field("partition"))
        .filter(field("region").eq("us"))
        .limit(10)
    )
    assert default_results == []
