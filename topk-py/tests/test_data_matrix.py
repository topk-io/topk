import pytest
import numpy as np
from typing import Literal
from topk_sdk import data
from topk_sdk.error import InvalidArgumentError

# Basic matrix creation with list of lists (f32)
def test_matrix_f32_list_of_lists():
    result = data.matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]], "f32")
    assert result is not None
    assert str(result) == "Matrix(3, F32([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]))"


def test_matrix_f32_single_row():
    result = data.matrix([[1.0, 2.0, 3.0]], "f32")
    assert result is not None
    assert str(result) == "Matrix(3, F32([1.0, 2.0, 3.0]))"


def test_matrix_f32_default_type():
    result = data.matrix([[1.0, 2.0], [3.0, 4.0]])
    assert result is not None
    assert str(result) == "Matrix(2, F32([1.0, 2.0, 3.0, 4.0]))"


def test_matrix_f32_keyword_argument():
    result = data.matrix([[1.0, 2.0], [3.0, 4.0]], value_type="f32")
    assert result is not None
    assert str(result) == "Matrix(2, F32([1.0, 2.0, 3.0, 4.0]))"


@pytest.mark.parametrize(
    "value_type,expected_str,test_data",
    [
        ("f16", "Matrix(2, F16([1.0, 2.0, 3.0, 4.0]))", [[1.0, 2.0], [3.0, 4.0]]),
        ("f8", "Matrix(2, F8([1.0, 2.0, 3.0, 4.0]))", [[1.0, 2.0], [3.0, 4.0]]),
        ("u8", "Matrix(3, U8([0, 1, 255, 128, 64, 32]))", [[0, 1, 255], [128, 64, 32]]),
        ("i8", "Matrix(3, I8([-128, -1, 0, 1, 127, 64]))", [[-128, -1, 0], [1, 127, 64]]),
    ],
)
def test_matrix_value_types(
    value_type: Literal["f32", "f16", "f8", "u8", "i8"],
    expected_str: str,
    test_data: list[list[float]] | list[list[int]],
) -> None:
    result = data.matrix(test_data, value_type)
    assert result is not None
    assert str(result) == expected_str



# Numpy array tests
@pytest.mark.parametrize(
    "dtype,arr_data,expected_str",
    [
        # Basic shapes
        (np.float32, [1.0, 2.0, 3.0], "Matrix(3, F32([1.0, 2.0, 3.0]))"),
        (np.float32, [[1.0, 2.0], [3.0, 4.0]], "Matrix(2, F32([1.0, 2.0, 3.0, 4.0]))"),
        # Different dtypes
        (np.float16, [[1.0, 2.0], [3.0, 4.0]], "Matrix(2, F16([1.0, 2.0, 3.0, 4.0]))"),
        (np.uint8, [[0, 1, 255], [128, 64, 32]], "Matrix(3, U8([0, 1, 255, 128, 64, 32]))"),
        (np.int8, [[-128, -1, 0], [1, 127, 64]], "Matrix(3, I8([-128, -1, 0, 1, 127, 64]))"),
        # Various sizes
        (np.float32, [1.0], "Matrix(1, F32([1.0]))"),  # Single element 1D
        (np.float32, [[1.0]], "Matrix(1, F32([1.0]))"),  # Single element 2D
        (np.float32, [1.0, 2.0, 3.0, 4.0, 5.0], "Matrix(5, F32([1.0, 2.0, 3.0, 4.0, 5.0]))"),  # Longer 1D
        (np.float32, [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]], "Matrix(2, F32([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]))"),  # Multiple rows
        (np.float32, [[1.0, 2.0, 3.0, 4.0]], "Matrix(4, F32([1.0, 2.0, 3.0, 4.0]))"),  # Single row, multiple cols
    ],
)
def test_matrix_numpy_arrays(dtype: np.dtype, arr_data: list[list[float]], expected_str: str):
    arr = np.array(arr_data, dtype=dtype)
    result = data.matrix(arr)
    assert result is not None
    assert str(result) == expected_str


@pytest.mark.parametrize(
    "dtype,arr_data,expected_str",
    [
        (np.float32, [[1.0, 2.0], [3.0, 4.0]], "Matrix(2, F32([1.0, 2.0, 3.0, 4.0]))"),
        (np.float16, [[1.0, 2.0], [3.0, 4.0]], "Matrix(2, F16([1.0, 2.0, 3.0, 4.0]))"),
        (np.uint8, [[0, 1], [2, 3]], "Matrix(2, U8([0, 1, 2, 3]))"),
        (np.int8, [[-1, 0], [1, 2]], "Matrix(2, I8([-1, 0, 1, 2]))"),
    ],
)
def test_matrix_numpy_readonly_array(dtype: np.dtype, arr_data: list[list[float]], expected_str: str):
    # Test that readonly arrays work
    arr = np.array(arr_data, dtype=dtype)
    arr.setflags(write=False)  # Make it readonly
    result = data.matrix(arr)
    assert result is not None
    assert str(result) == expected_str


# Invalid state tests
@pytest.mark.parametrize(
    "value_type,test_data",
    [
        ("u8", [[0.0, 1.5, 255.0], [128.0, 64.0, 32.0]]),
        ("i8", [[-128.0, -1.0, 0.0], [1.0, 127.0, 64.0]]),
    ],
)
def test_matrix_int_types_with_floats_invalid(value_type: Literal["u8", "i8"], test_data: list[list[float]]):
    # u8/i8 should not accept floats - should raise TypeError
    with pytest.raises(TypeError, match="'float' object cannot be interpreted as an integer"):
        data.matrix(test_data, value_type)


def test_matrix_numpy_3d_invalid():
    arr = np.array([[[1.0, 2.0], [3.0, 4.0]], [[5.0, 6.0], [7.0, 8.0]]], dtype=np.float32)
    with pytest.raises(InvalidArgumentError, match="Expected numpy array with ndim=1 or ndim=2"):
        data.matrix(arr)


def test_matrix_empty_list():
    with pytest.raises(InvalidArgumentError, match="Cannot create matrix from empty list"):
        data.matrix([])

def test_matrix_empty_list_of_lists():
    with pytest.raises(InvalidArgumentError, match="Cannot create matrix from empty list"):
        data.matrix([[], [1.0, 2.0]])

def test_matrix_mismatched_row_lengths():
    with pytest.raises(
        InvalidArgumentError,
        match=r"len\(values\) must be divisible by num_cols",
    ):
        data.matrix([[1.0, 2.0], [3.0, 4.0, 5.0]])


def test_matrix_invalid_value_type():
    with pytest.raises(
        InvalidArgumentError,
        match="Unsupported value_type:.*Supported types: f8, f16, f32, u8, i8",
    ):
        data.matrix([[1.0, 2.0], [3.0, 4.0]], "invalid_type")  # type: ignore


def test_matrix_invalid_input_type():
    with pytest.raises(TypeError, match="'str' object cannot be cast as 'list'"):
        data.matrix("not a list or array")  # type: ignore


@pytest.mark.parametrize(
    "dtype,test_data",
    [
        (np.float64, [[1.0, 2.0], [3.0, 4.0]]),
        (np.int32, [[1, 2], [3, 4]]),
    ],
)
def test_matrix_invalid_numpy_dtypes(dtype: np.dtype, test_data: list[list[float]] | list[list[int]]):
    arr = np.array(test_data, dtype=dtype)
    with pytest.raises(Exception, match="Unsupported numpy dtype"):
        data.matrix(arr)


@pytest.mark.parametrize(
    "arr_data",
    [
        [],
        [[]],
    ],
)
def test_matrix_numpy_empty_array(arr_data: list[list[float]] | list[list[int]]):
    arr = np.array(arr_data, dtype=np.float32)
    with pytest.raises(InvalidArgumentError, match="Cannot create matrix from empty list"):
        data.matrix(arr)


def test_matrix_numpy_slice_view_invalid():
    # Test that numpy array slices/views are not supported
    arr = np.array([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]], dtype=np.float32)
    sliced = arr[:2, :2]  # First 2 rows, first 2 columns
    with pytest.raises(TypeError, match="The given array is not contiguous"):
        data.matrix(sliced)


def test_matrix_numpy_ndim_0_invalid():
    # Test 0-dimensional array (scalar)
    arr = np.array(1.0, dtype=np.float32)
    with pytest.raises(InvalidArgumentError, match="Expected numpy array with ndim=1 or ndim=2"):
        data.matrix(arr)
