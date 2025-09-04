import pytest
from topk_sdk.data import f32_list, f64_list, i32_list, i64_list, string_list, u32_list


def test_u32_list():
    result = u32_list([0, 1, 255, 4294967295])
    assert result is not None
    assert str(result) == "List(U32([0, 1, 255, 4294967295]))"


def test_u32_list_empty():
    result = u32_list([])
    assert result is not None
    assert str(result) == "List(U32([]))"


def test_u32_list_invalid_input():
    with pytest.raises(
        TypeError, match="Expected list\\[int\\] for u32_list\\(\\) function"
    ):
        u32_list("not a list")  # type: ignore

    with pytest.raises(
        TypeError, match="Expected list\\[int\\] for u32_list\\(\\) function"
    ):
        u32_list(123)  # type: ignore


def test_i32_list():
    result = i32_list([-2147483648, -1, 0, 1, 2147483647])
    assert result is not None
    assert str(result) == "List(I32([-2147483648, -1, 0, 1, 2147483647]))"


def test_i32_list_empty():
    result = i32_list([])
    assert result is not None
    assert str(result) == "List(I32([]))"


def test_i32_list_invalid_input():
    with pytest.raises(
        TypeError, match="Expected list\\[int\\] for i32_list\\(\\) function"
    ):
        i32_list("not a list")  # type: ignore

    with pytest.raises(
        TypeError, match="Expected list\\[int\\] for i32_list\\(\\) function"
    ):
        i32_list(123)  # type: ignore


def test_i64_list():
    result = i64_list([-9223372036854775808, -1, 0, 1, 9223372036854775807])
    assert result is not None
    assert str(result) == "List(I64([-9223372036854775808, -1, 0, 1, 9223372036854775807]))"


def test_i64_list_empty():
    result = i64_list([])
    assert result is not None
    assert str(result) == "List(I64([]))"


def test_i64_list_invalid_input():
    with pytest.raises(
        TypeError, match="Expected list\\[int\\] for i64_list\\(\\) function"
    ):
        i64_list("not a list")  # type: ignore

    with pytest.raises(
        TypeError, match="Expected list\\[int\\] for i64_list\\(\\) function"
    ):
        i64_list(123)  # type: ignore


def test_f32_list():
    result = f32_list([1.0, 2.5, -3.5, 0.0])
    assert result is not None
    assert str(result) == "List(F32([1.0, 2.5, -3.5, 0.0]))"



def test_f32_list_empty():
    result = f32_list([])
    assert result is not None
    assert str(result) == "List(F32([]))"


def test_f32_list_invalid_input():
    with pytest.raises(
        TypeError, match="Expected list\\[float\\] for f32_list\\(\\) function"
    ):
        f32_list("not a list")  # type: ignore

    with pytest.raises(
        TypeError, match="Expected list\\[float\\] for f32_list\\(\\) function"
    ):
        f32_list(123)  # type: ignore


def test_f64_list():
    result = f64_list([1.0, 2.5, -3.14, 0.0])
    assert result is not None
    assert str(result) == "List(F64([1.0, 2.5, -3.14, 0.0]))"


def test_f64_list_empty():
    result = f64_list([])
    assert result is not None
    assert str(result) == "List(F64([]))"


def test_f64_list_invalid_input():
    with pytest.raises(
        TypeError, match="Expected list\\[float\\] for f64_list\\(\\) function"
    ):
        f64_list("not a list")  # type: ignore

    with pytest.raises(
        TypeError, match="Expected list\\[float\\] for f64_list\\(\\) function"
    ):
        f64_list(123)  # type: ignore


def test_string_list():
    result = string_list(["1", "2", "3"])
    assert result is not None
    assert str(result) == "List(String([\"1\", \"2\", \"3\"]))"


def test_string_list_empty():
    result = string_list([])
    assert result is not None
    assert str(result) == "List(String([]))"

def test_string_list_invalid_input():
    with pytest.raises(
        TypeError, match="Expected list\\[str\\] for string_list\\(\\) function"
    ):
        string_list("not a list")  # type: ignore

    with pytest.raises(
        TypeError, match="Expected list\\[str\\] for string_list\\(\\) function"
    ):
        string_list(123)  # type: ignore
