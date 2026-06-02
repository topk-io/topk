import pytest

from topk_sdk.error import CollectionNotFoundError
from topk_sdk.query import field, select

from . import AsyncProjectContext


@pytest.mark.asyncio
async def test_async_partition_upsert_isolation(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    default_lsn = await async_ctx.client.collection(collection.name).upsert(
        [
            {"_id": "shared", "partition": "default"},
            {"_id": "only-default", "partition": "default"},
        ]
    )
    assert default_lsn == "1"

    p1_lsn = await async_ctx.client.collection(collection.name, "p1").upsert(
        [
            {"_id": "shared", "partition": "p1"},
            {"_id": "only-p1", "partition": "p1"},
        ]
    )
    assert p1_lsn == "1"

    p2_lsn = await async_ctx.client.collection(collection.name, partition="p2").upsert(
        [{"_id": "shared", "partition": "p2"}]
    )
    assert p2_lsn == "1"

    default_docs = await async_ctx.client.collection(collection.name).get(
        ["shared", "only-default", "only-p1"], lsn=default_lsn
    )
    assert set(default_docs.keys()) == {"shared", "only-default"}
    assert default_docs["shared"]["partition"] == "default"

    p1_docs = await async_ctx.client.collection(collection.name, "p1").get(
        ["shared", "only-default", "only-p1"], lsn=p1_lsn
    )
    assert set(p1_docs.keys()) == {"shared", "only-p1"}
    assert p1_docs["shared"]["partition"] == "p1"

    p2_docs = await async_ctx.client.collection(collection.name, "p2").get(
        ["shared"], lsn=p2_lsn
    )
    assert set(p2_docs.keys()) == {"shared"}
    assert p2_docs["shared"]["partition"] == "p2"


@pytest.mark.asyncio
async def test_async_list_partitions_empty(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    partitions = [p async for p in async_ctx.client.collection(collection.name).list_partitions()]
    assert partitions == []


@pytest.mark.asyncio
async def test_async_list_partitions(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    await async_ctx.client.collection(collection.name, "partition-a").upsert([{"_id": "doc-a"}])
    await async_ctx.client.collection(collection.name, "partition-b").upsert([{"_id": "doc-b"}])

    partitions = sorted(
        [p async for p in async_ctx.client.collection(collection.name).list_partitions()],
        key=lambda p: p.name,
    )
    assert [p.name for p in partitions] == ["partition-a", "partition-b"]
    assert all(p.created_at for p in partitions)


@pytest.mark.asyncio
async def test_async_list_partitions_with_prefix(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    for name in ["foo", "foot", "bar"]:
        await async_ctx.client.collection(collection.name, name).upsert([{"_id": "doc"}])

    names = {
        p.name
        async for p in async_ctx.client.collection(collection.name).list_partitions(prefix="foo")
    }
    assert names == {"foo", "foot"}


@pytest.mark.asyncio
async def test_async_list_partitions_excludes_default(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    await async_ctx.client.collection(collection.name).upsert(
        [{"_id": "doc", "partition": "default"}]
    )
    await async_ctx.client.collection(collection.name, "named-partition").upsert([{"_id": "doc"}])

    partitions = [p async for p in async_ctx.client.collection(collection.name).list_partitions()]
    assert len(partitions) == 1
    assert partitions[0].name == "named-partition"


@pytest.mark.asyncio
async def test_async_delete_partition(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    await async_ctx.client.collection(collection.name, "test-partition").upsert(
        [
            {"_id": "doc1", "value": "one"},
            {"_id": "doc2", "value": "two"},
        ]
    )

    partitions = [p async for p in async_ctx.client.collection(collection.name).list_partitions()]
    assert len(partitions) == 1
    assert partitions[0].name == "test-partition"

    await async_ctx.client.collection(collection.name).delete_partition("test-partition")

    partitions = [p async for p in async_ctx.client.collection(collection.name).list_partitions()]
    assert partitions == []

    with pytest.raises(CollectionNotFoundError):
        await async_ctx.client.collection(collection.name, "test-partition").count()


@pytest.mark.asyncio
async def test_async_delete_partition_does_not_affect_other_partitions(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    p1_lsn = await async_ctx.client.collection(collection.name, "partition-a").upsert(
        [{"_id": "doc-a", "partition": "partition-a"}]
    )
    p2_lsn = await async_ctx.client.collection(collection.name, "partition-b").upsert(
        [{"_id": "doc-a", "partition": "partition-b"}]
    )

    await async_ctx.client.collection(collection.name).delete_partition("partition-a")

    partitions = [p async for p in async_ctx.client.collection(collection.name).list_partitions()]
    assert len(partitions) == 1
    assert partitions[0].name == "partition-b"

    p2_docs = await async_ctx.client.collection(collection.name, "partition-b").get(
        ["doc-a"], lsn=p2_lsn
    )
    assert p2_docs["doc-a"]["partition"] == "partition-b"
    assert p1_lsn == "1"
    assert p2_lsn == "1"


@pytest.mark.asyncio
async def test_async_upsert_creates_partition(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    lsn = await async_ctx.client.collection(collection.name, "new-partition").upsert(
        [{"_id": "one", "value": "created"}]
    )
    assert lsn == "1"

    docs = await async_ctx.client.collection(collection.name, "new-partition").get(
        ["one"], lsn=lsn
    )
    assert docs["one"]["value"] == "created"

    count = await async_ctx.client.collection(collection.name, "new-partition").count(lsn=lsn)
    assert count == 1


@pytest.mark.asyncio
async def test_async_query_non_existent_partition(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    with pytest.raises(CollectionNotFoundError):
        await async_ctx.client.collection(collection.name, "missing-partition").count()

    with pytest.raises(CollectionNotFoundError):
        await async_ctx.client.collection(collection.name, "missing-partition").get(["doc"])


@pytest.mark.asyncio
async def test_async_partition_with_invalid_name(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    with pytest.raises(Exception):
        await async_ctx.client.collection(collection.name, "$foo&bar").upsert(
            [{"_id": "one", "value": "created"}]
        )


@pytest.mark.asyncio
async def test_async_partition_query_filter(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    p1_lsn = await async_ctx.client.collection(collection.name, "p1").upsert(
        [
            {"_id": "doc1", "partition": "p1", "region": "us"},
            {"_id": "doc2", "partition": "p1", "region": "eu"},
            {"_id": "doc3", "partition": "p1", "region": "us"},
        ]
    )
    await async_ctx.client.collection(collection.name, "p2").upsert(
        [
            {"_id": "doc1", "partition": "p2", "region": "us"},
            {"_id": "doc2", "partition": "p2", "region": "us"},
        ]
    )

    p1_results = await async_ctx.client.collection(collection.name, "p1").query(
        select(partition=field("partition"))
        .filter(field("region").eq("us"))
        .limit(10),
        lsn=p1_lsn,
    )
    assert {doc["_id"] for doc in p1_results} == {"doc1", "doc3"}
    assert all(doc["partition"] == "p1" for doc in p1_results)

    default_results = await async_ctx.client.collection(collection.name).query(
        select(partition=field("partition"))
        .filter(field("region").eq("us"))
        .limit(10)
    )
    assert default_results == []
