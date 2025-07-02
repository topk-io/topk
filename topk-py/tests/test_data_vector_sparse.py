import pytest
from topk_sdk.data import f32_sparse_vector, u8_sparse_vector

TYPE_ERROR = "Invalid sparse vector"


class TestF32SparseVector:
    def test_valid(self):
        f32_sparse_vector({1: 1.1})
        f32_sparse_vector({1: 1})

    def test_empty_case(self):
        f32_sparse_vector({})

    def test_to_string(self):
        assert str(f32_sparse_vector({1: 1.1})) == "SparseVector(F32([1], [1.1]))"

    def test_invalid_key(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_sparse_vector({"foo": 1})  # type: ignore

    def test_invalid_value(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_sparse_vector({1: "foo"})  # type: ignore

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_sparse_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_sparse_vector([])  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_sparse_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_sparse_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_sparse_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_sparse_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            f32_sparse_vector(float("-inf"))  # type: ignore


class TestU8SparseVector:
    def test_valid(self):
        u8_sparse_vector({1: 1})
        u8_sparse_vector({1: 1, 2: 2})

    def test_empty_case(self):
        u8_sparse_vector({})

    def test_to_string(self):
        assert str(u8_sparse_vector({1: 1})) == "SparseVector(U8([1], [1]))"

    def test_invalid_key(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector({"foo": 1})  # type: ignore

    def test_invalid_value(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector({1: "foo"})  # type: ignore

    def test_invalid_number_range(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector({1: 256})  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector({1: -1})  # type: ignore

    def test_invalid_arguments(self):
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector(0)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector([])  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector(float("-inf"))  # type: ignore
