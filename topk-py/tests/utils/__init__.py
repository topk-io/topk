from typing import Any


def doc_ids(result: list[dict[str, Any]]) -> set[str]:
    return {doc["_id"] for doc in result}


def doc_ids_ordered(result: list[dict[str, Any]]) -> list[str]:
    return [doc["_id"] for doc in result]
