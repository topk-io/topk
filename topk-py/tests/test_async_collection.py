import asyncio
import pytest
from topk_sdk import error
from topk_sdk.query import select, field, literal

from . import AsyncProjectContext
from .utils import dataset, doc_ids


@pytest.mark.asyncio
async def test_async_upsert(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(
        async_ctx.scope("test"), schema={}
    )
    async_collection = async_ctx.client.collection(collection.name)

    lsn = await async_collection.upsert([{"_id": "one"}])
    assert lsn == "1"


@pytest.mark.asyncio
async def test_async_get(async_ctx: AsyncProjectContext):
    collection = await dataset.books.setup_async(async_ctx)
    async_collection = async_ctx.client.collection(collection.name)

    docs = await async_collection.get(["lotr"])

    assert docs == {
        "lotr": {
            "_id": "lotr",
            "title": "The Lord of the Rings: The Fellowship of the Ring",
            "published_year": 1954,
            "summary": "A group of unlikely heroes sets out to destroy a powerful, evil ring.",
            "summary_embedding": [9.0] * 16,
            "scalar_i8_embedding": [-100] * 16,
            "sparse_f32_embedding": {9: 1.0, 10: 2.0, 11: 3.0},
            "sparse_u8_embedding": {9: 1, 10: 2, 11: 3},
            "f8_embedding": [9.0] * 16,
            "f16_embedding": [9.0] * 16,
            "tags": ["lord of the rings", "fellowship", "magic", "wizard", "elves"],
            "codes": [
                "ISBN 978-0-547-92821-0",
                "ISBN 0-547-92821-2",
                "OCLC 434394005",
                "LCCN 2004558654",
                "Barcode 0618346252",
            ],
            "user_ratings": ["epic", "legendary", "good"],
        }
    }


@pytest.mark.asyncio
async def test_async_delete(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(
        async_ctx.scope("test"), schema={}
    )
    async_collection = async_ctx.client.collection(collection.name)

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
async def test_async_delete_filter(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(
        async_ctx.scope("test"), schema={}
    )
    async_collection = async_ctx.client.collection(collection.name)

    lsn = await async_collection.upsert(
        [
            {"_id": "one", "rank": 1},
            {"_id": "two", "rank": 2},
            {"_id": "three", "rank": 3},
        ]
    )
    assert lsn == "1"

    # wait for write to be flushed
    await async_collection.count()

    lsn = await async_collection.delete(field("rank") != 2)
    assert lsn == "2"

    docs = await async_collection.query(
        select("title").topk(field("rank"), 100, True), lsn=lsn
    )

    assert doc_ids(docs) == {"two"}

@pytest.mark.asyncio
async def test_async_delete_filter_complex(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(
        async_ctx.scope("test"), schema={}
    )
    async_collection = async_ctx.client.collection(collection.name)

    lsn = await async_collection.upsert(
        [
            {"_id": "one", "rank": 1},
            {"_id": "two", "rank": 2},
            {"_id": "three", "rank": 3},
        ]
    )
    assert lsn == "1"

    # wait for write to be flushed
    await async_collection.count()

    lsn = await async_collection.delete((field("rank") != 2) & (field("rank") != 999) & (field("rank") != -20))
    assert lsn == "2"

    docs = await async_collection.query(
        select("title").topk(field("rank"), 100, True), lsn=lsn
    )

    assert doc_ids(docs) == {"two"}

@pytest.mark.asyncio
async def test_async_query(async_ctx: AsyncProjectContext):
    collection = await dataset.books.setup_async(async_ctx)

    results = await async_ctx.client.collection(collection.name).query(
        select("title", "published_year", literal=literal(1.0))
        .filter(field("title") == "1984")
        .topk(field("published_year"), 100, True)
    )

    assert results == [
        {"_id": "1984", "title": "1984", "published_year": 1949, "literal": 1.0}
    ]

@pytest.mark.asyncio
async def test_async_query_complex(async_ctx: AsyncProjectContext):
    collection = await dataset.books.setup_async(async_ctx)

    results = await async_ctx.client.collection(collection.name).query(
        select("title", "published_year", literal=literal(1.0))
        .filter(field("title") == "1984")
        .topk(field("published_year") * field("published_year") * field("published_year"), 100, True)
    )

    assert results == [
        {"_id": "1984", "title": "1984", "published_year": 1949, "literal": 1.0}
    ]

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
async def test_async_collection_parallel_queries(async_ctx: AsyncProjectContext):
    collection = await dataset.books.setup_async(async_ctx)
    async_collection = async_ctx.client.collection(collection.name)

    # Upsert test documents
    lsn = await async_collection.upsert(test_documents)

    # Define multiple queries
    query1 = select("title", "published_year").topk(field("published_year"), 5)
    query2 = select("title", "summary").topk(field("published_year"), 3)
    query3 = select("published_year").topk(field("published_year"), 10)

    # Execute queries in parallel
    results = await asyncio.gather(
        async_collection.query(query1, lsn=lsn),
        async_collection.query(query2, lsn=lsn),
        async_collection.query(query3, lsn=lsn),
    )

    # Verify all queries returned results
    assert len(results) == 3
    assert all(len(result) >= 1 for result in results)

    # Verify query-specific fields are present
    assert "title" in results[0][0]
    assert "published_year" in results[0][0]

    assert "title" in results[1][0]
    assert "summary" in results[1][0]

    assert "published_year" in results[2][0]


def test_async_collection_sync_usage(async_ctx: AsyncProjectContext):
    async def async_operations():
        collection = await dataset.books.setup_async(async_ctx)
        async_collection = async_ctx.client.collection(collection.name)

        # Perform async operations
        lsn = await async_collection.upsert(test_documents)

        docs = await async_collection.get(["test_doc"])
        assert "test_doc" in docs

        return lsn

    # Run async operations in sync context
    lsn = asyncio.run(async_operations())
    assert isinstance(lsn, str)


@pytest.mark.asyncio
async def test_async_update_batch(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})
    async_collection = async_ctx.client.collection(collection.name)

    lsn = await async_collection.upsert(
        [
            {"_id": "1", "foo": "bar1"},
            {"_id": "2", "foo": "bar2"},
            {"_id": "3", "foo": "bar3"},
            {"_id": "4", "foo": "bar4"},
        ]
    )
    assert lsn == "1"

    lsn = await async_collection.update(
        [
            {"_id": "2", "foo": "bar2.2", "baz": "foo"},
            {"_id": "3", "foo": None},
            {"_id": "4", "foo": "bar4.2"},
            {"_id": "5", "foo": "bar5"},  # missing id
        ],
        False,
    )
    assert lsn == "2"

    docs = await async_collection.get(["1", "2", "3", "4", "5"], lsn=lsn)

    assert len(docs) == 4
    assert docs["1"] == {"_id": "1", "foo": "bar1"}
    assert docs["2"] == {"_id": "2", "foo": "bar2.2", "baz": "foo"}
    assert docs["3"] == {"_id": "3"}
    assert docs["4"] == {"_id": "4", "foo": "bar4.2"}


@pytest.mark.asyncio
async def test_async_update_missing_id_with_fail_on_missing(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(async_ctx.scope("test"), schema={})
    async_collection = async_ctx.client.collection(collection.name)

    # Upsert some docs
    lsn = await async_collection.upsert(
        [
            {"_id": "1", "foo": "bar1"},
            {"_id": "2", "foo": "bar2"},
        ]
    )
    assert lsn == "1"

    # Update non-existent doc
    with pytest.raises(error.DocumentValidationError) as exc_info:
        await async_collection.update(
            [{"_id": "3", "foo": "bar3"}], True
        )
    assert "DocumentNotFound" in str(exc_info.value)
    assert 'doc_id: "3"' in str(exc_info.value)
