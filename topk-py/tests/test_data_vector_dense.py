import pytest
from topk_sdk.data import binary_vector, f8_vector, f16_vector, f32_vector, u8_vector, i8_vector

TYPE_ERROR = "Invalid vector value"
TYPE_ERROR_INT_TO_VECTOR = "argument 'vector': 'int' object cannot be cast as 'Sequence'"
TYPE_ERROR_OUT_OF_RANGE = "out of range integral type conversion attempted"
TYPE_ERROR_NONE_TO_VECTOR = "argument 'vector': 'NoneType' object cannot be cast as 'Sequence'"
TYPE_ERROR_BOOL_TO_VECTOR = "argument 'vector': 'bool' object cannot be cast as 'Sequence'"
TYPE_ERROR_FLOAT_TO_VECTOR = "argument 'vector': 'float' object cannot be cast as 'Sequence'"
TYPE_ERROR_DICT_TO_VECTOR = "argument 'vector': 'dict' object cannot be cast as 'Sequence'"


class TestF8Vector:
    def test_valid(self):
        f8_vector([0.1, 0.2, 0.3])

    def test_empty_case(self):
        f8_vector([])

    def test_to_string(self):
        # F8 has limited precision, so we check the format rather than exact values
        result = str(f8_vector([1.0, 2.0, 3.0]))
        assert result.startswith("List(F8(")

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR_INT_TO_VECTOR):
            f8_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_NONE_TO_VECTOR):
            f8_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_BOOL_TO_VECTOR):
            f8_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            f8_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            f8_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            f8_vector(float("-inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            f8_vector({1: 256})  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            f8_vector({1: -1})  # type: ignore


class TestF16Vector:
    def test_valid(self):
        f16_vector([0.1, 0.2, 0.3])

    def test_empty_case(self):
        f16_vector([])

    def test_to_string(self):
        # F16 has limited precision, so we check the format rather than exact values
        result = str(f16_vector([1.0, 2.0, 3.0]))
        assert result.startswith("List(F16(")

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR_INT_TO_VECTOR):
            f16_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_NONE_TO_VECTOR):
            f16_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_BOOL_TO_VECTOR):
            f16_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            f16_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            f16_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            f16_vector(float("-inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            f16_vector({1: 256})  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            f16_vector({1: -1})  # type: ignore


class TestF32Vector:
    def test_valid(self):
        f32_vector([1, 2, 3])

    def test_empty_case(self):
        f32_vector([])

    def test_to_string(self):
        assert str(f32_vector([1, 2, 3])) == "List(F32([1.0, 2.0, 3.0]))"

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR_INT_TO_VECTOR):
            f32_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_NONE_TO_VECTOR):
            f32_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_BOOL_TO_VECTOR):
            f32_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            f32_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            f32_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            f32_vector(float("-inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            f32_vector({1: 256})  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            f32_vector({1: -1})  # type: ignore


class TestU8Vector:
    def test_valid(self):
        u8_vector([1, 2, 3])

    def test_empty_case(self):
        u8_vector([])

    def test_to_string(self):
        assert str(u8_vector([1, 2, 3])) == "List(U8([1, 2, 3]))"

    def test_invalid_number_range(self):
        with pytest.raises(OverflowError, match=TYPE_ERROR_OUT_OF_RANGE):
            u8_vector([256])  # type: ignore
        with pytest.raises(OverflowError, match=TYPE_ERROR_OUT_OF_RANGE):
            u8_vector([-1])  # type: ignore

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR_INT_TO_VECTOR):
            u8_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_NONE_TO_VECTOR):
            u8_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_BOOL_TO_VECTOR):
            u8_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            u8_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            u8_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            u8_vector({1: 256})  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            u8_vector({1: -1})  # type: ignore


class TestI8Vector:
    def test_valid(self):
        i8_vector([-128, 0, 127])

    def test_empty_case(self):
        i8_vector([])

    def test_to_string(self):
        assert str(i8_vector([-128, 0, 127])) == "List(I8([-128, 0, 127]))"

    def test_invalid_number_range(self):
        with pytest.raises(OverflowError, match=TYPE_ERROR_OUT_OF_RANGE):
            i8_vector([-129])  # type: ignore
        with pytest.raises(OverflowError, match=TYPE_ERROR_OUT_OF_RANGE):
            i8_vector([128])  # type: ignore

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR_INT_TO_VECTOR):
            i8_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_NONE_TO_VECTOR):
            i8_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_BOOL_TO_VECTOR):
            i8_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            i8_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            i8_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            i8_vector({1: 256})  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            i8_vector({1: -1})  # type: ignore


class TestBinaryVector:
    def test_valid(self):
        binary_vector([1, 2, 3])

    def test_empty_case(self):
        binary_vector([])

    def test_to_string(self):
        assert str(binary_vector([1, 2, 3])) == "List(U8([1, 2, 3]))"

    def test_invalid_number_range(self):
        with pytest.raises(OverflowError, match=TYPE_ERROR_OUT_OF_RANGE):
            binary_vector([256])  # type: ignore
        with pytest.raises(OverflowError, match=TYPE_ERROR_OUT_OF_RANGE):
            binary_vector([-1])  # type: ignore

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR_INT_TO_VECTOR):
            binary_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_NONE_TO_VECTOR):
            binary_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_BOOL_TO_VECTOR):
            binary_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            binary_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            binary_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_FLOAT_TO_VECTOR):
            binary_vector(float("-inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            binary_vector({1: 256})  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR_DICT_TO_VECTOR):
            binary_vector({1: -1})  # type: ignore
