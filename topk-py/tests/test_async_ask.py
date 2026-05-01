from pathlib import Path
import pytest

from topk_sdk import Answer

from . import AsyncProjectContext


@pytest.mark.asyncio
async def test_async_ask(async_ctx: AsyncProjectContext):
    dataset = await async_ctx.client.datasets().create(async_ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    handle = await async_ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})
    await async_ctx.client.dataset(dataset.name).wait_for_handle(handle)

    stream = async_ctx.client.ask("summarize", [dataset.name])

    answer_received = False
    async for message in stream:
        if isinstance(message, Answer):
            answer_received = True
            break

    assert answer_received, "Expected Answer in the stream"
