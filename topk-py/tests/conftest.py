import typing

import pytest
import pytest_asyncio

from . import AsyncProjectContext, ProjectContext, new_async_project_context, new_project_context


@pytest.fixture
def ctx() -> typing.Generator[ProjectContext, None, None]:
    context = new_project_context()
    yield context
    context.cleanup()


@pytest_asyncio.fixture
async def async_ctx() -> typing.AsyncGenerator[AsyncProjectContext, None]:
    context = new_async_project_context()
    yield context
    await context.cleanup()
