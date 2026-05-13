import asyncio
import os
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from typing import Any, Coroutine
from uuid import uuid4

from topk_sdk import AsyncClient, Client
from topk_sdk.error import CollectionNotFoundError, DatasetNotFoundError


@dataclass
class ProjectContext:
    client: Client
    scope_prefix: str
    used: set[str]

    def scope(self, name: str):
        wrapped = f"{self.scope_prefix}-{name}"
        self.used.add(wrapped)
        return wrapped

    def cleanup(self):
        def delete_dataset(name: str):
            try:
                self.client.datasets().delete(name)
            except DatasetNotFoundError:
                pass
            except Exception as e:
                print(f"Teardown error deleting dataset {name}: {e}")

        def delete_collection(name: str):
            try:
                self.client.collections().delete(name)
            except CollectionNotFoundError:
                pass
            except Exception as e:
                print(f"Teardown error deleting collection {name}: {e}")

        with ThreadPoolExecutor() as executor:
            executor.map(delete_dataset, self.used)
            executor.map(delete_collection, self.used)


@dataclass
class AsyncProjectContext:
    client: AsyncClient
    scope_prefix: str
    used: set[str]

    def scope(self, name: str):
        wrapped = f"{self.scope_prefix}-{name}"
        self.used.add(wrapped)
        return wrapped

    async def cleanup(self):
        futs: list[Coroutine[Any, Any, None]] = []

        async def delete_dataset(name: str):
            try:
                await self.client.datasets().delete(name)
            except DatasetNotFoundError:
                pass
            except Exception as e:
                print(f"Teardown error deleting dataset {name}: {e}")

        async def delete_collection(name: str):
            try:
                await self.client.collections().delete(name)
            except CollectionNotFoundError:
                pass
            except Exception as e:
                print(f"Teardown error deleting collection {name}: {e}")

        for name in self.used:
            futs.append(delete_dataset(name))
            futs.append(delete_collection(name))

        await asyncio.gather(*futs)


def new_project_context():
    TOPK_API_KEY = os.environ["TOPK_API_KEY"].splitlines()[0].strip()
    TOPK_HOST = os.environ.get("TOPK_HOST", "topk.io")
    TOPK_REGION = os.environ.get("TOPK_REGION", "elastica")
    TOPK_HTTPS = os.environ.get("TOPK_HTTPS", "true") == "true"

    client = Client(
        api_key=TOPK_API_KEY,
        region=TOPK_REGION,
        host=TOPK_HOST,
        https=TOPK_HTTPS,
    )

    return ProjectContext(
        scope_prefix=f"topk-py-{uuid4()}",
        client=client,
        used=set(),
    )


def new_async_project_context():
    TOPK_API_KEY = os.environ["TOPK_API_KEY"].splitlines()[0].strip()
    TOPK_HOST = os.environ.get("TOPK_HOST", "topk.io")
    TOPK_REGION = os.environ.get("TOPK_REGION", "elastica")
    TOPK_HTTPS = os.environ.get("TOPK_HTTPS", "true") == "true"

    client = AsyncClient(
        api_key=TOPK_API_KEY,
        region=TOPK_REGION,
        host=TOPK_HOST,
        https=TOPK_HTTPS,
    )

    return AsyncProjectContext(
        scope_prefix=f"topk-py-{uuid4()}",
        client=client,
        used=set(),
    )
