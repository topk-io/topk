import builtins
import typing
from enum import Enum

from . import query, schema


class Client:
    """
    Synchronous client for interacting with the TopK API.
    """

    def __init__(
        self,
        api_key: builtins.str,
        region: builtins.str,
        host: builtins.str = "topk.io",
        https: builtins.bool = True,
        retry_config: typing.Optional[RetryConfig] = None,
    ) -> None:
        ...
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
    Asynchronous client for interacting with the TopK API.
    """

    def __init__(
        self,
        api_key: builtins.str,
        region: builtins.str,
        host: builtins.str = "topk.io",
        https: builtins.bool = True,
        retry_config: typing.Optional[RetryConfig] = None,
    ) -> None:
        ...
    def collection(self, collection: builtins.str) -> AsyncCollectionClient:
        """Get an async client for a specific collection.
        """
        ...
    def collections(self) -> AsyncCollectionsClient:
        """Get an async client for managing collections.
        """
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
    def delete(self, ids: typing.Sequence[builtins.str]) -> builtins.str:
        """
        Delete documents by their IDs.
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
    ) -> typing.Awaitable[builtins.dict[builtins.str, builtins.dict[builtins.str, typing.Any]]]:
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
    def delete(self, ids: typing.Sequence[builtins.str]) -> typing.Awaitable[builtins.str]:
        """
        Delete documents by their IDs asynchronously.
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
    """

    max_retries: typing.Optional[builtins.int]
    timeout: typing.Optional[builtins.int]
    backoff: typing.Optional[BackoffConfig]


class BackoffConfig:
    """
    Configuration for backoff behavior in retries.
    """

    base: typing.Optional[builtins.int]
    init_backoff: typing.Optional[builtins.int]
    max_backoff: typing.Optional[builtins.int]
