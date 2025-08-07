import pytest
from topk_sdk.data import bytes


def test_bytes_from_list():
    result = bytes([0, 1, 255, 128])
    assert result is not None
    assert str(result) == str(b'\x00\x01\xff\x80')


def test_bytes_from_bytes():
    result = bytes(b"\x00\x01\xff\x80")
    assert result is not None
    assert str(result) == str(b'\x00\x01\xff\x80')


def test_bytes_invalid_input():
    with pytest.raises(TypeError, match="Expected bytes or list\\[int\\]"):
        bytes("not a valid input")  # type: ignore

    with pytest.raises(TypeError, match="Expected bytes or list\\[int\\]"):
        bytes(123)  # type: ignore


def test_bytes_empty():
    result1 = bytes([])
    result2 = bytes(b"")
    assert result1 is not None
    assert result2 is not None
    assert str(result1) == "b''"
    assert str(result2) == "b''"


def test_bytes_with_invalid_list_values():
    with pytest.raises(
        TypeError,
        match="Expected list\\[int\\] with values in range \\[0, 255\\]",
    ):
        bytes([0, 1, "invalid", 3])  # type: ignore

    with pytest.raises(
        TypeError,
        match="Expected list\\[int\\] with values in range \\[0, 255\\]",
    ):
        bytes([0, 1, 256, 3])

    with pytest.raises(
        TypeError,
        match="Expected list\\[int\\] with values in range \\[0, 255\\]",
    ):
        bytes([0, -1, 2, 3])
