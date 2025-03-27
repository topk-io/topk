import pytest
from topk_sdk import Collection, error
from topk_sdk.schema import (
    binary_vector,
    bool,
    bytes,
    f32_vector,
    float,
    int,
    keyword_index,
    semantic_index,
    text,
    u8_vector,
    vector_index,
)

from . import ProjectContext


def test_create_collection(ctx: ProjectContext):
    schema = {
        "title": text().required().index(keyword_index()),
        "title_embedding": f32_vector(1536)
        .required()
        .index(vector_index(metric="euclidean")),
        "summary": text().required().index(semantic_index()),
        "published_year": int().required(),
    }

    collection = ctx.client.collections().create(
        ctx.scope("books"),
        schema=schema,
    )

    assert collection.name == ctx.scope("books")
    assert collection.schema == schema


def test_create_collection_all_data_types(ctx: ProjectContext):
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

    collection = ctx.client.collections().create(
        ctx.scope("books"),
        schema=schema,
    )

    assert collection.name == ctx.scope("books")
    assert collection.schema == schema


def test_incorrect_schema(ctx: ProjectContext):
    with pytest.raises(error.SchemaValidationError):
        ctx.client.collections().create(
            ctx.scope("books"),
            schema={"name": text().index(vector_index(metric="cosine"))},
        )


def test_list_collections(ctx: ProjectContext):
    # Note: All python tests run within the same project,
    # so list of collections is shared across tests.

    a = ctx.client.collections().create(ctx.scope("books"), schema={})
    assert a in ctx.client.collections().list()

    b = ctx.client.collections().create(ctx.scope("books2"), schema={})
    assert a in ctx.client.collections().list()
    assert b in ctx.client.collections().list()

    c = ctx.client.collections().create(ctx.scope("books3"), schema={})
    assert a in ctx.client.collections().list()
    assert b in ctx.client.collections().list()
    assert c in ctx.client.collections().list()


def test_get_collections(ctx: ProjectContext):
    # get non-existent collection
    with pytest.raises(error.CollectionNotFoundError):
        ctx.client.collections().get(ctx.scope("foo"))

    # create collection
    ctx.client.collections().create(ctx.scope("foo"), schema={})

    # get collection
    collection = ctx.client.collections().get(ctx.scope("foo"))

    # assert collection
    assert collection.name == ctx.scope("foo")


def test_delete_collection(ctx: ProjectContext):
    assert ctx.scope("books") not in [c.name for c in ctx.client.collections().list()]

    ctx.client.collections().create(ctx.scope("books"), schema={})

    assert ctx.scope("books") in [c.name for c in ctx.client.collections().list()]

    ctx.client.collections().delete(ctx.scope("books"))

    assert ctx.scope("books") not in [c.name for c in ctx.client.collections().list()]


def test_delete_non_existent_collection(ctx: ProjectContext):
    try:
        ctx.client.collections().delete(ctx.scope("books"))
    except error.CollectionNotFoundError:
        pass
    else:
        assert False, "CollectionNotFoundError not raised"


###


@pytest.fixture
def ctx():
    from . import new_project_context

    return new_project_context()
