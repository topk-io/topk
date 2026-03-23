import pytest
from pathlib import Path
from topk_sdk import SearchResult

from . import AsyncProjectContext


@pytest.mark.asyncio
@pytest.mark.xfail(reason="ctx")
async def test_async_search(async_ctx: AsyncProjectContext):
    dataset = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    upsert_resp = await async_ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, {}
    )

    await async_ctx.client.dataset(dataset.name).wait_for_handle(upsert_resp.handle)

    result: list[SearchResult] = await async_ctx.client.search(
        "technical", [dataset.name], top_k=10
    )

    assert len(result) > 0, "Expected at least one search result"


@pytest.mark.asyncio
@pytest.mark.xfail(reason="ctx")
async def test_async_search_stream(async_ctx: AsyncProjectContext):
    dataset = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    upsert_resp = await async_ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, {}
    )

    await async_ctx.client.dataset(dataset.name).wait_for_handle(upsert_resp.handle)

    stream = async_ctx.client.search_stream(
        "technical",
        [dataset.name],
        top_k=10,
    )

    results: list[SearchResult] = []
    async for r in stream:
        results.append(r)

    assert len(results) > 0, "Expected at least one search result from stream"
    assert all(isinstance(item, SearchResult) for item in results), (
        f"Expected all SearchResult items, got {[type(item) for item in results]}"
    )
