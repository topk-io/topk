import builtins
import typing
from enum import Enum

from . import data, error, query, schema  # noqa

class Client:
    def __new__(
        cls,
        api_key: builtins.str,
        region: builtins.str,
        host: builtins.str = "topk.io",
        https: builtins.bool = True,
        retry_config: typing.Optional[RetryConfig] = None,
    ) -> Client: ...
    def collection(self, collection: builtins.str) -> CollectionClient: ...
    def collections(self) -> CollectionsClient: ...

class Collection:
    name: builtins.str
    org_id: builtins.str
    project_id: builtins.str
    region: builtins.str
    schema: builtins.dict[builtins.str, schema.FieldSpec]
    def __new__(
        cls,
        name: builtins.str,
        org_id: builtins.str,
        project_id: builtins.str,
        region: builtins.str,
        schema: typing.Mapping[builtins.str, schema.FieldSpec],
    ) -> Collection: ...
    def __repr__(self) -> builtins.str: ...
    def __eq__(self, other: Collection) -> builtins.bool: ...

class CollectionClient:
    def get(
        self,
        ids: typing.Sequence[builtins.str],
        fields: typing.Optional[typing.Sequence[builtins.str]] = None,
        lsn: typing.Optional[builtins.str] = None,
        consistency: typing.Optional[ConsistencyLevel] = None,
    ) -> builtins.dict[builtins.str, builtins.dict[builtins.str, typing.Any]]: ...
    def count(
        self,
        lsn: typing.Optional[builtins.str] = None,
        consistency: typing.Optional[ConsistencyLevel] = None,
    ) -> builtins.int: ...
    def query(
        self,
        query: query.Query,
        lsn: typing.Optional[builtins.str] = None,
        consistency: typing.Optional[ConsistencyLevel] = None,
    ) -> builtins.list[builtins.dict[builtins.str, typing.Any]]: ...
    def upsert(
        self, documents: typing.Sequence[typing.Mapping[builtins.str, typing.Any]]
    ) -> builtins.str: ...
    def delete(self, ids: typing.Sequence[builtins.str]) -> builtins.str: ...

class CollectionsClient:
    def get(self, collection_name: builtins.str) -> Collection: ...
    def list(self) -> builtins.list[Collection]: ...
    def create(
        self,
        collection_name: builtins.str,
        schema: typing.Mapping[builtins.str, schema.FieldSpec],
    ) -> Collection: ...
    def delete(self, collection_name: builtins.str) -> None: ...

class RetryConfig: ...

class ConsistencyLevel(Enum):
    Indexed = ...
    Strong = ...
