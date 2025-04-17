import pytest
from topk_sdk import error

from . import ProjectContext
from .utils import dataset


def test_get_from_non_existent_collection(ctx: ProjectContext):
    # TODO: check if Python SDK returns CollectionNotFoundError or DocumentNotFoundError
    with pytest.raises((error.CollectionNotFoundError, error.DocumentNotFoundError)):
        ctx.client.collection("missing").get("doc1")


def test_get_non_existent_document(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.DocumentNotFoundError):
        ctx.client.collection(collection.name).get("missing")


def test_get_document(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # Get the expected document from our test dataset
    expected_doc = None
    for doc in dataset.books.docs():
        if doc["_id"] == "lotr":
            expected_doc = doc
            break

    doc = ctx.client.collection(collection.name).get("lotr")

    # Compare the relevant fields
    assert doc["_id"] == expected_doc["_id"]
    assert doc["title"] == expected_doc["title"]
    assert doc["published_year"] == expected_doc["published_year"]


def test_get_document_fields(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    doc = ctx.client.collection(collection.name).get(
        "lotr", fields=["title", "published_year"]
    )

    assert doc["_id"] == "lotr"
    assert doc["title"] == "The Lord of the Rings: The Fellowship of the Ring"
    assert doc["published_year"] == 1954
    assert "summary" not in doc
