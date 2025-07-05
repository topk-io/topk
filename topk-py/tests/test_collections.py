import pytest
from topk_sdk import error
from topk_sdk.schema import (
    binary_vector,
    bool,
    bytes,
    f32_vector,
    float,
    int,
    semantic_index,
    text,
    u8_vector,
    vector_index,
)

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


def test_collection_schema_validation(ctx: ProjectContext):
    schema = {
        "text": text(),
        "int": int(),
        "float": float(),
        "bool": bool(),
        "vector": f32_vector(1536),
        "float_vector": f32_vector(1536),
        "byte_vector": u8_vector(1536),
        "binary_vector": binary_vector(1536),
        "bytes": bytes(),
    }

    collection = ctx.client.collections().create(ctx.scope("books"), schema)

    assert collection.name == ctx.scope("books")

    # Test that all fields are present in the schema
    for field_name in schema.keys():
        assert field_name in collection.schema


def test_create_collection_with_invalid_schema(ctx: ProjectContext):
    with pytest.raises(TypeError):
        ctx.client.collections().create(
            ctx.scope("books"),
            {
                "title": "invalid",
            },
        )


def test_incorrect_schema(ctx: ProjectContext):
    with pytest.raises(error.SchemaValidationError):
        ctx.client.collections().create(
            ctx.scope("books"),
            {
                "name": text().index(vector_index(metric="cosine")),
            },
        )
