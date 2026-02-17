import pytest
from topk_sdk import error

from . import ProjectContext


def test_list_datasets(ctx: ProjectContext):
    d = ctx.client.datasets().create(ctx.scope("test"))
    response = ctx.client.datasets().list()
    assert d in response


def test_create_dataset(ctx: ProjectContext):
    d = ctx.client.datasets().create(ctx.scope("test"))
    datasets = ctx.client.datasets().list()
    assert d in datasets


def test_create_duplicate_dataset(ctx: ProjectContext):
    ctx.client.datasets().create(ctx.scope("test"))

    with pytest.raises(error.DatasetAlreadyExistsError):
        ctx.client.datasets().create(ctx.scope("test"))


def test_delete_non_existent_dataset(ctx: ProjectContext):
    with pytest.raises(error.DatasetNotFoundError):
        ctx.client.datasets().delete(ctx.scope("test"))


def test_delete_dataset(ctx: ProjectContext):
    d = ctx.client.datasets().create(ctx.scope("test"))
    ctx.client.datasets().delete(ctx.scope("test"))

    datasets = ctx.client.datasets().list()
    assert d not in datasets


def test_get_dataset(ctx: ProjectContext):
    # Test getting non-existent dataset
    with pytest.raises(error.DatasetNotFoundError):
        ctx.client.datasets().get(ctx.scope("test"))

    # Create dataset
    d = ctx.client.datasets().create(ctx.scope("test"))

    # Get dataset
    dataset = ctx.client.datasets().get(ctx.scope("test"))
    assert dataset == d
