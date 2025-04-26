import pytest
from topk_sdk import error

from . import ProjectContext
from .utils import dataset


def test_get_from_non_existent_collection(ctx: ProjectContext):
    with pytest.raises(error.CollectionNotFoundError):
        ctx.client.collection("missing").get(["doc1"])


def test_get_non_existent_document(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    docs = ctx.client.collection(collection.name).get(["missing"])
    assert docs == {}


def test_get_document(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    docs = ctx.client.collection(collection.name).get(["lotr"])

    assert docs == {
        "lotr": {
            "_id": "lotr",
            "title": "The Lord of the Rings: The Fellowship of the Ring",
            "published_year": 1954,
            "summary": "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
            "summary_embedding": [9.0] * 16,
        }
    }


def test_get_multiple_documents(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    docs = ctx.client.collection(collection.name).get(["lotr", "moby"])

    assert docs == {
        "lotr": {
            "_id": "lotr",
            "title": "The Lord of the Rings: The Fellowship of the Ring",
            "published_year": 1954,
            "summary": "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
            "summary_embedding": [9.0] * 16,
        },
        "moby": {
            "_id": "moby",
            "title": "Moby-Dick",
            "published_year": 1851,
            "summary": "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.",
            "summary_embedding": [6.0] * 16,
        },
    }


def test_get_document_fields(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    docs = ctx.client.collection(collection.name).get(
        ["lotr"],
        fields=["title", "published_year"],
    )

    assert docs == {
        "lotr": {
            "_id": "lotr",
            "title": "The Lord of the Rings: The Fellowship of the Ring",
            "published_year": 1954,
        }
    }
