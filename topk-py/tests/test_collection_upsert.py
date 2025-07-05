import pytest
from topk_sdk import data, error, query, schema

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
        ctx.scope("test"), schema={"name": schema.text().required()}
    )

    with pytest.raises(error.DocumentValidationError) as exc_info:
        ctx.client.collection(collection.name).upsert([{"_id": "one"}])
    assert "MissingField" in str(exc_info.value)


@pytest.mark.parametrize(
    "params",
    [
        (True, schema.bool()),
        ("hello", schema.text()),
        (1, schema.int()),
        (1.0, schema.float()),
        (b"hello", schema.bytes()),
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


def test_upsert_with_bytes_helper(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test-bytes"),
        schema={
            "title": schema.text().required(),
            "thumbnail": schema.bytes(),
        },
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {
                "_id": "doc1",
                "rank": 1,
                "title": "Document with bytes from list",
                "thumbnail": data.bytes([0, 1, 255, 128]),
            },
            {
                "_id": "doc2",
                "rank": 2,
                "title": "Document with bytes from bytes object",
                "thumbnail": data.bytes(b"\x00\x01\xff\x80"),
            },
            {
                "_id": "doc3",
                "rank": 3,
                "title": "Document with empty bytes",
                "thumbnail": data.bytes([]),
            },
            {
                "_id": "doc4",
                "rank": 4,
                "title": "Document with native bytes",
                "thumbnail": bytes([10, 20, 30]),  # Test native Python bytes still work
            },
        ]
    )

    ctx.client.collection(collection.name).count(lsn=lsn)

    results = ctx.client.collection(collection.name).query(
        query.select("title", "thumbnail").topk(query.field("rank"), 10, True)
    )

    assert len(results) == 4

    for doc in results:
        if doc["_id"] == "doc1":
            assert doc["thumbnail"] == bytes([0, 1, 255, 128])
        elif doc["_id"] == "doc2":
            assert doc["thumbnail"] == bytes([0, 1, 255, 128])
        elif doc["_id"] == "doc3":
            assert doc["thumbnail"] == bytes([])
        elif doc["_id"] == "doc4":
            assert doc["thumbnail"] == bytes([10, 20, 30])


def test_upsert_vectors(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "f32_vector": schema.f32_vector(3),
            "u8_vector": schema.u8_vector(3),
            "binary_vector": schema.binary_vector(3),
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
            "f32_sparse_vector": schema.f32_sparse_vector(),
            "u8_sparse_vector": schema.u8_sparse_vector(),
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
