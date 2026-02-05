import pytest
from pathlib import Path
from topk_sdk import error

from . import ProjectContext


def test_upsert_file_to_non_existent_dataset(ctx: ProjectContext):
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"
    with pytest.raises(error.DatasetNotFoundError):
        ctx.client.dataset(ctx.scope("nonexistent")).upsert_file(
            "doc1", pdf_path, {}
        )


def test_upsert_file_pdf(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    metadata = {
        "title": "test"
    }

    handle = ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, metadata
    )
    assert handle is not None
    assert len(handle) > 0


def test_upsert_file_markdown_tuple(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test"))

    markdown_content = b"# Test Document\n\nThis is a test markdown file."
    markdown_tuple = ("test.md", markdown_content, "text/markdown")

    metadata = {
        "title": "test markdown"
    }

    handle = ctx.client.dataset(dataset.name).upsert_file(
        "doc1", markdown_tuple, metadata
    )
    assert handle is not None
    assert len(handle) > 0


def test_get_metadata(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    original_metadata = {"title": "test"}

    ctx.client.dataset(dataset.name).upsert_file(
        "doc1", pdf_path, original_metadata
    )

    retrieved_metadata = ctx.client.dataset(dataset.name).get_metadata("doc1")
    assert retrieved_metadata.get("title") == original_metadata.get("title")


def test_update_metadata(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    new_metadata = {"title": "Updated Title"}
    handle = ctx.client.dataset(dataset.name).update_metadata(
        "doc1", new_metadata
    )
    assert handle is not None


def test_delete_document(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    delete_handle = ctx.client.dataset(dataset.name).delete("doc1")
    assert delete_handle is not None


def test_check_handle(ctx: ProjectContext):
    dataset = ctx.client.datasets().create(ctx.scope("test"))
    pdf_path = Path(__file__).parent.parent.parent / "tests" / "pdfko.pdf"

    handle = ctx.client.dataset(dataset.name).upsert_file("doc1", pdf_path, {})

    # Poll check_handle
    max_attempts = 120
    processed = False
    for _ in range(max_attempts):
        processed = ctx.client.dataset(dataset.name).check_handle(handle)
        if processed:
            break
        import time
        time.sleep(1)

    assert processed, "Handle was not processed within timeout"
