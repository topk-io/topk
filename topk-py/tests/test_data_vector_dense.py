import pytest
from topk_sdk.data import binary_vector, f32_vector, u8_vector

TYPE_ERROR = "Invalid vector value"


class TestF32Vector:
    def test_valid(self):
        f32_vector([1, 2, 3])

    def test_empty_case(self):
        f32_vector([])

    def test_to_string(self):
        assert str(f32_vector([1, 2, 3])) == "Vector(F32([1.0, 2.0, 3.0]))"

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_vector(float("-inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_vector({1: 256})  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_vector({1: -1})  # type: ignore


class TestU8Vector:
    def test_valid(self):
        u8_vector([1, 2, 3])

    def test_empty_case(self):
        u8_vector([])

    def test_to_string(self):
        assert str(u8_vector([1, 2, 3])) == "Vector(U8([1, 2, 3]))"

    def test_invalid_number_range(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_vector([256])  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_vector([-1])  # type: ignore

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_vector({1: 256})  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_vector({1: -1})  # type: ignore


class TestBinaryVector:
    def test_valid(self):
        binary_vector([1, 2, 3])

    def test_empty_case(self):
        binary_vector([])

    def test_to_string(self):
        assert str(binary_vector([1, 2, 3])) == "Vector(U8([1, 2, 3]))"

    def test_invalid_number_range(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            binary_vector([256])  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            binary_vector([-1])  # type: ignore

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            binary_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            binary_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            binary_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            binary_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            binary_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            binary_vector(float("-inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            binary_vector({1: 256})  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            binary_vector({1: -1})  # type: ignore
