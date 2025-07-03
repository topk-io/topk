import pytest
from topk_sdk import data, error
from topk_sdk.schema import (
    binary_vector,
    bool,
    bytes,
    f32_sparse_vector,
    f32_vector,
    float,
    int,
    text,
    u8_sparse_vector,
    u8_vector,
    vector_index,
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

    with pytest.raises(error.DocumentValidationError, match="NoDocuments"):
        ctx.client.collection(collection.name).upsert([])


def test_upsert_invalid_document(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})

    with pytest.raises(error.DocumentValidationError, match="MissingId"):
        ctx.client.collection(collection.name).upsert([{}])


def test_upsert_schema_validation(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"), schema={"name": text().required()}
    )

    with pytest.raises(error.DocumentValidationError, match="MissingField"):
        ctx.client.collection(collection.name).upsert([{"_id": "one"}])


def test_upsert_with_invalid_document_null_field_value(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("books"),
        schema={
            "title": text().required(),
            "f32_embedding": f32_vector(3)
            .required()
            .index(vector_index(metric="euclidean")),
        },
    )

    with pytest.raises(
        error.DocumentValidationError, match="InvalidDataType.*title.*null"
    ):
        ctx.client.collection(collection.name).upsert(
            [{"_id": "doc1", "title": None, "f32_embedding": [1, 2, 3]}]
        )


def test_upsert_with_invalid_document_missing_required_field(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("books"),
        schema={
            "title": text().required(),
            "f32_embedding": f32_vector(3)
            .required()
            .index(vector_index(metric="euclidean")),
        },
    )

    with pytest.raises(error.DocumentValidationError, match="MissingField.*title"):
        ctx.client.collection(collection.name).upsert(
            [{"_id": "doc1", "f32_embedding": [1, 2, 3]}]
        )


def test_upsert_with_invalid_document_wrong_vector_dimension(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("books"),
        schema={
            "title": text().required(),
            "f32_embedding": f32_vector(3)
            .required()
            .index(vector_index(metric="euclidean")),
        },
    )

    with pytest.raises(
        error.DocumentValidationError,
        match="InvalidVectorDimension.*f32_embedding.*expected_dimension: 3.*got_dimension: 2",
    ):
        ctx.client.collection(collection.name).upsert(
            [{"_id": "doc1", "title": "one", "f32_embedding": [1, 2]}]
        )


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


def test_upsert_sparse_vectors(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "f32_sparse_vector": f32_sparse_vector(),
            "u8_sparse_vector": u8_sparse_vector(),
        },
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {
                "_id": "x",
                "f32_sparse_vector": {1: 1.2, 2: 2.3, 3: 3.4},
                "u8_sparse_vector": data.u8_sparse_vector({1: 4, 2: 5, 3: 6}),
            }
        ]
    )

    obj = ctx.client.collection(collection.name).get(["x"], lsn=lsn)

    assert set(obj["x"]["f32_sparse_vector"].keys()) == {1, 2, 3}
    assert "{:.2f}".format(obj["x"]["f32_sparse_vector"][1]) == "1.20"
    assert "{:.2f}".format(obj["x"]["f32_sparse_vector"][2]) == "2.30"
    assert "{:.2f}".format(obj["x"]["f32_sparse_vector"][3]) == "3.40"
    assert set(obj["x"]["u8_sparse_vector"].keys()) == {1, 2, 3}
    assert obj["x"]["u8_sparse_vector"][1] == 4
    assert obj["x"]["u8_sparse_vector"][2] == 5
    assert obj["x"]["u8_sparse_vector"][3] == 6
