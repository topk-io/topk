"""Schema DSL parser for collection field specs."""

from typing import Literal

import typer
import topk_sdk.schema as _schema

# TODO: export Metric enum from topk-py instead of duplicating here
type Metric = Literal["cosine", "euclidean", "dot_product", "hamming"]

_VECTOR_TYPES = {
    "f32_vector", "f16_vector", "f8_vector",
    "u8_vector", "i8_vector", "binary_vector",
}


def parse_field(spec):
    """Parse a field spec (string or dict) into an SDK schema field.

    Compact string format:
      "text"                    -> text, no index
      "text:keyword"            -> text + keyword index
      "text:semantic"           -> text + semantic index (default model)
      "text:semantic:my-model"  -> text + semantic index, custom model
      "int" / "float" / "bool"  -> scalar fields
      "f32_vector:1536"         -> vector, no index
      "f32_vector:1536:cosine"  -> vector + cosine index

    Dict format:
      {"type": "text", "index": "semantic", "model": "...", "required": true}
      {"type": "f32_vector", "dim": 1536, "index": "vector", "metric": "cosine"}
    """
    if isinstance(spec, dict):
        field_type = spec["type"]
        index_type = spec.get("index")
        required = spec.get("required", False)
        metric: Metric = spec.get("metric", "cosine")
        dim = spec.get("dim") or spec.get("dimension")
        model = spec.get("model", "cohere/embed-english-v3.0")
    else:
        parts = str(spec).split(":")
        field_type, index_type, dim, required = parts[0], None, None, False
        metric: Metric = "cosine"
        model = "cohere/embed-english-v3.0"

        if field_type in _VECTOR_TYPES:
            if len(parts) > 1:
                dim = int(parts[1])
            if len(parts) > 2:
                index_type = "vector"
                metric = parts[2]  # type: ignore[assignment]
        else:
            if len(parts) > 1:
                index_type = parts[1]
            if index_type == "semantic" and len(parts) > 2:
                model = ":".join(parts[2:])

    schema_fn = getattr(_schema, field_type, None)
    if schema_fn is None:
        typer.echo(f"Error: Unknown field type '{field_type}'", err=True)
        raise typer.Exit(1)

    f = schema_fn(dim) if dim is not None else schema_fn()
    if required:
        f = f.required()
    if index_type == "keyword":
        f = f.index(_schema.keyword_index())
    elif index_type == "semantic":
        f = f.index(_schema.semantic_index(model))
    elif index_type == "vector":
        f = f.index(_schema.vector_index(metric))
    return f


def parse_schema(schema_json: str) -> dict:
    import json
    raw = json.loads(schema_json)
    return {name: parse_field(spec) for name, spec in raw.items()}
