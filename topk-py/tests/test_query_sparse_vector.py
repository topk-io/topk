from typing import Any

from topk_sdk import data
from topk_sdk.query import field, fn, select

from . import ProjectContext
from .utils import dataset, doc_ids_ordered


def is_sorted(result: list[dict[str, Any]], field_name: str) -> bool:
    values = [doc[field_name] for doc in result]
    return all(values[i] <= values[i + 1] for i in range(len(values) - 1))


def test_query_sparse_vector_distance_f32(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            "title",
            score=fn.vector_distance("sparse_f32_embedding", {0: 1.0, 1: 2.0, 2: 3.0}),
        ).topk(field("score"), 3, False)
    )

    assert doc_ids_ordered(result) == ["mockingbird", "1984", "alchemist"]


def test_query_sparse_vector_distance_u8_vector(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            score=fn.vector_distance(
                "sparse_u8_embedding", data.u8_sparse_vector({0: 1, 1: 2, 2: 3})
            )
        ).topk(field("score"), 3, False)
    )

    assert doc_ids_ordered(result) == ["mockingbird", "1984", "alchemist"]


def test_query_sparse_vector_distance_nullable(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            "title",
            sparse_u8_distance=fn.vector_distance(
                "sparse_u8_embedding", data.u8_sparse_vector({0: 1, 1: 2, 2: 3})
            ),
        ).topk(field("sparse_u8_distance"), 3, False)
    )

    assert doc_ids_ordered(result) == ["mockingbird", "1984", "alchemist"]

    # Get the mockingbird document and set its sparse_u8_embedding to null
    mockingbird = ctx.client.collection(collection.name).get(["mockingbird"])[
        "mockingbird"
    ]
    mockingbird["sparse_u8_embedding"] = None
    # binary embeddings need to use `data.binary_vector` constructor
    mockingbird["binary_embedding"] = data.binary_vector(
        mockingbird["binary_embedding"]
    )
    # u8 embeddings need to use `data.u8_vector` constructor
    mockingbird["scalar_embedding"] = data.u8_vector(mockingbird["scalar_embedding"])

    # i8 embeddings need to use `data.i8_vector` constructor
    mockingbird["scalar_i8_embedding"] = data.i8_vector(mockingbird["scalar_i8_embedding"])

    # native value (empty list string) needs to be converted to an empty string list instance
    mockingbird["tags"] = data.string_list(mockingbird["tags"])

    # Upsert the modified document
    lsn = ctx.client.collection(collection.name).upsert([mockingbird])

    # Query again with the LSN to ensure we get the updated data
    result = ctx.client.collection(collection.name).query(
        select(
            "title",
            sparse_u8_distance=fn.vector_distance(
                "sparse_u8_embedding", data.u8_sparse_vector({0: 1, 1: 2, 2: 3})
            ),
        ).topk(field("sparse_u8_distance"), 3, False),
        lsn=lsn,
    )

    assert doc_ids_ordered(result) == ["1984", "alchemist", "catcher"]
