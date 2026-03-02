import pytest
from topk_sdk import error

from . import ProjectContext


def test_list_datasets(ctx: ProjectContext):
    create_resp = ctx.client.datasets().create(ctx.scope("test"))
    print(create_resp)
    d = create_resp.dataset
    response = ctx.client.datasets().list()
    print(response)
    assert d in response.datasets


def test_create_dataset(ctx: ProjectContext):
    create_resp = ctx.client.datasets().create(ctx.scope("test"))
    print(create_resp)
    d = create_resp.dataset
    response = ctx.client.datasets().list()
    print(response)
    assert d in response.datasets


def test_create_duplicate_dataset(ctx: ProjectContext):
    response = ctx.client.datasets().create(ctx.scope("test"))
    print(response)

    with pytest.raises(error.DatasetAlreadyExistsError):
        ctx.client.datasets().create(ctx.scope("test"))


def test_delete_non_existent_dataset(ctx: ProjectContext):
    with pytest.raises(error.DatasetNotFoundError):
        ctx.client.datasets().delete(ctx.scope("test"))


def test_delete_dataset(ctx: ProjectContext):
    d = ctx.client.datasets().create(ctx.scope("test")).dataset
    response = ctx.client.datasets().delete(ctx.scope("test"))
    print(response)

    response = ctx.client.datasets().list()
    print(response)
    assert d not in response.datasets


def test_get_dataset(ctx: ProjectContext):
    # Test getting non-existent dataset
    with pytest.raises(error.DatasetNotFoundError):
        ctx.client.datasets().get(ctx.scope("test"))

    # Create dataset
    create_resp = ctx.client.datasets().create(ctx.scope("test"))
    print(create_resp)
    d = create_resp.dataset

    # Get dataset
    response = ctx.client.datasets().get(ctx.scope("test"))
    print(response)
    assert response.dataset == d
