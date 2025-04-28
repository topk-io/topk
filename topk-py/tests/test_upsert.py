import pytest
from topk_sdk import error
from topk_sdk.schema import text

from . import ProjectContext


def test_upsert_to_non_existent_collection(ctx: ProjectContext):
    with pytest.raises(error.CollectionNotFoundError):
        ctx.client.collection("missing").upsert([{"_id": "one"}])


def test_upsert_basic(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    lsn = ctx.client.collection(collection.name).upsert([{"_id": "one"}])
    assert lsn == "1"


def test_upsert_batch(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "one"}, {"_id": "two"}]
    )
    assert lsn == "1"


def test_upsert_sequential(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    lsn = ctx.client.collection(collection.name).upsert([{"_id": "one"}])
    assert lsn == "1"

    lsn = ctx.client.collection(collection.name).upsert([{"_id": "two"}])
    assert lsn == "2"

    lsn = ctx.client.collection(collection.name).upsert([{"_id": "three"}])
    assert lsn == "3"


def test_upsert_no_documents(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    with pytest.raises(error.DocumentValidationError) as exc_info:
        ctx.client.collection(collection.name).upsert([])
    assert "NoDocuments" in str(exc_info.value)


def test_upsert_invalid_document(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    with pytest.raises(error.DocumentValidationError) as exc_info:
        ctx.client.collection(collection.name).upsert([{}])
    assert "MissingId" in str(exc_info.value)


def test_upsert_schema_validation(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"), schema={"name": text().required()}
    )

    with pytest.raises(error.DocumentValidationError) as exc_info:
        ctx.client.collection(collection.name).upsert([{"_id": "one"}])
    assert "MissingField" in str(exc_info.value)
