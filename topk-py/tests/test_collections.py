import pytest
from topk_sdk import error
from topk_sdk.schema import matrix, multi_vector_index, text, vector_index

from . import ProjectContext


def test_list_collections(ctx: ProjectContext):
    c = ctx.client.collections().create(ctx.scope("test"), schema={})
    response = ctx.client.collections().list()
    assert c in response


def test_create_collection(ctx: ProjectContext):
    c = ctx.client.collections().create(ctx.scope("test"), schema={})
    collections = ctx.client.collections().list()
    assert c in collections


def test_create_duplicate_collection(ctx: ProjectContext):
    ctx.client.collections().create(ctx.scope("test"), schema={})

    with pytest.raises(error.CollectionAlreadyExistsError):
        ctx.client.collections().create(ctx.scope("test"), schema={})


def test_delete_non_existent_collection(ctx: ProjectContext):
    with pytest.raises(error.CollectionNotFoundError):
        ctx.client.collections().delete(ctx.scope("test"))


def test_delete_collection(ctx: ProjectContext):
    c = ctx.client.collections().create(ctx.scope("test"), schema={})
    ctx.client.collections().delete(ctx.scope("test"))

    collections = ctx.client.collections().list()
    assert c not in collections


def test_get_collection(ctx: ProjectContext):
    # Test getting non-existent collection
    with pytest.raises(error.CollectionNotFoundError):
        ctx.client.collections().get(ctx.scope("test"))

    # Create collection
    c = ctx.client.collections().create(ctx.scope("test"), schema={})

    # Get collection
    collection = ctx.client.collections().get(ctx.scope("test"))
    assert collection == c


def test_matrix_schema(ctx: ProjectContext):
    matrix_value_types: list[tuple[str, str]] = [
        ("f32", "F32"),
        ("u8", "U8"),
        ("i8", "I8"),
    ]

    for value_type, expected_value_type in matrix_value_types:
        schema = {
            "token_embeddings": matrix(7, value_type).index(multi_vector_index("maxsim")),  # type: ignore
        }
        collection = ctx.client.collections().create(
            ctx.scope(f"test_matrix_{value_type}"), schema=schema
        )
        field_spec_str = str(collection.schema["token_embeddings"])
        assert "data_type: Matrix" in field_spec_str
        assert "dimension: 7" in field_spec_str
        assert f"value_type: {expected_value_type}" in field_spec_str
        assert "required: false" in field_spec_str
        assert "index: Some(MultiVectorIndex" in field_spec_str
        assert "metric: Maxsim" in field_spec_str


@pytest.mark.parametrize(
    "value_type,expected_value_type",
    [
        ("f16", "F16"),
        ("f8", "F8"),
    ],
)
def test_matrix_schema_f16_and_f8(ctx: ProjectContext, value_type: str, expected_value_type: str):
    schema_without_index = {
        "token_embeddings": matrix(7, value_type),  # type: ignore
    }
    collection = ctx.client.collections().create(
        ctx.scope(f"test_matrix_{value_type}_no_index"), schema=schema_without_index
    )
    field_spec_str = str(collection.schema["token_embeddings"])
    assert (
        f'FieldSpec {{ data_type: Matrix {{ dimension: 7, value_type: {expected_value_type} }}, required: false, index: None }}'
        in field_spec_str
    )


def test_incorrect_schema(ctx: ProjectContext):
    # Test that text field cannot have vector index
    with pytest.raises(error.SchemaValidationError) as exc_info:
        ctx.client.collections().create(
            ctx.scope("books"),
            schema={"name": text().index(vector_index(metric="cosine"))},
        )
    assert (
        'InvalidIndex { field: "name", index: "vector", data_type: "text" }'
        in str(exc_info.value)
    )

    # Test that f16 matrix cannot have maxsim index
    with pytest.raises(error.SchemaValidationError) as exc_info:
        ctx.client.collections().create(
            ctx.scope("test_matrix_f16_invalid"),
            schema={
                "token_embeddings": matrix(7, "f16").index(multi_vector_index("maxsim"))
            },
        )
    assert (
        'InvalidIndex { field: "token_embeddings", index: "multi_vector", data_type: "matrix<f16>" }'
        in str(exc_info.value)
    )

    # Test that f8 matrix cannot have maxsim index
    with pytest.raises(error.SchemaValidationError) as exc_info:
        ctx.client.collections().create(
            ctx.scope("test_matrix_f8_invalid"),
            schema={
                "token_embeddings": matrix(7, "f8").index(multi_vector_index("maxsim"))
            },
        )
    assert (
        'InvalidIndex { field: "token_embeddings", index: "multi_vector", data_type: "matrix<f8>" }'
        in str(exc_info.value)
    )
