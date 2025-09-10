import pytest
import pytest_asyncio


@pytest.fixture
def ctx():
    from . import new_project_context

    ctx = new_project_context()

    yield ctx

    ctx.cleanup()


@pytest_asyncio.fixture
async def async_ctx():
    from . import new_async_project_context

    ctx = new_async_project_context()

    yield ctx

    await ctx.cleanup()
