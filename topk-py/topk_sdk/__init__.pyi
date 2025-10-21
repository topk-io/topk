import builtins
import typing
from enum import Enum

from . import query, schema

class Client:
    """
    Client for interacting with the TopK API. For available regions see https://docs.topk.io/regions
    """

    def __init__(
        self,
        api_key: builtins.str,
        region: builtins.str,
        host: builtins.str = "topk.io",
        https: builtins.bool = True,
        retry_config: typing.Optional[RetryConfig | dict[builtins.str, typing.Any]] = None,
    ) -> None: ...
    def collection(self, collection: builtins.str) -> CollectionClient:
        """
        Get a client for managing data operations on a specific collection such as querying, upserting, and deleting documents.
        """
        ...
    def collections(self) -> CollectionsClient:
        """
        Get a client for managing collections.
        """
        ...

class AsyncClient:
    """
    Async client for interacting with the TopK API. For available regions see https://docs.topk.io/regions
    """

    def __init__(
        self,
        api_key: builtins.str,
        region: builtins.str,
        host: builtins.str = "topk.io",
        https: builtins.bool = True,
        retry_config: typing.Optional[RetryConfig | dict[builtins.str, typing.Any]] = None,
    ) -> None: ...
    def collection(self, collection: builtins.str) -> AsyncCollectionClient:
        """Get an async client for a specific collection."""
        ...
    def collections(self) -> AsyncCollectionsClient:
        """Get an async client for managing collections."""
        ...

class CollectionClient:
    """
    Synchronous client for collection operations.
    """

    def get(
        self,
        ids: typing.Sequence[builtins.str],
        fields: typing.Optional[typing.Sequence[builtins.str]] = None,
        lsn: typing.Optional[builtins.str] = None,
        consistency: typing.Optional[ConsistencyLevel] = None,
    ) -> builtins.dict[builtins.str, builtins.dict[builtins.str, typing.Any]]:
        """
        Get documents by their IDs.
        """
        ...
    def count(
        self,
        lsn: typing.Optional[builtins.str] = None,
        consistency: typing.Optional[ConsistencyLevel] = None,
    ) -> builtins.int:
        """
        Get the count of documents in the collection.
        """
        ...
    def query(
        self,
        query: query.Query,
        lsn: typing.Optional[builtins.str] = None,
        consistency: typing.Optional[ConsistencyLevel] = None,
    ) -> builtins.list[builtins.dict[builtins.str, typing.Any]]:
        """
        Execute a query against the collection.
        """
        ...
    def upsert(
        self, documents: typing.Sequence[typing.Mapping[builtins.str, typing.Any]]
    ) -> builtins.str:
        """
        Insert or update documents in the collection.
        """
        ...
    def delete(self, expr: typing.Union[typing.Sequence[builtins.str], query.LogicalExpr]) -> builtins.str:
        """
        Delete documents by their IDs or using a filter expression.

        **Example:**

        Delete documents by their IDs:
        ```python
        client.collection("books").delete(["id_1", "id_2"])
        ```

        Delete documents by a filter expression:
        ```python
        from topk_sdk.query import field

        client.collection("books").delete(field("published_year").gt(1997))
        ```
        """
        ...

class AsyncCollectionClient:
    """
    Asynchronous client for collection operations.
    """

    def get(
        self,
        ids: typing.Sequence[builtins.str],
        fields: typing.Optional[typing.Sequence[builtins.str]] = None,
        lsn: typing.Optional[builtins.str] = None,
        consistency: typing.Optional[ConsistencyLevel] = None,
    ) -> typing.Awaitable[
        builtins.dict[builtins.str, builtins.dict[builtins.str, typing.Any]]
    ]:
        """
        Get documents by their IDs asynchronously.
        """
        ...
    def count(
        self,
        lsn: typing.Optional[builtins.str] = None,
        consistency: typing.Optional[ConsistencyLevel] = None,
    ) -> typing.Awaitable[builtins.int]:
        """
        Get the count of documents in the collection asynchronously.
        """
        ...
    def query(
        self,
        query: query.Query,
        lsn: typing.Optional[builtins.str] = None,
        consistency: typing.Optional[ConsistencyLevel] = None,
    ) -> typing.Awaitable[builtins.list[builtins.dict[builtins.str, typing.Any]]]:
        """
        Execute a query against the collection asynchronously.
        """
        ...
    def upsert(
        self, documents: typing.Sequence[typing.Mapping[builtins.str, typing.Any]]
    ) -> typing.Awaitable[builtins.str]:
        """
        Insert or update documents in the collection asynchronously.
        """
        ...
    def delete(
        self, expr: typing.Union[typing.Sequence[builtins.str], query.LogicalExpr]
    ) -> typing.Awaitable[builtins.str]:
        """
        Delete documents by their IDs or using a filter expression asynchronously.

        **Example:**

        Delete documents by their IDs:
        ```python
        await client.collection("books").delete(["id_1", "id_2"])
        ```

        Delete documents by a filter expression:
        ```python
        from topk_sdk.query import field

        await client.collection("books").delete(field("published_year").gt(1997))
        ```
        """
        ...

class Collection:
    """
    Represents a collection in the TopK system.
    """

    name: builtins.str
    org_id: builtins.str
    project_id: builtins.str
    region: builtins.str
    schema: builtins.dict[builtins.str, schema.FieldSpec]

class CollectionsClient:
    """
    Synchronous client for managing collections.
    """

    def get(self, collection_name: builtins.str) -> Collection:
        """
        Get information about a specific collection.
        """
        ...
    def list(self) -> builtins.list[Collection]:
        """
        List all collections.
        """
        ...
    def create(
        self,
        collection_name: builtins.str,
        schema: typing.Mapping[builtins.str, schema.FieldSpec],
    ) -> Collection:
        """
        Create a new collection with the specified schema.
        """
        ...
    def delete(self, collection_name: builtins.str) -> None:
        """
        Delete a collection.
        """
        ...

class AsyncCollectionsClient:
    """
    Asynchronous client for managing collections.
    """

    def get(self, collection_name: builtins.str) -> typing.Awaitable[Collection]:
        """
        Get information about a specific collection asynchronously.
        """
        ...
    def list(self) -> typing.Awaitable[builtins.list[Collection]]:
        """
        List all collections asynchronously.
        """
        ...
    def create(
        self,
        collection_name: builtins.str,
        schema: typing.Mapping[builtins.str, schema.FieldSpec],
    ) -> typing.Awaitable[Collection]:
        """
        Create a new collection with the specified schema asynchronously.
        """
        ...
    def delete(self, collection_name: builtins.str) -> typing.Awaitable[None]:
        """
        Delete a collection asynchronously.
        """
        ...

class ConsistencyLevel(Enum):
    """
    Enumeration of consistency levels for operations.
    """

    Indexed = "indexed"
    Strong = "strong"

class RetryConfig:
    """
    Configuration for retry behavior.

    By default, retries occur in two situations:
    1. When the server requests the client to reduce its request rate, resulting in a [SlowDownError](https://docs.topk.io/sdk/topk-py/error#slowdownerror).
    2. When using the `query(..., lsn=N)` to wait for writes to be available.
    """

    def __init__(
        self,
        max_retries: typing.Optional[builtins.int] = None,
        timeout: typing.Optional[builtins.int] = None,
        backoff: typing.Optional[BackoffConfig] = None,
    ) -> None: ...

    max_retries: typing.Annotated[typing.Optional[builtins.int], "Maximum number of retries to attempt. Default is 3 retries."]
    """
    Maximum number of retries to attempt.
    Default is 3 retries.
    """
    timeout: typing.Annotated[typing.Optional[builtins.int], "The total timetout for the retry chain in milliseconds. Default is 30,000 milliseconds (30 seconds)"]
    """
    The total timeout for the retry chain in milliseconds.
    Default is 30,000 milliseconds (30 seconds).
    """
    backoff: typing.Annotated[typing.Optional[BackoffConfig], "The backoff configuration for the client."]
    """
    The backoff configuration for the client.
    """

class BackoffConfig:
    """
    Configuration for backoff behavior in retries.
    """

    def __init__(
        self,
        base: typing.Optional[builtins.int] = None,
        init_backoff: typing.Optional[builtins.int] = None,
        max_backoff: typing.Optional[builtins.int] = None,
    ) -> None: ...

    base: typing.Annotated[typing.Optional[builtins.int], "The base for the backoff. Default is 2x backoff."]
    """
    The base for the backoff. Default is 2x backoff.
    """
    init_backoff: typing.Annotated[typing.Optional[builtins.int], "The initial backoff in milliseconds. Default is 100 milliseconds."]
    """
    The initial backoff in milliseconds.
    Default is 100 milliseconds.
    """
    max_backoff: typing.Annotated[typing.Optional[builtins.int], "The maximum backoff in milliseconds. Default is 10,000 milliseconds (10 seconds)."]
    """
    The maximum backoff in milliseconds.
    Default is 10,000 milliseconds (10 seconds).
    """
