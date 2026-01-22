import builtins
import typing
import numpy

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

class Matrix:
    """
    *Internal*

    Instances of the `Matrix` class are used to represent matrices in TopK.
    Usually created using data constructors such as [`matrix()`](#matrix-2).
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
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing an 8-bit signed integer vector.

    Example:

    ```python
    from topk_sdk.data import i8_vector

    i8_vector([-128, 127, -1, 0, 1])
    ```
    """
    ...

def binary_vector(data: builtins.list[int]) -> List:
    """
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing a binary vector.

    Example:

    ```python
    from topk_sdk.data import binary_vector

    binary_vector([0, 1, 1, 0])
    ```
    """
    ...

def f32_sparse_vector(data: builtins.dict[int, float]) -> SparseVector:
    """
    Create a [SparseVector](https://docs.topk.io/sdk/topk-py/data#SparseVector) type containing a 32-bit float sparse vector.

    Example:

    ```python
    from topk_sdk.data import f32_sparse_vector

    f32_sparse_vector({0: 0.12, 6: 0.67, 17: 0.82, 97: 0.53})
    ```
    """
    ...

def u8_sparse_vector(data: builtins.dict[int, int]) -> SparseVector:
    """
    Create a [SparseVector](https://docs.topk.io/sdk/topk-py/data#SparseVector) type containing an 8-bit unsigned integer sparse vector.

    Example:

    ```python
    from topk_sdk.data import u8_sparse_vector

    u8_sparse_vector({0: 12, 6: 67, 17: 82, 97: 53})
    ```
    """
    ...

def bytes(data: typing.Union[builtins.list[int], builtins.bytes]) -> List:
    """
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing bytes data.

    Example:

    ```python
    from topk_sdk.data import bytes

    bytes([0, 1, 1, 0])
    ```
    """
    ...

def u32_list(data: builtins.list[int]) -> List:
    """
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing a list of 32-bit unsigned integers.

    Example:

    ```python
    from topk_sdk.data import u32_list

    u32_list([0, 1, 2, 3])
    ```
    """
    ...

def i32_list(data: builtins.list[int]) -> List:
    """
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing a list of 32-bit signed integers.

    Example:

    ```python
    from topk_sdk.data import i32_list

    i32_list([0, 1, 2, 3])
    ```
    """
    ...

def i64_list(data: builtins.list[int]) -> List:
    """
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing a list of 64-bit signed integers.

    Example:

    ```python
    from topk_sdk.data import i64_list

    i64_list([0, 1, 2, 3])
    ```
    """
    ...

def f32_list(data: builtins.list[float]) -> List:
    """
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing a list of 32-bit floating point numbers.

    Example:

    ```python
    from topk_sdk.data import f32_list

    f32_list([0.12, 0.67, 0.82, 0.53])
    ```
    """
    ...

def f64_list(data: builtins.list[float]) -> List:
    """
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing a list of 64-bit floating point numbers.

    Example:

    ```python
    from topk_sdk.data import f64_list

    f64_list([0.12, 0.67, 0.82, 0.53])
    ```
    """
    ...

def string_list(data: builtins.list[str]) -> List:
    """
    Create a [List](https://docs.topk.io/sdk/topk-py/data#List) type containing a list of strings.

    Example:

    ```python
    from topk_sdk.data import string_list

    string_list(["foo", "bar", "baz"])
    ```
    """
    ...


def matrix(
    values: typing.Union[
        builtins.list[builtins.list[float]],
        builtins.list[builtins.list[int]],
        numpy.ndarray,
    ],
    value_type: typing.Optional[typing.Literal["f32", "f16", "f8", "u8", "i8"]] = None,
) -> Matrix:
    """
    Create a [Matrix](https://docs.topk.io/sdk/topk-py/data#Matrix) type containing matrix values.

    The `values` parameter can be a list of lists or a [numpy array](https://numpy.org/doc/stable/reference/generated/numpy.array.html). When passing a numpy array,
    the matrix type is inferred from the array's dtype (float32, float16, uint8, int8).
    When passing a list of lists, the optional `value_type` parameter specifies the matrix type.
    If `value_type` is not provided, the matrix defaults to f32.

    ```python
    from topk_sdk.data import matrix
    import numpy as np

    # List of lists with explicit type
    matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]], "f32")

    # List of lists defaults to f32
    matrix([[1.0, 2.0], [3.0, 4.0]])

    # Numpy array infers type from dtype
    matrix(np.array([[1.0, 2.0], [3.0, 4.0]], dtype=np.float16))
    ```
    """
    ...
