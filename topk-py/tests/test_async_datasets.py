import pytest
from topk_sdk import error

from . import AsyncProjectContext


@pytest.mark.asyncio
async def test_async_list_datasets(async_ctx: AsyncProjectContext):
    d = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    response = await async_ctx.client.datasets().list()
    assert d in response


@pytest.mark.asyncio
async def test_async_create_dataset(async_ctx: AsyncProjectContext):
    d = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    datasets = await async_ctx.client.datasets().list()
    assert d in datasets


@pytest.mark.asyncio
async def test_async_create_duplicate_dataset(async_ctx: AsyncProjectContext):
    await async_ctx.client.datasets().create(async_ctx.scope("test"))

    with pytest.raises(error.DatasetAlreadyExistsError):
        await async_ctx.client.datasets().create(async_ctx.scope("test"))


@pytest.mark.asyncio
async def test_async_delete_non_existent_dataset(async_ctx: AsyncProjectContext):
    with pytest.raises(error.DatasetNotFoundError):
        await async_ctx.client.datasets().delete(async_ctx.scope("test"))


@pytest.mark.asyncio
async def test_async_delete_dataset(async_ctx: AsyncProjectContext):
    d = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    await async_ctx.client.datasets().delete(async_ctx.scope("test"))

    datasets = await async_ctx.client.datasets().list()
    assert d not in datasets


@pytest.mark.asyncio
async def test_async_get_dataset(async_ctx: AsyncProjectContext):
    # Test getting non-existent dataset
    with pytest.raises(error.DatasetNotFoundError):
        await async_ctx.client.datasets().get(async_ctx.scope("test"))

    # Create dataset
    d = await async_ctx.client.datasets().create(async_ctx.scope("test"))

    # Get dataset
    dataset = await async_ctx.client.datasets().get(async_ctx.scope("test"))
    assert dataset == d
