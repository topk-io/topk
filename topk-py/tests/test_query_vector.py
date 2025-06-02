from typing import Any

from topk_sdk.data import binary_vector, f32_vector, u8_vector
from topk_sdk.query import field, fn, select

from . import ProjectContext
from .utils import dataset, doc_ids


def is_sorted(result: list[dict[str, Any]], field_name: str) -> bool:
    values = [doc[field_name] for doc in result]
    return all(values[i] <= values[i + 1] for i in range(len(values) - 1))


def test_query_vector_distance(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            "title",
            summary_distance=fn.vector_distance("summary_embedding", [2.0] * 16),
        ).topk(field("summary_distance"), 3, True)
    )

    assert is_sorted(result, "summary_distance")
    assert all(field in result[0] for field in ["_id", "title", "summary_distance"])
    assert doc_ids(result) == {"1984", "pride", "mockingbird"}


def test_query_vector_distance_nullable(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance(
                "nullable_embedding", f32_vector([3.0] * 16)
            )
        ).topk(field("summary_distance"), 3, True)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_ids(result) == {"1984", "mockingbird", "catcher"}


def test_query_vector_distance_u8_vector(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance("scalar_embedding", u8_vector([8] * 16))
        ).topk(field("summary_distance"), 3, True)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_ids(result) == {"harry", "1984", "catcher"}


def test_query_vector_distance_binary_vector(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance(
                "binary_embedding", binary_vector([0, 1])
            )
        ).topk(field("summary_distance"), 2, True)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_ids(result) == {"1984", "mockingbird"}
