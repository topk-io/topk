import builtins
import typing

class List:
    """
    *Internal*

    Instances of the `List` class are used to represent lists of values in TopK.
    Usually created using data constructors such as [`f32_list()`](#f32-list), [`i32_list()`](#i32-list), etc.
    """
    ...


class SparseVector:
    """
    *Internal*

    Instances of the `SparseVector` class are used to represent sparse vectors in TopK.
    Usually created using data constructors such as [`f32_sparse_vector()`](#f32-sparse-vector) or [`u8_sparse_vector()`](#u8-sparse-vector).
    """
    ...


def f32_vector(data: builtins.list[float]) -> List:
    """
    Create a 32-bit float vector.
    """
    ...


def u8_vector(data: builtins.list[int]) -> List:
    """
    Create an 8-bit unsigned integer vector.
    """
    ...


def i8_vector(data: builtins.list[int]) -> List:
    """
    Create an 8-bit signed integer vector.
    """
    ...


def binary_vector(data: builtins.list[int]) -> List:
    """
    Create a binary vector.
    """
    ...


def f32_sparse_vector(data: builtins.dict[int, float]) -> SparseVector:
    """
    Create a 32-bit float sparse vector.
    """
    ...


def u8_sparse_vector(data: builtins.dict[int, int]) -> SparseVector:
    """
    Create an 8-bit unsigned integer sparse vector.
    """
    ...


def bytes(data: typing.Union[builtins.list[int], builtins.bytes]) -> List:
    """
    Create a bytes data structure.
    """
    ...


def u32_list(data: builtins.list[int]) -> List:
    """
    Create a list of 32-bit unsigned integers.
    """
    ...


def i32_list(data: builtins.list[int]) -> List:
    """
    Create a list of 32-bit signed integers.
    """
    ...


def i64_list(data: builtins.list[int]) -> List:
    """
    Create a list of 64-bit signed integers.
    """
    ...


def f32_list(data: builtins.list[float]) -> List:
    """
    Create a list of 32-bit floating point numbers.
    """
    ...


def f64_list(data: builtins.list[float]) -> List:
    """
    Create a list of 64-bit floating point numbers.
    """
    ...


def string_list(data: builtins.list[str]) -> List:
    """
    Create a list of strings.
    """
    ...
