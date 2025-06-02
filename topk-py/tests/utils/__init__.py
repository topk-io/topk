from typing import Any


def doc_ids(result: list[dict[str, Any]]) -> set[str]:
    return {doc["_id"] for doc in result}
