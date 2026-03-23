import pytest
from topk_sdk import Answer

from . import AsyncProjectContext


@pytest.mark.asyncio
@pytest.mark.xfail(reason="ctx")
async def test_async_ask(async_ctx: AsyncProjectContext):
    result = await async_ctx.client.ask("summarize", [])

    assert isinstance(result, Answer), f"Expected Answer, got {type(result)}"
    assert len(result.facts) > 0, f"Expected at least 1 fact, got {len(result.facts)}"


@pytest.mark.asyncio
@pytest.mark.xfail(reason="ctx")
async def test_async_ask_stream(async_ctx: AsyncProjectContext):
    stream = async_ctx.client.ask_stream("summarize", [])

    answer_received = False
    async for message in stream:
        if isinstance(message, Answer):
            answer_received = True
            break

    assert answer_received, "Expected at least one Answer in the stream"
