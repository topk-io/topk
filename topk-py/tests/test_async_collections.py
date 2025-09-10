import pytest
from topk_sdk import error

from . import AsyncProjectContext


@pytest.mark.asyncio
async def test_async_list_collections(async_ctx: AsyncProjectContext):
    c = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})
    response = await async_ctx.client.collections().list()
    assert c in response


@pytest.mark.asyncio
async def test_async_create_collection(async_ctx: AsyncProjectContext):
    c = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})
    collections = await async_ctx.client.collections().list()
    assert c in collections


@pytest.mark.asyncio
async def test_async_create_duplicate_collection(async_ctx: AsyncProjectContext):
    await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    with pytest.raises(error.CollectionAlreadyExistsError):
        await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})


@pytest.mark.asyncio
async def test_async_delete_non_existent_collection(async_ctx: AsyncProjectContext):
    with pytest.raises(error.CollectionNotFoundError):
        await async_ctx.client.collections().delete(async_ctx.scope("test"))


@pytest.mark.asyncio
async def test_async_delete_collection(async_ctx: AsyncProjectContext):
    c = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})
    await async_ctx.client.collections().delete(async_ctx.scope("test"))

    collections = await async_ctx.client.collections().list()
    assert c not in collections


@pytest.mark.asyncio
async def test_async_get_collection(async_ctx: AsyncProjectContext):
    # Test getting non-existent collection
    with pytest.raises(error.CollectionNotFoundError):
        await async_ctx.client.collections().get(async_ctx.scope("test"))

    # Create collection
    c = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})

    # Get collection
    collection = await async_ctx.client.collections().get(async_ctx.scope("test"))
    assert collection == c
