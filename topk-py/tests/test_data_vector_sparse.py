import pytest
from topk_sdk.data import f32_sparse_vector, u8_sparse_vector

TYPE_ERROR = "Invalid sparse vector"


class TestF32SparseVector:
    def test_valid(self):
        f32_sparse_vector({1: 1.1})
        f32_sparse_vector({1: 1})

    def test_empty_case(self):
        f32_sparse_vector({})
        
    def test_new_format_with_indices_values(self):
        # Test with indices and values as keyword arguments
        vec = f32_sparse_vector(indices=[1, 5, 10], values=[1.1, 2.2, 3.3])
        assert str(vec) == "SparseVector(F32([1, 5, 10], [1.1, 2.2, 3.3]))"
        
    def test_new_format_empty(self):
        # Test empty indices and values
        vec = f32_sparse_vector(indices=[], values=[])
        assert str(vec) == "SparseVector(F32([], []))"
        
    def test_new_format_single_element(self):
        vec = f32_sparse_vector(indices=[42], values=[3.14])
        assert str(vec) == "SparseVector(F32([42], [3.14]))"
        
    def test_new_format_mismatched_length(self):
        with pytest.raises(TypeError, match="indices and values must have the same length"):
            f32_sparse_vector(indices=[1, 2], values=[1.0])
            
    def test_new_format_unsorted_indices(self):
        with pytest.raises(TypeError, match="indices must be sorted"):
            f32_sparse_vector(indices=[5, 1, 10], values=[1.0, 2.0, 3.0])
            
    def test_new_format_duplicate_indices(self):
        with pytest.raises(TypeError, match="indices must be sorted"):
            f32_sparse_vector(indices=[1, 5, 5, 10], values=[1.0, 2.0, 3.0, 4.0])
            
    def test_dict_format_with_indices_values_keys(self):
        # Test that dict format with indices/values keys works
        vec = f32_sparse_vector({"indices": [1, 5, 10], "values": [1.1, 2.2, 3.3]})
        assert str(vec) == "SparseVector(F32([1, 5, 10], [1.1, 2.2, 3.3]))"

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
        with pytest.raises(TypeError, match="requires either a dict argument"):
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
        
    def test_new_format_with_indices_values(self):
        # Test with indices and values as keyword arguments
        vec = u8_sparse_vector(indices=[1, 5, 10], values=[10, 20, 30])
        assert str(vec) == "SparseVector(U8([1, 5, 10], [10, 20, 30]))"
        
    def test_new_format_empty(self):
        # Test empty indices and values
        vec = u8_sparse_vector(indices=[], values=[])
        assert str(vec) == "SparseVector(U8([], []))"
        
    def test_new_format_single_element(self):
        vec = u8_sparse_vector(indices=[42], values=[255])
        assert str(vec) == "SparseVector(U8([42], [255]))"
        
    def test_new_format_mismatched_length(self):
        with pytest.raises(TypeError, match="indices and values must have the same length"):
            u8_sparse_vector(indices=[1, 2], values=[10])
            
    def test_new_format_unsorted_indices(self):
        with pytest.raises(TypeError, match="indices must be sorted"):
            u8_sparse_vector(indices=[5, 1, 10], values=[10, 20, 30])
            
    def test_new_format_duplicate_indices(self):
        with pytest.raises(TypeError, match="indices must be sorted"):
            u8_sparse_vector(indices=[1, 5, 5, 10], values=[10, 20, 30, 40])
            
    def test_dict_format_with_indices_values_keys(self):
        # Test that dict format with indices/values keys works
        vec = u8_sparse_vector({"indices": [1, 5, 10], "values": [10, 20, 30]})
        assert str(vec) == "SparseVector(U8([1, 5, 10], [10, 20, 30]))"

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
        with pytest.raises(TypeError, match="requires either a dict argument"):
            u8_sparse_vector(None)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector(False)  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector(float("nan"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector(float("inf"))  # type: ignore
        with pytest.raises(TypeError, match=TYPE_ERROR):
            u8_sparse_vector(float("-inf"))  # type: ignore
