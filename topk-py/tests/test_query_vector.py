import numpy as np
import pytest

from topk_sdk.data import (
    binary_vector,
    f16_vector,
    f32_vector,
    f8_vector,
    u8_vector,
    i8_vector,
)
from topk_sdk.query import field, fn, select

from . import ProjectContext
from .utils import dataset, doc_ids, is_sorted, doc_fields


def test_query_vector_distance(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            "title",
            summary_distance=fn.vector_distance("summary_embedding", [2.0] * 16),
        ).topk(field("summary_distance"), 3, True)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_fields(result) == {"_id", "title", "summary_distance"}
    assert doc_ids(result) == {"1984", "pride", "mockingbird"}


def test_query_vector_distance_numpy_f32(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            "title",
            summary_distance=fn.vector_distance(
                "summary_embedding", np.array([2.0] * 16, dtype=np.float32)
            ),
        ).sort(field("summary_distance"), True).limit(3)
    )

    assert is_sorted(result, "summary_distance")
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


def test_query_vector_distance_numpy_u8(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance(
                "scalar_embedding", np.array([8] * 16, dtype=np.uint8)
            )
        ).sort(field("summary_distance"), True).limit(3)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_ids(result) == {"harry", "1984", "catcher"}


def test_query_vector_distance_i8_vector(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance(
                "scalar_i8_embedding", i8_vector([-10] * 16)
            )
        ).topk(field("summary_distance"), 3, True)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_ids(result) == {"pride", "1984", "gatsby"}


def test_query_vector_distance_numpy_i8(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance(
                "scalar_i8_embedding", np.array([-10] * 16, dtype=np.int8)
            )
        ).sort(field("summary_distance"), True).limit(3)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_ids(result) == {"pride", "1984", "gatsby"}


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


def test_query_vector_distance_numpy_binary(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance(
                "binary_embedding", np.array([0, 1], dtype=np.uint8)
            )
        ).sort(field("summary_distance"), True).limit(2)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_ids(result) == {"1984", "mockingbird"}


def test_query_vector_distance_f8_vector(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance("f8_embedding", f8_vector([1.0] * 16))
        ).topk(field("summary_distance"), 3, True)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_ids(result) == {"mockingbird", "1984", "pride"}


def test_query_vector_distance_f16_vector(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance("f16_embedding", f16_vector([1.0] * 16))
        ).topk(field("summary_distance"), 3, True)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_ids(result) == {"mockingbird", "1984", "pride"}


def test_query_vector_distance_numpy_f16(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance(
                "f16_embedding", np.array([1.0] * 16, dtype=np.float16)
            )
        ).sort(field("summary_distance"), True).limit(3)
    )

    assert is_sorted(result, "summary_distance")
    assert doc_ids(result) == {"mockingbird", "1984", "pride"}


def test_query_vector_distance_numpy_2d_invalid():
    with pytest.raises(
        ValueError, match="Vector query must be a vector or sparse vector"
    ):
        fn.vector_distance(
            "summary_embedding", np.array([[1.0, 2.0]], dtype=np.float32)
        )


def test_query_vector_distance_numpy_unsupported_dtype():
    with pytest.raises(TypeError, match="Unsupported numpy dtype"):
        fn.vector_distance("summary_embedding", np.array([1.0, 2.0], dtype=np.float64))
