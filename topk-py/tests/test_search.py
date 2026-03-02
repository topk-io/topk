import asyncio
import time
from pathlib import Path

import pytest
from topk_sdk import SearchResult

from . import AsyncProjectContext, ProjectContext


def test_search(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test")).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    upsert_resp = ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    # Wait for processing
    max_attempts = 120
    processed = False
    for _ in range(max_attempts):
        check_resp = ctx.client.dataset(dataset.name).check_handle(upsert_resp.handle)
        processed = check_resp.processed
        if processed:
            break
        time.sleep(1)

    assert processed, "Handle was not processed within timeout"

    sources = [{"dataset": dataset.name}]

    result = ctx.client.search(
        "ninth grade general education score",
        sources,
        filter=None,
        top_k=10,
    )

    print(result)
    assert isinstance(result, list), f"Expected list, got {type(result)}"
    assert len(result) > 0, "Expected at least one search result"
    assert all(isinstance(r, SearchResult) for r in result), f"Expected SearchResult items, got {[type(r) for r in result]}"
    assert result[0].doc_id == "doc1"
    assert result[0].dataset == dataset.name


# def test_search_stream(ctx: ProjectContext):
#     dataset = ctx.client.datasets().create(ctx.scope("test")).dataset
#     pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

#     upsert_resp = ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

#     # Wait for processing
#     max_attempts = 120
#     processed = False
#     for _ in range(max_attempts):
#         check_resp = ctx.client.dataset(dataset.name).check_handle(upsert_resp.handle)
#         processed = check_resp.processed
#         if processed:
#             break
#         time.sleep(1)

#     assert processed, "Handle was not processed within timeout"

#     sources = [{"dataset": dataset.name}]

#     stream = ctx.client.search_stream(
#         "ninth grade general education score",
#         sources,
#         filter=None,
#         top_k=10,
#     )

#     results = list(stream)
#     print(results)
#     assert len(results) > 0, "Expected at least one search result from stream"
#     assert all(isinstance(r, SearchResult) for r in results)


# @pytest.mark.asyncio
# async def test_async_search(async_ctx: AsyncProjectContext):
#     dataset = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
#     pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

#     upsert_resp = await async_ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

#     # Wait for processing
#     max_attempts = 120
#     processed = False
#     for _ in range(max_attempts):
#         check_resp = await async_ctx.client.dataset(dataset.name).check_handle(upsert_resp.handle)
#         processed = check_resp.processed
#         if processed:
#             break
#         await asyncio.sleep(1)

#     assert processed, "Handle was not processed within timeout"

#     sources = [{"dataset": dataset.name}]

#     result = await async_ctx.client.search(
#         "ninth grade general education score",
#         sources,
#         filter=None,
#         top_k=10,
#     )

#     print(result)
#     assert isinstance(result, list), f"Expected list, got {type(result)}"
#     assert len(result) > 0, "Expected at least one search result"
#     assert all(isinstance(r, SearchResult) for r in result)
#     assert result[0].doc_id == "doc1"
#     assert result[0].dataset == dataset.name


# @pytest.mark.asyncio
# async def test_async_search_stream(async_ctx: AsyncProjectContext):
#     dataset = (await async_ctx.client.datasets().create(async_ctx.scope("test"))).dataset
#     pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

#     upsert_resp = await async_ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

#     # Wait for processing
#     max_attempts = 120
#     processed = False
#     for _ in range(max_attempts):
#         check_resp = await async_ctx.client.dataset(dataset.name).check_handle(upsert_resp.handle)
#         processed = check_resp.processed
#         if processed:
#             break
#         await asyncio.sleep(1)

#     assert processed, "Handle was not processed within timeout"

#     sources = [{"dataset": dataset.name}]

#     stream = async_ctx.client.search_stream(
#         "ninth grade general education score",
#         sources,
#         filter=None,
#         top_k=10,
#     )

#     results = []
#     async for r in stream:
#         results.append(r)

#     print(results)
#     assert len(results) > 0, "Expected at least one search result from async stream"
#     assert all(isinstance(r, SearchResult) for r in results)
