import pytest


@pytest.fixture
def ctx():
    from . import new_project_context

    ctx = new_project_context()

    yield ctx

    ctx.cleanup()
