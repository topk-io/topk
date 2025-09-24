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
    Create a text field specification.
    """
    ...

def int() -> FieldSpec:
    """
    Create an integer field specification.
    """
    ...

def float() -> FieldSpec:
    """
    Create a float field specification.
    """
    ...

def bool() -> FieldSpec:
    """
    Create a boolean field specification.
    """
    ...

def f32_vector(dimension: builtins.int) -> FieldSpec:
    """
    Create a 32-bit float vector field specification.
    """
    ...

def u8_vector(dimension: builtins.int) -> FieldSpec:
    """
    Create an 8-bit unsigned integer vector field specification.
    """
    ...

def i8_vector(dimension: builtins.int) -> FieldSpec:
    """
    Create an 8-bit signed integer vector field specification.
    """
    ...

def binary_vector(dimension: builtins.int) -> FieldSpec:
    """
    Create a binary vector field specification.
    """
    ...

def f32_sparse_vector() -> FieldSpec:
    """
    Create a f32 sparse vector field specification.
    """
    ...

def u8_sparse_vector() -> FieldSpec:
    """
    Create a u8 sparse vector field specification.
    """
    ...

def bytes() -> FieldSpec:
    """
    Create a bytes field specification.
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
