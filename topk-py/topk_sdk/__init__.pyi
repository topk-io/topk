import builtins
import os
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
    def dataset(self, dataset: builtins.str) -> DatasetClient:
        """
        Get a client for managing data operations on a specific dataset such as upserting files, managing metadata, and deleting files.
        """
        ...
    def datasets(self) -> DatasetsClient:
        """
        Get a client for managing datasets.
        """
        ...
    def ask(
        self,
        query: builtins.str,
        sources: typing.Union[typing.Sequence[Source], typing.Sequence[str], typing.Sequence[dict[builtins.str, typing.Any]]],
        filter: typing.Optional[query.LogicalExpr] = None,
        mode: typing.Optional[typing.Literal["summarize", "reason", "deep_research"]] = None,
        select_fields: typing.Optional[typing.Sequence[builtins.str]] = None,
    ) -> typing.Union[Answer, Search, Reason]:
        """
        Ask a question and wait for the stream to complete, returning the last message.
        """
        ...
    def ask_stream(
        self,
        query: builtins.str,
        sources: typing.Union[typing.Sequence[Source], typing.Sequence[str], typing.Sequence[dict[builtins.str, typing.Any]]],
        filter: typing.Optional[query.LogicalExpr] = None,
        mode: typing.Optional[typing.Literal["summarize", "reason", "deep_research"]] = None,
        select_fields: typing.Optional[typing.Sequence[builtins.str]] = None,
    ) -> typing.Iterator[typing.Union[Answer, Search, Reason]]:
        """
        Ask a question and get streaming responses as an iterator.
        """
        ...
    def search(
        self,
        query: builtins.str,
        sources: typing.Union[typing.Sequence[Source], typing.Sequence[str], typing.Sequence[dict[builtins.str, typing.Any]]],
        filter: typing.Optional[query.LogicalExpr] = None,
        top_k: builtins.int = 10,
        select_fields: typing.Optional[typing.Sequence[builtins.str]] = None,
    ) -> builtins.list[SearchResult]:
        """
        Search for documents and wait for the stream to complete, returning all results.
        """
        ...
    def search_stream(
        self,
        query: builtins.str,
        sources: typing.Union[typing.Sequence[Source], typing.Sequence[str], typing.Sequence[dict[builtins.str, typing.Any]]],
        filter: typing.Optional[query.LogicalExpr] = None,
        top_k: builtins.int = 10,
        select_fields: typing.Optional[typing.Sequence[builtins.str]] = None,
    ) -> SearchIterator:
        """
        Search for documents and get streaming responses as an iterator.
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
    def dataset(self, dataset: builtins.str) -> AsyncDatasetClient:
        """Get an async client for managing data operations on a specific dataset."""
        ...
    def datasets(self) -> AsyncDatasetsClient:
        """Get an async client for managing datasets."""
        ...
    def ask(
        self,
        query: builtins.str,
        sources: typing.Union[typing.Sequence[Source], typing.Sequence[str], typing.Sequence[dict[builtins.str, typing.Any]]],
        filter: typing.Optional[query.LogicalExpr] = None,
        mode: typing.Optional[typing.Literal["summarize", "reason", "deep_research"]] = None,
        select_fields: typing.Optional[typing.Sequence[builtins.str]] = None,
    ) -> typing.Awaitable[typing.Union[Answer, Search, Reason]]:
        """
        Ask a question and wait for the stream to complete asynchronously, returning the last message.
        """
        ...
    def ask_stream(
        self,
        query: builtins.str,
        sources: typing.Union[typing.Sequence[Source], typing.Sequence[str], typing.Sequence[dict[builtins.str, typing.Any]]],
        filter: typing.Optional[query.LogicalExpr] = None,
        mode: typing.Optional[typing.Literal["summarize", "reason", "deep_research"]] = None,
        select_fields: typing.Optional[typing.Sequence[builtins.str]] = None,
    ) -> typing.AsyncIterator[typing.Union[Answer, Search, Reason]]:
        """
        Ask a question and get streaming responses asynchronously as an async iterator.
        """
        ...
    def search(
        self,
        query: builtins.str,
        sources: typing.Union[typing.Sequence[Source], typing.Sequence[str], typing.Sequence[dict[builtins.str, typing.Any]]],
        filter: typing.Optional[query.LogicalExpr] = None,
        top_k: builtins.int = 10,
        select_fields: typing.Optional[typing.Sequence[builtins.str]] = None,
    ) -> typing.Awaitable[builtins.list[SearchResult]]:
        """
        Search for documents and wait for the stream to complete asynchronously, returning all results.
        """
        ...
    def search_stream(
        self,
        query: builtins.str,
        sources: typing.Union[typing.Sequence[Source], typing.Sequence[str], typing.Sequence[dict[builtins.str, typing.Any]]],
        filter: typing.Optional[query.LogicalExpr] = None,
        top_k: builtins.int = 10,
        select_fields: typing.Optional[typing.Sequence[builtins.str]] = None,
    ) -> typing.AsyncIterator[SearchResult]:
        """
        Search for documents and get streaming responses asynchronously as an async iterator.
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

    def update(
        self, documents: typing.Sequence[typing.Mapping[builtins.str, typing.Any]], fail_on_missing: typing.Optional[builtins.bool] = None
    ) -> builtins.str:
        """
        Update documents in the collection.

        Existing documents will be merged with the provided fields.
        Missing documents will be ignored.

        Returns the `LSN` at which the update was applied.
        If no updates were applied, this will be empty.
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
    def update(
        self, documents: typing.Sequence[typing.Mapping[builtins.str, typing.Any]], fail_on_missing: typing.Optional[builtins.bool] = None
    ) -> typing.Awaitable[builtins.str]:
        """
        Update documents in the collection asynchronously.

        Existing documents will be merged with the provided fields.
        Missing documents will be ignored.

        Returns the `LSN` at which the update was applied.
        If no updates were applied, this will be empty.
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

class Dataset:
    """
    Represents a dataset in the TopK system.
    """

    name: builtins.str
    org_id: builtins.str
    project_id: builtins.str
    region: builtins.str

class Response:
    """
    Base class for API response objects with request_id.
    """

    request_id: typing.Optional[builtins.str]

class CreateDatasetResponse(Response):
    """Response from creating a dataset."""

    dataset: Dataset

class GetDatasetResponse(Response):
    """Response from getting a dataset."""

    dataset: Dataset

class ListDatasetsResponse(Response):
    """Response from listing datasets."""

    datasets: builtins.list[Dataset]

class DeleteDatasetResponse(Response):
    """Response from deleting a dataset."""

class UpsertFileResponse(Response):
    """Response from upserting a file."""

    handle: builtins.str

class GetMetadataResponse(Response):
    """Response from getting file metadata."""

    metadata: builtins.dict[builtins.str, typing.Any]

class UpdateMetadataResponse(Response):
    """Response from updating file metadata."""

    handle: builtins.str

class DeleteFileResponse(Response):
    """Response from deleting a file."""

    handle: builtins.str

class CheckHandleResponse(Response):
    """Response from checking handle status."""

    processed: builtins.bool

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

class DatasetsClient:
    """
    Synchronous client for managing datasets.
    """

    def get(self, dataset_name: builtins.str) -> GetDatasetResponse:
        """
        Get information about a specific dataset.
        """
        ...
    def list(self) -> ListDatasetsResponse:
        """
        List all datasets.
        """
        ...
    def create(self, dataset_name: builtins.str) -> CreateDatasetResponse:
        """
        Create a new dataset.
        """
        ...
    def delete(self, dataset_name: builtins.str) -> DeleteDatasetResponse:
        """
        Delete a dataset.
        """
        ...

class DatasetClient:
    """
    Synchronous client for dataset operations.
    """

    def upsert_file(
        self,
        file_id: builtins.str,
        input: typing.Union[os.PathLike[typing.Any], typing.Tuple[builtins.str, builtins.bytes, builtins.str]],
        metadata: typing.Mapping[builtins.str, typing.Any],
    ) -> UpsertFileResponse:
        """
        Upsert a file to the dataset.
        """
        ...
    def get_metadata(
        self,
        file_id: builtins.str,
        fields: typing.Optional[typing.Sequence[builtins.str]] = None,
    ) -> GetMetadataResponse:
        """
        Get metadata for a file.

        If `fields` is provided, only the specified fields will be returned.
        """
        ...
    def update_metadata(
        self,
        file_id: builtins.str,
        metadata: typing.Mapping[builtins.str, typing.Any],
    ) -> UpdateMetadataResponse:
        """
        Update metadata for a file.
        """
        ...
    def delete(self, file_id: builtins.str) -> DeleteFileResponse:
        """
        Delete a file from the dataset.
        """
        ...
    def check_handle(self, handle: builtins.str) -> CheckHandleResponse:
        """
        Check if a handle has been processed.
        """
        ...

class AsyncDatasetsClient:
    """
    Asynchronous client for managing datasets.
    """

    def get(self, dataset_name: builtins.str) -> typing.Awaitable[GetDatasetResponse]:
        """
        Get information about a specific dataset asynchronously.
        """
        ...
    def list(self) -> typing.Awaitable[ListDatasetsResponse]:
        """
        List all datasets asynchronously.
        """
        ...
    def create(self, dataset_name: builtins.str) -> typing.Awaitable[CreateDatasetResponse]:
        """
        Create a new dataset asynchronously.
        """
        ...
    def delete(self, dataset_name: builtins.str) -> typing.Awaitable[DeleteDatasetResponse]:
        """
        Delete a dataset asynchronously.
        """
        ...

class AsyncDatasetClient:
    """
    Asynchronous client for dataset operations.
    """

    def upsert_file(
        self,
        file_id: builtins.str,
        input: typing.Union[os.PathLike[typing.Any], typing.Tuple[builtins.str, builtins.bytes, builtins.str]],
        metadata: typing.Mapping[builtins.str, typing.Any],
    ) -> typing.Awaitable[UpsertFileResponse]:
        """
        Upsert a file to the dataset asynchronously.
        """
        ...
    def get_metadata(
        self,
        file_id: builtins.str,
        fields: typing.Optional[typing.Sequence[builtins.str]] = None,
    ) -> typing.Awaitable[GetMetadataResponse]:
        """
        Get metadata for a file asynchronously.

        If `fields` is provided, only the specified fields will be returned.
        """
        ...
    def update_metadata(
        self,
        file_id: builtins.str,
        metadata: typing.Mapping[builtins.str, typing.Any],
    ) -> typing.Awaitable[UpdateMetadataResponse]:
        """
        Update metadata for a file asynchronously.
        """
        ...
    def delete(self, file_id: builtins.str) -> typing.Awaitable[DeleteFileResponse]:
        """
        Delete a file from the dataset asynchronously.
        """
        ...
    def check_handle(self, handle: builtins.str) -> typing.Awaitable[CheckHandleResponse]:
        """
        Check if a handle has been processed asynchronously.
        """
        ...

class Source:
    """
    Represents a dataset with an optional filter.
    """

    dataset: builtins.str
    filter: typing.Optional[query.LogicalExpr] = None

class Fact:
    """
    Represents a fact in an ask response.
    """

    fact: builtins.str
    source_ids: builtins.list[builtins.str]

class Chunk:
    """
    Text chunk content.
    """

    text: builtins.str
    doc_pages: builtins.list[builtins.int]

class Image:
    """
    Image content.
    """

    data: builtins.bytes
    mime_type: builtins.str

class Page:
    """
    Page content with optional image.
    """

    page_number: builtins.int
    image: typing.Optional[Image]

class Content:
    """
    Content in a search result. One of chunk, page, or image.
    """

    type: typing.Literal["chunk", "page", "image"]
    data: typing.Union[Chunk, Page, Image]

class SearchResult:
    """
    Represents a search result in an ask response.
    """

    doc_id: builtins.str
    doc_type: builtins.str
    dataset: builtins.str
    content: Content
    metadata: builtins.dict[builtins.str, typing.Any]

class Answer:
    """
    Represents a final answer in an ask response.
    """

    facts: builtins.list[Fact]
    sources: builtins.dict[builtins.str, SearchResult]

class Search:
    """
    Represents a sub-query in an ask response.
    """

    objective: builtins.str
    facts: builtins.list[Fact]
    sources: builtins.dict[builtins.str, SearchResult]


class Reason:
    """
    Represents a reason in an ask response.
    """

    thought: builtins.str

class AskIterator:
    """
    Iterator for synchronous ask responses.
    """

    def __iter__(self) -> AskIterator: ...
    def __next__(self) -> typing.Optional[typing.Union[Answer, Search, Reason]]: ...

class AsyncAskIterator:
    """
    Iterator for asynchronous ask responses.
    """

    def __aiter__(self) -> AsyncAskIterator: ...
    def __anext__(self) -> typing.AsyncIterator[typing.Union[Answer, Search, Reason]]: ...

class SearchIterator:
    """
    Iterator for synchronous search responses.
    """

    def __iter__(self) -> SearchIterator: ...
    def __next__(self) -> typing.Optional[SearchResult]: ...

class AsyncSearchIterator:
    """
    Iterator for asynchronous search responses.
    """

    def __aiter__(self) -> AsyncSearchIterator: ...
    def __anext__(self) -> typing.AsyncIterator[SearchResult]: ...

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
