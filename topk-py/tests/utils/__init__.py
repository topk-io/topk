from typing import Any


def doc_ids(result: list[dict[str, Any]]) -> set[str]:
    return {doc["_id"] for doc in result}

def doc_ids_ordered(result: list[dict[str, Any]]) -> list[str]:
    return [doc["_id"] for doc in result]

def is_sorted(result: list[dict[str, Any]], field_name: str) -> bool:
    values = [doc[field_name] for doc in result]
    return all(values[i] <= values[i + 1] for i in range(len(values) - 1))
