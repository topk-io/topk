import time
from pathlib import Path
from topk_sdk import FinalAnswer

from . import ProjectContext


def test_ask(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    handle = ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    # Wait for processing
    max_attempts = 120

    processed = False
    for _ in range(max_attempts):
        processed = ctx.client.dataset(dataset.name).check_handle(handle)
        if processed:
            break
        time.sleep(1)

    assert processed, "Handle was not processed within timeout"

    sources = [{"dataset": dataset.name}]

    result = ctx.client.ask(
        "What score must general education students achieve who first entered ninth grade in 1997?",
        sources,
        filter=None
    )

    print(result)

    assert isinstance(result, FinalAnswer), f"Expected FinalAnswer, got {type(result)}"
    found_55 = any("55" in fact.fact for fact in result.facts)
    assert found_55, f"Expected '55' in facts, but got: {[f.fact for f in result.facts]}"


# def test_ask_stream(ctx: ProjectContext):
#     dataset = ctx.client.datasets().create(ctx.scope("test"))
#     pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

#     handle = ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

#     # Wait for processing
#     max_attempts = 120
#     processed = False
#     for _ in range(max_attempts):
#         processed = ctx.client.dataset(dataset.name).check_handle(handle)
#         if processed:
#             break
#         time.sleep(1)

#     assert processed, "Handle was not processed within timeout"

#     sources = [{"dataset": dataset.name}]

#     stream = ctx.client.ask_stream(
#         "What score must general education students achieve who first entered ninth grade in 1997?",
#         sources,
#         filter=None
#     )

#     message_count = 0
#     final_answer_received = False

#     for message in stream:
#         message_count += 1
#         if isinstance(message, FinalAnswer):
#             final_answer_received = True
#             found_55 = any(
#                 "55" in fact.fact for fact in message.facts
#             )
#             assert found_55, f"Expected '55' in facts, but got: {[f.fact for f in message.facts]}"
#             break

#     assert final_answer_received, f"Should receive a final answer (received {message_count} messages)"
