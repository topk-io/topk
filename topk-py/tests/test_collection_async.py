import asyncio
import pytest
from topk_sdk import data, error, query, schema
from topk_sdk.query import select, field, literal

from . import ProjectContext
from .utils import dataset, doc_ids

test_documents = [
    {
        "_id": "test_doc",
        "title": "Test Document",
        "published_year": 2023,
        "summary": "A test document for async collection operations.",
        "summary_embedding": [0.5] * 16,
    }
]


@pytest.mark.asyncio
async def test_async_upsert_to_non_existent_collection(ctx: ProjectContext):
    with pytest.raises(error.CollectionNotFoundError):
        await ctx.client.async_collection("missing").upsert([{"_id": "one"}])


@pytest.mark.asyncio
async def test_async_upsert_basic(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert([{"_id": "one"}])
    assert lsn == "1"


@pytest.mark.asyncio
async def test_async_upsert_batch(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert(
        [{"_id": "one"}, {"_id": "two"}]
    )
    assert lsn == "1"


@pytest.mark.asyncio
async def test_async_upsert_sequential(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert([{"_id": "one"}])
    assert lsn == "1"

    lsn = await async_collection.upsert([{"_id": "two"}])
    assert lsn == "2"

    lsn = await async_collection.upsert([{"_id": "three"}])
    assert lsn == "3"


@pytest.mark.asyncio
async def test_async_upsert_no_documents(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})
    async_collection = ctx.client.async_collection(collection.name)

    with pytest.raises(error.DocumentValidationError) as exc_info:
        await async_collection.upsert([])
    assert "NoDocuments" in str(exc_info.value)


@pytest.mark.asyncio
async def test_async_upsert_invalid_document(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})
    async_collection = ctx.client.async_collection(collection.name)

    with pytest.raises(error.DocumentValidationError) as exc_info:
        await async_collection.upsert([{}])
    assert "MissingId" in str(exc_info.value)


@pytest.mark.asyncio
async def test_async_upsert_schema_validation(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"), schema={"name": schema.text().required()}
    )
    async_collection = ctx.client.async_collection(collection.name)

    with pytest.raises(error.DocumentValidationError) as exc_info:
        await async_collection.upsert([{"_id": "one"}])
    assert "MissingField" in str(exc_info.value)


@pytest.mark.asyncio
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
async def test_async_upsert_primitives(ctx: ProjectContext, params):
    value, data_type = params  # type: ignore

    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"field": data_type},  # type: ignore
    )
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert(
        [
            {"_id": "x", "field": value},
        ]
    )

    obj = await async_collection.get(["x"], lsn=lsn)

    assert obj["x"]["field"] == value


@pytest.mark.asyncio
async def test_async_upsert_with_bytes_helper(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test-bytes"),
        schema={
            "title": schema.text().required(),
            "thumbnail": schema.bytes(),
        },
    )
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert(
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

    await async_collection.count(lsn=lsn)

    results = await async_collection.query(
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


@pytest.mark.asyncio
async def test_async_upsert_vectors(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "f32_vector": schema.f32_vector(3),
            "u8_vector": schema.u8_vector(3),
            "binary_vector": schema.binary_vector(3),
        },
    )
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert(
        [
            {
                "_id": "x",
                "f32_vector": [1, 2, 3],
                "u8_vector": data.u8_vector([4, 5, 6]),
                "binary_vector": data.binary_vector([7, 8, 9]),
            }
        ]
    )

    obj = await async_collection.get(["x"], lsn=lsn)

    assert obj["x"]["f32_vector"] == [1, 2, 3]
    assert obj["x"]["u8_vector"] == [4, 5, 6]
    assert obj["x"]["binary_vector"] == [7, 8, 9]


@pytest.mark.asyncio
async def test_async_upsert_sparse_vectors(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "f32_sparse_vector": schema.f32_sparse_vector(),
            "u8_sparse_vector": schema.u8_sparse_vector(),
        },
    )
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert(
        [
            {
                "_id": "x",
                "f32_sparse_vector": {1: 1.2, 2: 2.3, 3: 3.4},
                "u8_sparse_vector": data.u8_sparse_vector({1: 4, 2: 5, 3: 6}),
            }
        ]
    )

    obj = await async_collection.get(["x"], lsn=lsn)

    assert set(obj["x"]["f32_sparse_vector"].keys()) == {1, 2, 3}
    assert "{:.2f}".format(obj["x"]["f32_sparse_vector"][1]) == "1.20"
    assert "{:.2f}".format(obj["x"]["f32_sparse_vector"][2]) == "2.30"
    assert "{:.2f}".format(obj["x"]["f32_sparse_vector"][3]) == "3.40"
    assert set(obj["x"]["u8_sparse_vector"].keys()) == {1, 2, 3}
    assert obj["x"]["u8_sparse_vector"][1] == 4
    assert obj["x"]["u8_sparse_vector"][2] == 5
    assert obj["x"]["u8_sparse_vector"][3] == 6


@pytest.mark.asyncio
async def test_async_upsert_empty_float_list(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"f32_list": schema.list(value_type="float")},
    )
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert(
        [{"_id": "x", "f32_list": []}]
    )

    obj = await async_collection.get(["x"], lsn=lsn)

    assert obj["x"]["f32_list"] == []


@pytest.mark.asyncio
async def test_async_upsert_empty_float_list_with_helper(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"f32_list": schema.list(value_type="float")},
    )
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert(
        [{"_id": "x", "f32_list": data.f32_list([])}]
    )

    obj = await async_collection.get(["x"], lsn=lsn)

    assert obj["x"]["f32_list"] == []


@pytest.mark.asyncio
async def test_async_upsert_empty_integer_list_raises_error(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"i32_list": schema.list(value_type="integer")},
    )
    async_collection = ctx.client.async_collection(collection.name)

    with pytest.raises(error.DocumentValidationError) as exc_info:
        await async_collection.upsert(
            [{"_id": "x", "i32_list": []}]
        )
    assert "field: \"i32_list\", expected_type: \"list<integer>\", got_value: \"list<f32>\"" in str(exc_info.value)


@pytest.mark.asyncio
async def test_async_upsert_empty_integer_list_with_helper(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"i32_list": schema.list(value_type="integer")},
    )
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert(
        [{"_id": "x", "i32_list": data.i32_list([])}]
    )

    obj = await async_collection.get(["x"], lsn=lsn)

    assert obj["x"]["i32_list"] == []


@pytest.mark.asyncio
async def test_async_upsert_empty_string_list_raises_error(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"string_list": schema.list(value_type="text")},
    )
    async_collection = ctx.client.async_collection(collection.name)

    with pytest.raises(error.DocumentValidationError) as exc_info:
        await async_collection.upsert(
        [{"_id": "x", "string_list": []}]
    )

    assert "field: \"string_list\", expected_type: \"list<string>\", got_value: \"list<f32>\"" in str(exc_info.value)


@pytest.mark.asyncio
async def test_async_upsert_empty_string_list_with_helper(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"string_list": schema.list(value_type="text")},
    )
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert(
        [{"_id": "x", "string_list": data.string_list([])}]
    )

    obj = await async_collection.get(["x"], lsn=lsn)

    assert obj["x"]["string_list"] == []


@pytest.mark.asyncio
async def test_async_get_from_non_existent_collection(ctx: ProjectContext):
    with pytest.raises(error.CollectionNotFoundError):
        await ctx.client.async_collection("missing").get(["doc1"])


@pytest.mark.asyncio
async def test_async_get_non_existent_document(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    async_collection = ctx.client.async_collection(collection.name)

    docs = await async_collection.get(["missing"])
    assert docs == {}


@pytest.mark.asyncio
async def test_async_get_document(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    async_collection = ctx.client.async_collection(collection.name)

    docs = await async_collection.get(["lotr"])

    assert docs == {
        "lotr": {
            "_id": "lotr",
            "title": "The Lord of the Rings: The Fellowship of the Ring",
            "published_year": 1954,
            "summary": "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
            "summary_embedding": [9.0] * 16,
            "sparse_f32_embedding": {9: 1.0, 10: 2.0, 11: 3.0},
            "sparse_u8_embedding": {9: 1, 10: 2, 11: 3},
            "tags": ["lord of the rings", "fellowship", "magic", "wizard", "elves"],
            "codes": ["ISBN 978-0-547-92821-0", "ISBN 0-547-92821-2", "OCLC 434394005", "LCCN 2004558654", "Barcode 0618346252"],
        }
    }


@pytest.mark.asyncio
async def test_async_get_multiple_documents(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    async_collection = ctx.client.async_collection(collection.name)

    docs = await async_collection.get(["lotr", "moby"])

    assert docs == {
        "lotr": {
            "_id": "lotr",
            "title": "The Lord of the Rings: The Fellowship of the Ring",
            "published_year": 1954,
            "summary": "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
            "summary_embedding": [9.0] * 16,
            "sparse_f32_embedding": {9: 1.0, 10: 2.0, 11: 3.0},
            "sparse_u8_embedding": {9: 1, 10: 2, 11: 3},
            "tags": ["lord of the rings", "fellowship", "magic", "wizard", "elves"],
            "codes": ["ISBN 978-0-547-92821-0", "ISBN 0-547-92821-2", "OCLC 434394005", "LCCN 2004558654", "Barcode 0618346252"],

        },
        "moby": {
            "_id": "moby",
            "title": "Moby-Dick",
            "published_year": 1851,
            "summary": "A sailor's obsessive quest to hunt a great white whale leads to tragic consequences.",
            "summary_embedding": [6.0] * 16,
            "sparse_f32_embedding": {6: 1.0, 7: 2.0, 8: 3.0},
            "sparse_u8_embedding": {6: 1, 7: 2, 8: 3},
            "nullable_importance": 5.0,
            "tags": ["whale", "obsession", "tragedy", "sailing", "ocean"],
            "codes": [],
            "reprint_years": [],
        },
    }


@pytest.mark.asyncio
async def test_async_get_document_fields(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    async_collection = ctx.client.async_collection(collection.name)

    docs = await async_collection.get(
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


@pytest.mark.asyncio
async def test_async_delete_from_non_existent_collection(ctx: ProjectContext):
    async_collection = ctx.client.async_collection("missing")
    with pytest.raises(error.CollectionNotFoundError):
        await async_collection.delete(["one"])


@pytest.mark.asyncio
async def test_async_delete_document(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})
    async_collection = ctx.client.async_collection(collection.name)

    lsn = await async_collection.upsert(
        [
            {"_id": "one", "rank": 1},
            {"_id": "two", "rank": 2},
        ]
    )
    assert lsn == "1"

    # wait for write to be flushed
    await async_collection.count()

    lsn = await async_collection.delete(["one"])
    assert lsn == "2"

    docs = await async_collection.query(
        select("title").topk(field("rank"), 100, True), lsn=lsn
    )

    assert doc_ids(docs) == {"two"}


@pytest.mark.asyncio
async def test_async_delete_non_existent_document(ctx: ProjectContext):
    collection = ctx.client.collections().create(ctx.scope("test"), schema={})
    async_collection = ctx.client.async_collection(collection.name)

    # we can delete a non-existent document, and it will be ignored
    lsn = await async_collection.delete(["one"])
    assert lsn == "1"


@pytest.mark.asyncio
async def test_async_query_select_literal(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = await ctx.client.async_collection(collection.name).query(
        select("title", "published_year", literal=literal(1.0))
        .filter(field("title") == "1984")
        .topk(field("published_year"), 100, True)
    )

    assert results == [{"_id": "1984", "title": "1984", "published_year": 1949, "literal": 1.0}]


@pytest.mark.asyncio
async def test_async_collection_all_operations(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    async_collection = ctx.client.async_collection(collection.name)

    # Test async get
    docs = await async_collection.get(["mockingbird", "1984"])
    assert len(docs) == 2
    assert "mockingbird" in docs
    assert "1984" in docs
    assert docs["mockingbird"]["title"] == "To Kill a Mockingbird"
    assert docs["1984"]["title"] == "1984"

    # Test async count
    count = await async_collection.count()
    assert count == len(dataset.books.docs())

    # Test async query
    query = select("title", "summary", "published_year").topk(field("published_year"), 10)

    results = await async_collection.query(query)
    assert len(results) >= 2

    # Test async delete
    delete_lsn = await async_collection.delete(["mockingbird", "1984"])
    assert isinstance(delete_lsn, str)
    assert len(delete_lsn) > 0


@pytest.mark.asyncio
async def test_async_collection_all_operations_with_consistency(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    async_collection = ctx.client.async_collection(collection.name)

    # Test with LSN and consistency level
    lsn = await async_collection.upsert(test_documents)

    # Get with LSN
    docs = await async_collection.get(["test_doc"], lsn=lsn)
    assert "test_doc" in docs

    # Count with LSN
    count = await async_collection.count(lsn=lsn)
    assert count >= 1

    # Query with LSN
    query = select("title", "published_year").topk(field("published_year"), 5)
    results = await async_collection.query(query, lsn=lsn)
    assert len(results) >= 1


def test_async_collection_sync_usage(ctx: ProjectContext):
    async def async_operations():
        collection = dataset.books.setup(ctx)
        async_collection = ctx.client.async_collection(collection.name)

        # Perform async operations
        lsn = await async_collection.upsert(test_documents)

        docs = await async_collection.get(["test_doc"])
        assert "test_doc" in docs

        return lsn

    # Run async operations in sync context
    lsn = asyncio.run(async_operations())
    assert isinstance(lsn, str)
