import os
import turbopuffer


client = turbopuffer.Turbopuffer(
    api_key=os.environ["TURBOPUFFER_API_KEY"],
    region=os.environ["TURBOPUFFER_REGION"],
)


def setup(namespace: str):
    # Turbopuffer namespaces are created automatically
    pass


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


def query(
    namespace: str,
    vector: list[float],
    top_k: int,
    num_filter: int | None,
    keyword_filter: str | None,
):
    filters = []
    if num_filter:
        filters.append(("numerical_filter", "Lte", num_filter))
    if keyword_filter:
        filters.append(("categorical_filter", "ContainsAllTokens", keyword_filter))

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
                "numerical_filter": doc["numerical_filter"],
                "categorical_filter": doc["categorical_filter"],
            }
            for doc in docs
        ],
        distance_metric="cosine_distance",
        schema={
            "text": {"type": "string"},
            "numerical_filter": {"type": "int"},
            "categorical_filter": {"type": "string", "full_text_search": True},
        },
    )
