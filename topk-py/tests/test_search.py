from pathlib import Path

from topk_sdk import SearchResult

from . import ProjectContext


def test_search(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test")).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    upsert_resp = ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    ctx.client.dataset(dataset.name).wait_for_handle(upsert_resp.handle)

    result: list[SearchResult] = ctx.client.search(
        "technical", [dataset.name], top_k=10
    )

    assert len(result) > 0, "Expected at least one search result"


def test_search_stream(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test")).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    upsert_resp = ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    ctx.client.dataset(dataset.name).wait_for_handle(upsert_resp.handle)

    stream = ctx.client.search_stream(
        "technical",
        [dataset.name],
        top_k=10,
    )

    results = list(stream)
    assert len(results) > 0, "Expected at least one search result from stream"
    assert all(isinstance(r, SearchResult) for r in results), f"Expected all SearchResult items, got {[type(r) for r in results]}"
