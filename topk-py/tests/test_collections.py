import pytest
from topk_sdk import error

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
