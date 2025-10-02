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
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing a 32-bit float vector. This function is an alias for [f32_list()](https://docs.topk.io/sdk/topk-py/data#f32-list).

    Example:

    ```python
    from topk_sdk.data import f32_vector

    f32_vector([0.12, 0.67, 0.82, 0.53])
    ```
    """
    ...

def u8_vector(data: builtins.list[int]) -> List:
    """
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing an 8-bit unsigned integer vector. This function is an alias for [u8_list()](https://docs.topk.io/sdk/topk-py/data#u8-list).

    Example:

    ```python
    from topk_sdk.data import u8_vector

    u8_vector([0, 255, 1, 2, 3])
    ```
    """
    ...

def i8_vector(data: builtins.list[int]) -> List:
    """
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing an 8-bit signed integer vector. This function is an alias for [i8_list()](https://docs.topk.io/sdk/topk-py/data#i8-list).

    Example:

    ```python
    from topk_sdk.data import i8_vector

    i8_vector([-128, 127, -1, 0, 1])
    ```
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
