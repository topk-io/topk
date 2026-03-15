"""Convert SDK response objects to plain Python types."""

from topk_cli.pyo3 import is_pyo3, obj_to_dict


def to_plain(obj):
    """Recursively convert SDK response objects to plain Python types."""
    if obj is None or isinstance(obj, (str, int, float, bool)):
        return obj
    if isinstance(obj, dict):
        return {k: to_plain(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [to_plain(v) for v in obj]
    if is_pyo3(obj):
        return obj_to_dict(obj, to_plain)
    return obj
