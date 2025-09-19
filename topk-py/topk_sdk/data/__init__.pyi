import builtins
import typing

class List:
    """Internal class for representing list data types."""
    ...

class SparseVector:
    """Internal class for representing sparse vector data types."""
    ...

def f32_vector(data: builtins.list[float]) -> List:
    """Create a 32-bit float vector.

    :param data: A list of 32-bit floating point values.
    :type data: builtins.list[float]
    :return: A List containing the vector data.
    :rtype: List

    .. code-block:: python

        from topk_sdk.data import f32_vector

        vector = f32_vector([1.0, 2.5, -0.8, 4.2])
    """
    ...

def u8_vector(data: builtins.list[int]) -> List:
    """Create an 8-bit unsigned integer vector.

    :param data: A list of 8-bit unsigned integer values (0-255).
    :type data: builtins.list[int]
    :return: A List containing the vector data.
    :rtype: List

    .. code-block:: python

        from topk_sdk.data import u8_vector

        vector = u8_vector([1, 2, 3, 4])
    """
    ...

def binary_vector(data: builtins.list[int]) -> List:
    """Create a binary vector.

    :param data: A list of binary values (0 or 1).
    :type data: builtins.list[int]
    :return: A List containing the binary vector data.
    :rtype: List

    .. code-block:: python

        from topk_sdk.data import binary_vector

        vector = binary_vector([1, 0, 1, 0])
    """
    ...

def f32_sparse_vector(data: builtins.dict[int, float]) -> SparseVector:
    """Create a 32-bit float sparse vector.

    :param data: A dictionary mapping indices to 32-bit float values.
    :type data: builtins.dict[int, float]
    :return: A SparseVector containing the sparse vector data.
    :rtype: SparseVector

    .. code-block:: python

        from topk_sdk.data import f32_sparse_vector

        vector = f32_sparse_vector({0: 1.0, 1: 2.5, 2: -0.8, 3: 4.2})
    """
    ...

def u8_sparse_vector(data: builtins.dict[int, int]) -> SparseVector:
    """Create an 8-bit unsigned integer sparse vector.

    :param data: A dictionary mapping indices to 8-bit unsigned integer values.
    :type data: builtins.dict[int, int]
    :return: A SparseVector containing the sparse vector data.
    :rtype: SparseVector

    .. code-block:: python

        from topk_sdk.data import u8_sparse_vector

        vector = u8_sparse_vector({0: 1, 1: 2, 2: 3, 3: 4})
    """
    ...

def bytes(data: typing.Union[builtins.list[int], builtins.bytes]) -> List:
    """Create a bytes data structure.

    :param data: Either raw bytes or a list of integers (0-255).
    :type data: typing.Union[builtins.list[int], builtins.bytes]
    :return: A List containing the bytes data.
    :rtype: List

    .. code-block:: python

        from topk_sdk.data import bytes

        vector = bytes([1, 2, 3, 4])
    """
    ...

def u32_list(data: builtins.list[int]) -> List:
    """Create a list of 32-bit unsigned integers.

    :param data: A list of 32-bit unsigned integer values.
    :type data: builtins.list[int]
    :return: A List containing the integer data.
    :rtype: List

    .. code-block:: python

        from topk_sdk.data import u32_list

        vector = u32_list([1, 2, 3, 4])
    """
    ...

def i32_list(data: builtins.list[int]) -> List:
    """Create a list of 32-bit signed integers.

    :param data: A list of 32-bit signed integer values.
    :type data: builtins.list[int]
    :return: A List containing the integer data.
    :rtype: List

    .. code-block:: python

        from topk_sdk.data import i32_list

        vector = i32_list([1, 2, 3, 4])
    """
    ...

def i64_list(data: builtins.list[int]) -> List:
    """Create a list of 64-bit signed integers.

    :param data: A list of 64-bit signed integer values.
    :type data: builtins.list[int]
    :return: A List containing the integer data.
    :rtype: List

    .. code-block:: python

        from topk_sdk.data import i64_list

        vector = i64_list([1, 2, 3, 4])
    """
    ...

def f32_list(data: builtins.list[float]) -> List:
    """Create a list of 32-bit floating point numbers.

    :param data: A list of 32-bit floating point values.
    :type data: builtins.list[float]
    :return: A List containing the float data.
    :rtype: List

    .. code-block:: python

        from topk_sdk.data import f32_list

        vector = f32_list([1.0, 2.5, -0.8, 4.2])
    """
    ...

def f64_list(data: builtins.list[float]) -> List:
    """Create a list of 64-bit floating point numbers.

    :param data: A list of 64-bit floating point values.
    :type data: builtins.list[float]
    :return: A List containing the float data.
    :rtype: List

    .. code-block:: python

        from topk_sdk.data import f64_list

        vector = f64_list([1.0, 2.5, -0.8, 4.2])
    """
    ...

def string_list(data: builtins.list[str]) -> List:
    """Create a list of strings.

    :param data: A list of string values.
    :type data: builtins.list[str]
    :return: A List containing the string data.
    :rtype: List

    .. code-block:: python

        from topk_sdk.data import string_list

        vector = string_list(["hello", "world", "foo", "bar"])
    """
    ...
