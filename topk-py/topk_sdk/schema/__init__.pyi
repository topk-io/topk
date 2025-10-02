import builtins
import typing

class FieldIndex:
    """
    *Internal*

    Instances of the `FieldIndex` class represents a field index created by [`vector_index`](#vector-index), [`keyword_index`](#keyword-index), or [`semantic_index`](#semantic-index) functions.
    """

    ...

class FieldSpec:
    """
    *Internal*

    Instances of the `FieldSpec` class represents a field specification created by [`text`](#text), [`int`](#int), [`float`](#float), [`bool`](#bool), [`f32_vector`](#f32-vector), [`u8_vector`](#u8-vector), [`i8_vector`](#i8-vector), [`binary_vector`](#binary-vector), [`f32_sparse_vector`](#f32-sparse-vector), [`u8_sparse_vector`](#u8-sparse-vector), [`bytes`](#bytes), or [`list`](#list) functions.
    """

    def required(self) -> FieldSpec: ...
    def optional(self) -> FieldSpec: ...
    def index(self, index: FieldIndex) -> FieldSpec: ...

# data types
def text() -> FieldSpec:
    """
    Create a [FieldSpec](https://docs.topk.io/sdk/topk-py/schema#FieldSpec) type for `text` values.

    Example:

    ```python
    from topk_sdk.schema import text

    client.collections().create("books", schema={
        "title": text()
    })
    ```
    """
    ...

def int() -> FieldSpec:
    """
    Create an integer field specification.
    """
    ...

def float() -> FieldSpec:
    """
    Create a [FieldSpec](https://docs.topk.io/sdk/topk-py/schema#FieldSpec) type for `float` values.

    Example:

    ```python
    from topk_sdk.schema import float

    client.collections().create("books", schema={
        "price": float()
    })
    ```
    """
    ...

def bool() -> FieldSpec:
    """
    Create a [FieldSpec](https://docs.topk.io/sdk/topk-py/schema#FieldSpec) type for `bool` values.

    Example:

    ```python
    from topk_sdk.schema import bool

    client.collections().create("books", schema={
        "is_published": bool()
    })
    ```
    """
    ...

def f32_vector(dimension: builtins.int) -> FieldSpec:
    """
    Create a [FieldSpec](https://docs.topk.io/sdk/topk-py/schema#FieldSpec) type for `f32_vector` values.

    Example:

    ```python
    from topk_sdk.schema import f32_vector

    client.collections().create("books", schema={
        "title_embedding": f32_vector(dimension=1536)
    })
    ```
    """
    ...

def u8_vector(dimension: builtins.int) -> FieldSpec:
    """
    Create a [FieldSpec](https://docs.topk.io/sdk/topk-py/schema#FieldSpec) type for `u8_vector` values.

    Example:

    ```python
    from topk_sdk.schema import u8_vector

    client.collections().create("books", schema={
        "title_embedding": u8_vector(dimension=1536)
    })
    ```
    """
    ...

def i8_vector(dimension: builtins.int) -> FieldSpec:
    """
    Create a [FieldSpec](https://docs.topk.io/sdk/topk-py/schema#FieldSpec) type for `i8_vector` values.

    Example:

    ```python
    from topk_sdk.schema import i8_vector

    client.collections().create("books", schema={
        "title_embedding": i8_vector(dimension=1536)
    })
    ```
    """
    ...

def binary_vector(dimension: builtins.int) -> FieldSpec:
    """
    Create a [FieldSpec](https://docs.topk.io/sdk/topk-py/schema#FieldSpec) type for `binary_vector` values.

    Example:

    ```python
    from topk_sdk.schema import binary_vector

    client.collections().create("books", schema={
        "title_embedding": binary_vector(dimension=128)
    })
    ```
    """
    ...

def f32_sparse_vector() -> FieldSpec:
    """
    Create a [FieldSpec](https://docs.topk.io/sdk/topk-py/schema#FieldSpec) type for `f32_sparse_vector` values.

    Example:

    ```python
    from topk_sdk.schema import f32_sparse_vector

    client.collections().create("books", schema={
        "sparse_field": f32_sparse_vector()
    })
    ```
    """
    ...

def u8_sparse_vector() -> FieldSpec:
    """
    Create a [FieldSpec](https://docs.topk.io/sdk/topk-py/schema#FieldSpec) type for `u8_sparse_vector` values.

    Example:

    ```python
    from topk_sdk.schema import u8_sparse_vector

    client.collections().create("books", schema={
        "sparse_field": u8_sparse_vector()
    })
    ```
    """
    ...

def bytes() -> FieldSpec:
    """
    Create a [FieldSpec](https://docs.topk.io/sdk/topk-py/schema#FieldSpec) type for `bytes` values.

    Example:

    ```python
    from topk_sdk.schema import bytes

    client.collections().create("books", schema={
        "image": bytes()
    })
    ```
    """
    ...

def list(value_type: typing.Literal["text", "integer", "float"]) -> FieldSpec:
    """
    Create a list field specification.
    """
    ...

# indexes
def vector_index(
    metric: typing.Literal["cosine", "euclidean", "dot_product", "hamming"],
) -> FieldIndex:
    """
    Create a vector index on a vector field.
    """
    ...

def keyword_index() -> FieldIndex:
    """
    Create a keyword index on a text field.
    """
    ...

def semantic_index(model: str) -> FieldIndex:
    """
    Create a semantic index on a text field.
    """
    ...
