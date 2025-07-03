import pytest
from topk_sdk import data, error

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
            "sparse_f32_embedding": {9: 1.0, 10: 2.0, 11: 3.0},
            "sparse_u8_embedding": {9: 1, 10: 2, 11: 3},
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
            "sparse_f32_embedding": {9: 1.0, 10: 2.0, 11: 3.0},
            "sparse_u8_embedding": {9: 1, 10: 2, 11: 3},
        },
        "moby": {
            "_id": "moby",
            "title": "Moby-Dick",
            "published_year": 1851,
            "summary": "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.",
            "summary_embedding": [6.0] * 16,
            "sparse_f32_embedding": {6: 1.0, 7: 2.0, 8: 3.0},
            "sparse_u8_embedding": {6: 1, 7: 2, 8: 3},
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


def test_get_empty_document_list(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # Empty document list should raise an error
    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).get([])


def test_get_document_with_lsn(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # First upsert to get an LSN - use a document that matches the schema
    lsn = ctx.client.collection(collection.name).upsert(
        [
            {
                "_id": "test_doc",
                "title": "Test Document",
                "published_year": 2023,
                "summary": "A test document for LSN testing",
                "summary_embedding": [1.0] * 16,
                "sparse_f32_embedding": {1: 1.0},
                "sparse_u8_embedding": data.u8_sparse_vector({1: 1}),
            }
        ]
    )

    # Get the document with the LSN
    docs = ctx.client.collection(collection.name).get(["test_doc"], lsn=lsn)

    assert docs == {
        "test_doc": {
            "_id": "test_doc",
            "title": "Test Document",
            "published_year": 2023,
            "summary": "A test document for LSN testing",
            "summary_embedding": [1.0] * 16,
            "sparse_f32_embedding": {1: 1.0},
            "sparse_u8_embedding": {1: 1},
        }
    }
