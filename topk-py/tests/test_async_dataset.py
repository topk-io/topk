import asyncio
import pytest
from pathlib import Path
from topk_sdk import ListEntry, error

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
    dataset = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    metadata = {
        "title": "test"
    }

    response = await async_ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, metadata
    )
    print(response)

    assert response.handle is not None
    assert len(response.handle) > 0


@pytest.mark.asyncio
async def test_async_get_metadata(async_ctx: AsyncProjectContext):
    dataset = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    original_metadata = {"title": "test"}

    await async_ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, original_metadata
    )

    response = await async_ctx.client.dataset(dataset.name).get_metadata("doc1")
    print(response)
    assert response.metadata.get("title") == original_metadata.get("title")


@pytest.mark.asyncio
async def test_async_update_metadata(async_ctx: AsyncProjectContext):
    dataset = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    await async_ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    new_metadata = {"title": "Updated Title"}
    response = await async_ctx.client.dataset(dataset.name).update_metadata(
        "doc1", new_metadata
    )
    print(response)
    assert response.handle is not None


@pytest.mark.asyncio
async def test_async_delete_document(async_ctx: AsyncProjectContext):
    dataset = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    await async_ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    response = await async_ctx.client.dataset(dataset.name).delete("doc1")
    print(response)
    assert response.handle is not None


@pytest.mark.asyncio
async def test_async_check_handle(async_ctx: AsyncProjectContext):
    dataset = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    response = await async_ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    # Poll check_handle
    max_attempts = 120
    processed = False
    for _ in range(max_attempts):
        check_handle = await async_ctx.client.dataset(dataset.name).check_handle(response.handle)
        print(check_handle)
        processed = check_handle.processed
        if processed:
            break
        await asyncio.sleep(1)

    assert processed, "Handle was not processed within timeout"


@pytest.mark.asyncio
async def test_async_dataset_list(async_ctx: AsyncProjectContext):
    dataset = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    await async_ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, {"title": "test"}
    )

    entries: list[ListEntry] = []
    async for entry in async_ctx.client.dataset(dataset.name).list():
        entries.append(entry)
    assert len(entries) > 0
