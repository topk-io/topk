import pytest
import numpy as np
from typing import Literal, Optional, cast

from topk_sdk import data, error
from topk_sdk.query import field, fn, literal, select

from . import ProjectContext
from .utils import dataset, doc_ids_ordered

# Query vectors
Q1 = [
    -0.4449,
    1.3496,
    0.6855,
    -0.7714,
    -0.0942,
    -0.7982,
    -0.4429,
    -0.5834,
    -0.7113,
    1.009,
    1.1826,
    0.5344,
    0.0189,
    -0.2313,
]

Q2 = [
    1.5269,
    -0.2615,
    -0.1201,
    -1.495,
    0.5497,
    0.1703,
    -0.4399,
    1.8301,
    0.6419,
    -1.8175,
    1.8999,
    -0.3407,
    0.5301,
    -1.1665,
    -1.6396,
    2.2458,
    0.1597,
    0.8082,
    0.2963,
    0.1538,
    1.3943,
]


@pytest.mark.parametrize("value_type", ["f32", "f16", "f8"])
def test_query_multi_vector_float(ctx: ProjectContext, value_type: str):
    collection = dataset.multi_vec.setup(ctx, value_type)

    for q, expected_ids in [
        (Q1, ["doc_7", "doc_8", "doc_6"]),
        (Q2, ["doc_0", "doc_6", "doc_8"]),
    ]:
        # Convert flat list to matrix (2 rows x 7 cols for Q1, 3 rows x 7 cols for Q2)
        num_rows = len(q) // 7
        matrix_rows = [q[i * 7 : (i + 1) * 7] for i in range(num_rows)]
        query_matrix = dataset.multi_vec.cast(value_type, matrix_rows)

        result = ctx.client.collection(collection.name).query(
            select(
                title=field("title"),
                dist=fn.multi_vector_distance("token_embeddings", query_matrix),
            ).topk(field("dist"), 3, False)
        )

        assert len(result) == 3
        assert doc_ids_ordered(result) == expected_ids


def test_query_multi_vector_int(ctx: ProjectContext):
    for value_type, queries in [
        (
            "u8",
            [
                (Q1, ["doc_1", "doc_4", "doc_6"]),
                (Q2, ["doc_1", "doc_2", "doc_4"]),
            ],
        ),
        (
            "i8",
            [
                (Q1, ["doc_7", "doc_8", "doc_6"]),
                (Q2, ["doc_0", "doc_6", "doc_5"]),
            ],
        ),
    ]:
        print(f"value_type={value_type}")
        collection = dataset.multi_vec.setup(ctx, value_type)

        for q, expected_ids in queries:
            # Convert flat list to matrix
            num_rows = len(q) // 7
            matrix_rows = [q[i * 7 : (i + 1) * 7] for i in range(num_rows)]
            query_matrix = dataset.multi_vec.cast(value_type, matrix_rows)

            result = ctx.client.collection(collection.name).query(
                select(
                    title=field("title"),
                    dist=fn.multi_vector_distance("token_embeddings", query_matrix),
                ).topk(field("dist"), 3, False)
            )

            assert len(result) == 3
            assert doc_ids_ordered(result) == expected_ids


def test_query_multi_vector_with_filter(ctx: ProjectContext):
    collection = dataset.multi_vec.setup(ctx, "f32")

    for q, expected_ids in [
        (Q1, ["doc_7", "doc_6", "doc_1"]),
        (Q2, ["doc_0", "doc_6", "doc_5"]),
    ]:
        # Convert flat list to matrix
        num_rows = len(q) // 7
        matrix_rows = [q[i * 7 : (i + 1) * 7] for i in range(num_rows)]
        query_matrix = dataset.multi_vec.cast("f32", matrix_rows)

        result = ctx.client.collection(collection.name).query(
            select(
                title=field("title"),
                dist=fn.multi_vector_distance("token_embeddings", query_matrix),
            )
            .filter(field("_id") != literal("doc_8"))
            .topk(field("dist"), 3, False)
        )

        assert len(result) == 3
        assert doc_ids_ordered(result) == expected_ids


def test_query_multi_vector_with_invalid_dim(ctx: ProjectContext):
    collection = dataset.multi_vec.setup(ctx, "f32")

    with pytest.raises(error.InvalidArgumentError):
        # Use wrong dimension (2 instead of 7)
        num_rows = len(Q1) // 2  # This will create wrong number of columns
        matrix_rows = [Q1[i * 2 : (i + 1) * 2] for i in range(num_rows)]
        query_matrix = dataset.multi_vec.cast("f32", matrix_rows)

        ctx.client.collection(collection.name).query(
            select(
                title=field("title"),
                dist=fn.multi_vector_distance("token_embeddings", query_matrix),
            ).topk(field("dist"), 3, False)
        )


def test_query_multi_vector_with_invalid_data_type(ctx: ProjectContext):
    collection = dataset.multi_vec.setup(ctx, "f32")

    with pytest.raises(error.InvalidArgumentError):
        # Use wrong data type (f16 instead of f32)
        num_rows = len(Q1) // 7
        matrix_rows = [Q1[i * 7 : (i + 1) * 7] for i in range(num_rows)]
        query_matrix = dataset.multi_vec.cast("f16", matrix_rows)

        ctx.client.collection(collection.name).query(
            select(
                title=field("title"),
                dist=fn.multi_vector_distance("token_embeddings", query_matrix),
            ).topk(field("dist"), 3, False)
        )


def test_query_multi_vector_with_empty_query(ctx: ProjectContext):
    collection = dataset.multi_vec.setup(ctx, "f32")

    with pytest.raises(error.InvalidArgumentError):
        # Empty matrix
        query_matrix = dataset.multi_vec.cast("f32", [])

        ctx.client.collection(collection.name).query(
            select(
                title=field("title"),
                dist=fn.multi_vector_distance("token_embeddings", query_matrix),
            ).topk(field("dist"), 3, False)
        )


def test_query_multi_vector_with_missing_index(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        num_rows = len(Q1) // 7
        matrix_rows = [Q1[i * 7 : (i + 1) * 7] for i in range(num_rows)]
        query_matrix = dataset.multi_vec.cast("f32", matrix_rows)

        ctx.client.collection(collection.name).query(
            select(
                title=field("title"),
                dist=fn.multi_vector_distance("token_embeddings", query_matrix),
            ).topk(field("dist"), 3, False)
        )


def test_query_multi_vector_list_of_lists_f32(ctx: ProjectContext):
    collection = dataset.multi_vec.setup(ctx, "f32")
    num_rows = len(Q1) // 7
    matrix_rows = [Q1[i * 7 : (i + 1) * 7] for i in range(num_rows)]
    # Pass raw list of lists, not wrapped in data.matrix()

    result = ctx.client.collection(collection.name).query(
        select(
            title=field("title"),
            dist=fn.multi_vector_distance("token_embeddings", matrix_rows),  # type: ignore
        ).topk(field("dist"), 3, False)
    )

    assert len(result) == 3
    assert doc_ids_ordered(result) == ["doc_7", "doc_8", "doc_6"]


@pytest.mark.parametrize("value_type", ["f16", "f8", "u8", "i8"])
def test_query_multi_vector_list_of_lists_type_mismatch(
    ctx: ProjectContext,
    value_type: Literal["f16", "f8", "u8", "i8"],
) -> None:
    collection = dataset.multi_vec.setup(ctx, value_type)
    num_rows = len(Q1) // 7
    matrix_rows = [Q1[i * 7 : (i + 1) * 7] for i in range(num_rows)]

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
            select(
                title=field("title"),
                dist=fn.multi_vector_distance("token_embeddings", matrix_rows),
            ).topk(field("dist"), 3, False)
        )


@pytest.mark.parametrize(
    "dtype,value_type,expected_ids",
    [
        (np.float32, "f32", ["doc_7", "doc_8", "doc_6"]),
        (np.float16, "f16", ["doc_7", "doc_8", "doc_6"]),
        (np.uint8, "u8", ["doc_1", "doc_4", "doc_6"]),
        (np.int8, "i8", ["doc_7", "doc_8", "doc_6"]),
    ],
)
def test_query_multi_vector_numpy_array(ctx: ProjectContext, dtype: np.dtype, value_type: str, expected_ids: list[str]):
    """Test that numpy arrays work as query matrices"""
    collection = dataset.multi_vec.setup(ctx, value_type)
    num_rows = len(Q1) // 7
    matrix_rows = [Q1[i * 7 : (i + 1) * 7] for i in range(num_rows)]

    # Convert to numpy array with matching dtype
    # For u8/i8, apply the same conversion logic as dataset.multi_vec.cast()
    if value_type == "u8":
        # Convert f32 to u8: (abs(v) * 64.0).round() as u8
        converted_rows = [
            [int(round(abs(v) * 64.0)) for v in row] for row in matrix_rows
        ]
        arr = np.array(converted_rows, dtype=dtype)
    elif value_type == "i8":
        # Convert f32 to i8: (v * 64.0).round() and clip to i8 range
        def clip_to_i8(n: int) -> int:
            return max(min(n, 127), -128)

        converted_rows = [
            [clip_to_i8(int(round(v * 64.0))) for v in row] for row in matrix_rows
        ]
        arr = np.array(converted_rows, dtype=dtype)
    else:
        arr = np.array(matrix_rows, dtype=dtype)

    result = ctx.client.collection(collection.name).query(
        select(
            title=field("title"),
            dist=fn.multi_vector_distance("token_embeddings", arr),
        ).topk(field("dist"), 3, False)
    )

    assert len(result) == 3
    assert doc_ids_ordered(result) == expected_ids
