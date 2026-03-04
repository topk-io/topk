import pytest
from pathlib import Path
from topk_sdk import ListEntry, error

from . import ProjectContext

def test_upsert_file_to_non_existent_dataset(ctx: ProjectContext):
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"
    with pytest.raises(error.DatasetNotFoundError):
        ctx.client.dataset(ctx.scope("nonexistent")).upsert_file(
            "doc1", pdf_path, {}
        )


def test_upsert_file_pdf(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test")).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    metadata = {
        "title": "test"
    }

    resp = ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, metadata
    )
    print(resp)
    assert resp.handle is not None
    assert len(resp.handle) > 0


def test_upsert_file_markdown_tuple(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test")).dataset

    markdown_content = b"# Test Document\n\nThis is a test markdown file."
    markdown_tuple = ("test.md", markdown_content, "text/markdown")

    metadata = {
        "title": "test markdown"
    }

    resp = ctx.client.dataset(dataset.name).upsert_file(
        "doc1", markdown_tuple, metadata
    )
    print(resp)
    assert resp.handle is not None
    assert len(resp.handle) > 0


def test_get_metadata(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test")).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    original_metadata = {"title": "test"}

    ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, original_metadata
    )

    resp = ctx.client.dataset(dataset.name).get_metadata("doc1")
    print(resp)
    assert resp.metadata.get("title") == original_metadata.get("title")


def test_update_metadata(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test")).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    new_metadata = {"title": "Updated Title"}
    resp = ctx.client.dataset(dataset.name).update_metadata(
        "doc1", new_metadata
    )
    print(resp)
    assert resp.handle is not None


def test_delete_document(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test")).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    resp = ctx.client.dataset(dataset.name).delete("doc1")
    print(resp)
    assert resp.handle is not None


def test_check_handle(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test")).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    upsert_resp = ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    # Poll check_handle
    max_attempts = 120
    processed = False
    for _ in range(max_attempts):
        check_resp = ctx.client.dataset(dataset.name).check_handle(upsert_resp.handle)
        print(check_resp)
        processed = check_resp.processed
        if processed:
            break
        import time
        time.sleep(1)

    assert processed, "Handle was not processed within timeout"


def test_dataset_list(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test")).dataset
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {"title": "test"})

    entries = [e for e in ctx.client.dataset(dataset.name).list() if e is not None]
    assert len(entries) > 0
