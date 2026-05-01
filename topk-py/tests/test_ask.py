from pathlib import Path

from topk_sdk import Answer

from . import ProjectContext


def test_ask(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    handle = ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})
    ctx.client.dataset(dataset.name).wait_for_handle(handle)

    stream = ctx.client.ask("summarize", [dataset.name])

    answer_received = False
    for message in stream:
        if isinstance(message, Answer):
            answer_received = True
            break

    assert answer_received, "Expected at Answer in the stream"
