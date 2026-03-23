import pytest

from topk_sdk import Answer

from . import ProjectContext


@pytest.mark.xfail(reason="ctx")
def test_ask(ctx: ProjectContext):
    result = ctx.client.ask("summarize", [])

    assert isinstance(result, Answer), f"Expected Answer, got {type(result)}"
    assert len(result.facts) > 0, f"Expected at least 1 fact, got {len(result.facts)}"


@pytest.mark.xfail(reason="ctx")
def test_ask_stream(ctx: ProjectContext):
    stream = ctx.client.ask_stream("summarize", [])

    answer_received = False
    for message in stream:
        if isinstance(message, Answer):
            answer_received = True
            break

    assert answer_received, "Expected at least one Answer in the stream"
