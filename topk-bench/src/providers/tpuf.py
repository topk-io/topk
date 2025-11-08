import os
import turbopuffer


client = turbopuffer.Turbopuffer(
    api_key=os.environ["TURBOPUFFER_API_KEY"],
    region=os.environ["TURBOPUFFER_REGION"],
)


def setup(namespace: str):
    upsert(
        namespace,
        [
            {
                "id": "__bootstrap__",
                "text": "Hello, world!",
                "dense_embedding": [0.1] * 768,
                "int_filter": 1,
                "keyword_filter": "Hello",
            }
        ],
    )

    delete_by_id(
        namespace,
        ids=["__bootstrap__"],
    )


def ping(namespace: str):
    client.namespace(namespace).query(
        rank_by=("vector", "ANN", [0.1] * 768),
        top_k=1,
    )


def query_by_id(namespace: str, id: str):
    result = client.namespace(namespace).query(
        rank_by=("id", "desc"),
        filters=("id", "Eq", id),
        top_k=1,
    )

    return [r.__dict__ for r in result.rows]


def delete_by_id(namespace: str, ids: list[str]):
    client.namespace(namespace).write(
        deletes=ids,
    )


def query(
    namespace: str,
    vector: list[float],
    top_k: int,
    int_filter: int | None,
    keyword_filter: str | None,
):
    filters = []
    if int_filter:
        filters.append(("int_filter", "Lte", int_filter))
    if keyword_filter:
        filters.append(("keyword_filter", "ContainsAllTokens", keyword_filter))

    result = client.namespace(namespace).query(
        rank_by=("vector", "ANN", vector),
        top_k=top_k,
        filters=None if len(filters) == 0 else ("And", tuple(filters)),
    )
    return [r.__dict__ for r in result.rows]


def upsert(namespace: str, docs: list[dict]):
    client.namespace(namespace).write(
        upsert_rows=[
            {
                "id": doc["id"],
                "text": doc["text"],
                "vector": doc["dense_embedding"],
                "int_filter": doc["int_filter"],
                "keyword_filter": doc["keyword_filter"],
            }
            for doc in docs
        ],
        distance_metric="cosine_distance",
        schema={
            "text": {"type": "string"},
            "int_filter": {"type": "int"},
            "keyword_filter": {"type": "string", "full_text_search": True},
        },
    )


def delete_collection(namespace: str):
    client.namespace(namespace).delete_all()


def list_collections():
    return [ns.id for ns in client.namespaces()]
