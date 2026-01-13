import pytest
import numpy as np
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


# Matrix creation with different value types
def test_matrix_f16_list_of_lists():
    result = data.matrix([[1.0, 2.0], [3.0, 4.0]], "f16")
    assert result is not None
    # f16 values will be converted, check that it's F16 variant
    assert "F16" in str(result)
    assert "Matrix(2," in str(result)


def test_matrix_f8_list_of_lists():
    result = data.matrix([[1.0, 2.0], [3.0, 4.0]], "f8")
    assert result is not None
    # f8 values will be converted, check that it's F8 variant

    assert "F8" in str(result)
    assert "Matrix(2," in str(result)


def test_matrix_u8_list_of_lists():
    result = data.matrix([[0, 1, 255], [128, 64, 32]], "u8")
    assert result is not None
    assert "U8" in str(result)
    assert "Matrix(3," in str(result)


def test_matrix_u8_with_floats_invalid():
    # u8 should not accept floats - should raise TypeError
    with pytest.raises(TypeError, match="'float' object cannot be interpreted as an integer"):
        data.matrix([[0.0, 1.5, 255.0], [128.0, 64.0, 32.0]], "u8")


def test_matrix_i8_list_of_lists():
    result = data.matrix([[-128, -1, 0], [1, 127, 64]], "i8")
    assert result is not None
    assert "I8" in str(result)
    assert "Matrix(3," in str(result)


def test_matrix_i8_with_floats_invalid():
    # i8 should not accept floats - should raise TypeError
    with pytest.raises(TypeError, match="'float' object cannot be interpreted as an integer"):
        data.matrix([[-128.0, -1.0, 0.0], [1.0, 127.0, 64.0]], "i8")


# Numpy array tests
def test_matrix_numpy_1d():
    arr = np.array([1.0, 2.0, 3.0], dtype=np.float32)
    result = data.matrix(arr)
    assert result is not None
    assert str(result) == "Matrix(3, F32([1.0, 2.0, 3.0]))"


def test_matrix_numpy_2d():
    arr = np.array([[1.0, 2.0], [3.0, 4.0]], dtype=np.float32)
    result = data.matrix(arr)
    assert result is not None
    assert str(result) == "Matrix(2, F32([1.0, 2.0, 3.0, 4.0]))"


def test_matrix_numpy_float32():
    arr = np.array([[1.0, 2.0], [3.0, 4.0]], dtype=np.float32)
    result = data.matrix(arr)
    assert result is not None
    assert "F32" in str(result)
    assert "Matrix(2," in str(result)


def test_matrix_numpy_float16():
    arr = np.array([[1.0, 2.0], [3.0, 4.0]], dtype=np.float16)
    result = data.matrix(arr)
    assert result is not None
    assert "F16" in str(result)
    assert "Matrix(2," in str(result)


def test_matrix_numpy_uint8():
    arr = np.array([[0, 1, 255], [128, 64, 32]], dtype=np.uint8)
    result = data.matrix(arr)
    assert result is not None
    assert "U8" in str(result)
    assert "Matrix(3," in str(result)


def test_matrix_numpy_int8():
    arr = np.array([[-128, -1, 0], [1, 127, 64]], dtype=np.int8)
    result = data.matrix(arr)
    assert result is not None
    assert "I8" in str(result)
    assert "Matrix(3," in str(result)


def test_matrix_numpy_3d_invalid():
    arr = np.array([[[1.0, 2.0], [3.0, 4.0]], [[5.0, 6.0], [7.0, 8.0]]], dtype=np.float32)
    with pytest.raises(InvalidArgumentError, match="Expected numpy array with ndim=1 or ndim=2"):
        data.matrix(arr)


# Edge cases
def test_matrix_empty_list():
    with pytest.raises(InvalidArgumentError, match="Cannot create matrix from empty list"):
        data.matrix([])


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
    with pytest.raises(Exception):  # Should raise TypeError or similar
        data.matrix("not a list or array")  # type: ignore


def test_matrix_invalid_numpy_dtype():
    arr = np.array([[1.0, 2.0], [3.0, 4.0]], dtype=np.float64)
    with pytest.raises(Exception, match="Unsupported numpy dtype"):
        data.matrix(arr)


def test_matrix_invalid_numpy_dtype_int32():
    arr = np.array([[1, 2], [3, 4]], dtype=np.int32)
    with pytest.raises(Exception, match="Unsupported numpy dtype"):
        data.matrix(arr)
