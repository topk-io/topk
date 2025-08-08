import pytest
from topk_sdk.data import f64_list, i32_list, i64_list, u32_list


def test_u32_list():
    result = u32_list([0, 1, 255, 4294967295])
    assert result is not None
    assert str(result) == "[0, 1, 255, 4294967295]"


def test_u32_list_empty():
    result = u32_list([])
    assert result is not None
    assert str(result) == "[]"


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
    assert str(result) == "[-2147483648, -1, 0, 1, 2147483647]"


def test_i32_list_empty():
    result = i32_list([])
    assert result is not None
    assert str(result) == "[]"


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
    assert str(result) == "[-9223372036854775808, -1, 0, 1, 9223372036854775807]"


def test_i64_list_empty():
    result = i64_list([])
    assert result is not None
    assert str(result) == "[]"


def test_i64_list_invalid_input():
    with pytest.raises(
        TypeError, match="Expected list\\[int\\] for i64_list\\(\\) function"
    ):
        i64_list("not a list")  # type: ignore

    with pytest.raises(
        TypeError, match="Expected list\\[int\\] for i64_list\\(\\) function"
    ):
        i64_list(123)  # type: ignore


def test_f64_list():
    result = f64_list([1.0, 2.5, -3.14, 0.0])
    assert result is not None
    assert str(result) == "[1.0, 2.5, -3.14, 0.0]"


def test_f64_list_empty():
    result = f64_list([])
    assert result is not None
    assert str(result) == "[]"


def test_f64_list_invalid_input():
    with pytest.raises(
        TypeError, match="Expected list\\[float\\] for f64_list\\(\\) function"
    ):
        f64_list("not a list")  # type: ignore

    with pytest.raises(
        TypeError, match="Expected list\\[float\\] for f64_list\\(\\) function"
    ):
        f64_list(123)  # type: ignore
