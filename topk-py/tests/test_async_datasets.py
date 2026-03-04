import pytest
from topk_sdk import error

from . import AsyncProjectContext


@pytest.mark.asyncio
async def test_async_list_datasets(async_ctx: AsyncProjectContext):
    create_resp = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    print(create_resp)
    d = create_resp.dataset
    response = await async_ctx.client.datasets().list()
    print(response)
    assert d in response.datasets


@pytest.mark.asyncio
async def test_async_create_dataset(async_ctx: AsyncProjectContext):
    create_resp = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    print(create_resp)
    d = create_resp.dataset
    response = await async_ctx.client.datasets().list()
    print(response)
    assert d in response.datasets


@pytest.mark.asyncio
async def test_async_create_duplicate_dataset(async_ctx: AsyncProjectContext):
    response = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    print(response)

    with pytest.raises(error.DatasetAlreadyExistsError):
        await async_ctx.client.datasets().create(async_ctx.scope("test"))


@pytest.mark.asyncio
async def test_async_delete_non_existent_dataset(async_ctx: AsyncProjectContext):
    with pytest.raises(error.DatasetNotFoundError):
        await async_ctx.client.datasets().delete(async_ctx.scope("test"))


@pytest.mark.asyncio
async def test_async_delete_dataset(async_ctx: AsyncProjectContext):
    d = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
    response = await async_ctx.client.datasets().delete(async_ctx.scope("test"))
    print(response.request_id)

    response = await async_ctx.client.datasets().list()
    print(response)
    assert d not in response.datasets


@pytest.mark.asyncio
async def test_async_get_dataset(async_ctx: AsyncProjectContext):
    # Test getting non-existent dataset
    with pytest.raises(error.DatasetNotFoundError):
        await async_ctx.client.datasets().get(async_ctx.scope("test"))

    # Create dataset
    d = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset

    # Get dataset
    response = await async_ctx.client.datasets().get(async_ctx.scope("test"))
    print(response)
    assert response.dataset == d
