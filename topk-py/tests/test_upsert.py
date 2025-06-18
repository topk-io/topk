import pytest
from topk_sdk import error
from topk_sdk import data
from topk_sdk.schema import (
    bool,
    text,
    int,
    float,
    f32_vector,
    u8_vector,
    binary_vector,
    bytes,
)

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


@pytest.mark.parametrize(
    "params",
    [
        (True, bool()),
        ("hello", text()),
        (1, int()),
        (1.0, float()),
        (b"hello", bytes()),
    ],
)
def test_upsert_primitives(ctx: ProjectContext, params):
    value, data_type = params  # type: ignore

    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"field": data_type},  # type: ignore
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "x", "field": value},
        ]
    )

    obj = ctx.client.collection(collection.name).get(["x"], lsn=lsn)

    assert obj["x"]["field"] == value


def test_upsert_vectors(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "f32_vector": f32_vector(3),
            "u8_vector": u8_vector(3),
            "binary_vector": binary_vector(3),
        },
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {
                "_id": "x",
                "f32_vector": [1, 2, 3],
                "u8_vector": data.u8_vector([4, 5, 6]),
                "binary_vector": data.binary_vector([7, 8, 9]),
            }
        ]
    )

    obj = ctx.client.collection(collection.name).get(["x"], lsn=lsn)

    assert obj["x"]["f32_vector"] == [1, 2, 3]
    assert obj["x"]["u8_vector"] == [4, 5, 6]
    assert obj["x"]["binary_vector"] == [7, 8, 9]
