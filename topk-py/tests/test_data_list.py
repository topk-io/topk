import pytest
from topk_sdk.data import list_u32, list_i32, list_i64, list_f64

def test_list_u32():
    result = list_u32([0, 1, 255, 4294967295])
    assert result is not None
    assert str(result) == "[0, 1, 255, 4294967295]"


def test_list_u32_empty():
    result = list_u32([])
    assert result is not None
    assert str(result) == "[]"


def test_list_u32_invalid_input():
    with pytest.raises(TypeError, match="Expected list\\[int\\] for list\\(\\) function"):
        list_u32("not a list")  # type: ignore

    with pytest.raises(TypeError, match="Expected list\\[int\\] for list\\(\\) function"):
        list_u32(123)  # type: ignore


def test_list_i32():
    result = list_i32([-2147483648, -1, 0, 1, 2147483647])
    assert result is not None
    assert str(result) == "[-2147483648, -1, 0, 1, 2147483647]"


def test_list_i32_empty():
    result = list_i32([])
    assert result is not None
    assert str(result) == "[]"


def test_list_i32_invalid_input():
    with pytest.raises(TypeError, match="Expected list\\[int\\] for list\\(\\) function"):
        list_i32("not a list")  # type: ignore

    with pytest.raises(TypeError, match="Expected list\\[int\\] for list\\(\\) function"):
        list_i32(123)  # type: ignore


def test_list_i64():
    result = list_i64([-9223372036854775808, -1, 0, 1, 9223372036854775807])
    assert result is not None
    assert str(result) == "[-9223372036854775808, -1, 0, 1, 9223372036854775807]"


def test_list_i64_empty():
    result = list_i64([])
    assert result is not None
    assert str(result) == "[]"


def test_list_i64_invalid_input():
    with pytest.raises(TypeError, match="Expected list\\[int\\] for list\\(\\) function"):
        list_i64("not a list")  # type: ignore

    with pytest.raises(TypeError, match="Expected list\\[int\\] for list\\(\\) function"):
        list_i64(123)  # type: ignore


def test_list_f64():
    result = list_f64([1.0, 2.5, -3.14, 0.0])
    assert result is not None
    assert str(result) == "[1.0, 2.5, -3.14, 0.0]"


def test_list_f64_empty():
    result = list_f64([])
    assert result is not None
    assert str(result) == "[]"


def test_list_f64_invalid_input():
    with pytest.raises(TypeError, match="Expected list\\[int\\] for list\\(\\) function"):
        list_f64("not a list")  # type: ignore

    with pytest.raises(TypeError, match="Expected list\\[int\\] for list\\(\\) function"):
        list_f64(123)  # type: ignore

