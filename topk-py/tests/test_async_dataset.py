import asyncio
import pytest
from pathlib import Path
from topk_sdk import error

from . import AsyncProjectContext


@pytest.mark.asyncio
async def test_async_upsert_file_to_non_existent_dataset(async_ctx: AsyncProjectContext):
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"
    with pytest.raises(error.DatasetNotFoundError):
        await async_ctx.client.dataset(async_ctx.scope("nonexistent")).upsert_file(
            "doc1", pdf_path, {}
        )


@pytest.mark.asyncio
async def test_async_upsert_file_pdf(async_ctx: AsyncProjectContext):
    dataset = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    metadata = {
        "title": "test"
    }

    handle = await async_ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, metadata
    )
    assert handle is not None
    assert len(handle) > 0


@pytest.mark.asyncio
async def test_async_get_metadata(async_ctx: AsyncProjectContext):
    dataset = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    original_metadata = {"title": "test"}

    await async_ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, original_metadata
    )

    retrieved_metadata = await async_ctx.client.dataset(dataset.name).get_metadata("doc1")
    assert retrieved_metadata.get("title") == original_metadata.get("title")


@pytest.mark.asyncio
async def test_async_update_metadata(async_ctx: AsyncProjectContext):
    dataset = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    await async_ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    new_metadata = {"title": "Updated Title"}
    handle = await async_ctx.client.dataset(dataset.name).update_metadata(
        "doc1", new_metadata
    )
    assert handle is not None


@pytest.mark.asyncio
async def test_async_delete_document(async_ctx: AsyncProjectContext):
    dataset = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    await async_ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    delete_handle = await async_ctx.client.dataset(dataset.name).delete("doc1")
    assert delete_handle is not None


@pytest.mark.asyncio
async def test_async_check_handle(async_ctx: AsyncProjectContext):
    dataset = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    handle = await async_ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    # Poll check_handle
    max_attempts = 120
    processed = False
    for _ in range(max_attempts):
        processed = await async_ctx.client.dataset(dataset.name).check_handle(handle)
        if processed:
            break
        await asyncio.sleep(1)

    assert processed, "Handle was not processed within timeout"
