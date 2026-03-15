"""Workarounds for PyO3 SDK objects.

PyO3 objects report __module__ == "builtins" and don't support dict(),
json.dumps(), etc. These hacks introspect them via dir() to extract
plain dicts.

TODO: Replace with proper .to_dict() / serde in topk-py and remove this module.
"""

from typing import Any


def is_pyo3(obj) -> bool:
    """Check if obj is a PyO3 object (not a regular Python builtin)."""
    return obj.__class__.__module__ == "builtins" and obj.__class__.__qualname__ not in (
        "dict", "list", "tuple", "set", "frozenset",
        "str", "int", "float", "bool", "bytes", "NoneType",
    )


# SDK metadata keys that should not appear in output
_SKIP_KEYS = {"request_id"}


def obj_to_dict(obj, convert) -> Any:
    """Convert a PyO3 object to a dict by introspecting public attributes.

    If the object has exactly one non-metadata attribute, return its value
    directly (unwrap). This handles SDK response wrappers like
    ListDatasetsResponse { datasets: [...], request_id: "..." }.
    """
    attrs = {}
    for k in dir(obj):
        if k.startswith("_") or k in _SKIP_KEYS:
            continue
        v = getattr(obj, k)
        if callable(v):
            continue
        attrs[k] = convert(v)
    if not attrs:
        return str(obj)
    if len(attrs) == 1:
        return next(iter(attrs.values()))
    return attrs
