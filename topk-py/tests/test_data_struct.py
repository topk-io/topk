import pytest
from topk_sdk.data import struct


def test_struct_plain_dict():
    result = struct({"key": "value"})
    assert result is not None
    assert "Struct" in str(type(result))


def test_struct_empty():
    result = struct({})
    assert result is not None


def test_struct_nested_dict():
    result = struct({"nested": {"key": "value"}})
    assert result is not None


def test_struct_keyword_arg():
    # Parameter is named `fields`, not `data` — keyword call must work.
    result = struct(fields={"key": "value"})
    assert result is not None


def test_struct_non_string_key_raises():
    # PyO3 rejects non-string keys when deserializing HashMap<String, Value>
    with pytest.raises(TypeError):
        struct({123: "value"})  # type: ignore


def test_struct_str():
    result = struct({"a": "b"})
    assert "Struct" in str(result)
