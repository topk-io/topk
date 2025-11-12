import os
from qdrant_client import QdrantClient, models

client = QdrantClient(
    url=os.environ["QDRANT_URL"],
    api_key=os.environ["QDRANT_API_KEY"],
)


def setup(collection: str):
    if collection not in [c.name for c in client.get_collections().collections]:
        client.create_collection(
            collection_name=collection,
            vectors_config=models.VectorParams(
                size=768,
                distance=models.Distance.COSINE,
            ),
        )

        client.create_payload_index(
            collection_name=collection,
            field_name="int_filter",
            field_schema=models.PayloadSchemaType.INTEGER,
        )

        client.create_payload_index(
            collection_name=collection,
            field_name="keyword_filter",
            field_schema=models.PayloadSchemaType.KEYWORD,
        )


def ping(collection: str):
    client.search(
        collection_name=collection,
        query_vector=[0.1] * 768,
        limit=1,
        with_payload=True,
    )


def query_by_id(collection: str, id: str):
    result = client.retrieve(
        collection_name=collection,
        ids=[int(id)],
        with_payload=True,
    )
    return _convert_results(result)


def delete_by_id(collection: str, ids: list[str]):
    client.delete(
        collection_name=collection,
        points_selector=models.PointIdsList(points=[int(id) for id in ids]),
    )


def query(
    collection: str,
    vector: list[float],
    top_k: int,
    int_filter: int | None,
    keyword_filter: str | None,
):
    # Build filter
    filters = []
    if int_filter is not None:
        filters.append(
            models.FieldCondition(
                key="int_filter",
                range=models.Range(lte=int_filter),
            )
        )
    if keyword_filter is not None:
        filters.append(
            models.FieldCondition(
                key="keyword_filter",
                match=models.MatchValue(value=keyword_filter),
            )
        )
    qfilter = None
    if filters:
        qfilter = models.Filter(must=filters)

    result = client.query_points(
        collection_name=collection,
        query=vector,
        limit=top_k,
        with_payload=True,
        query_filter=qfilter,
    )
    return _convert_results(result.points)


def upsert(collection: str, docs: list[dict]):
    client.upsert(
        collection_name=collection,
        points=[
            models.PointStruct(
                id=int(doc["id"]),
                vector=doc["dense_embedding"],
                payload={
                    "text": doc["text"],
                    "int_filter": doc["int_filter"],
                    "keyword_filter": doc["keyword_filter"].split(" "),
                },
            )
            for doc in docs
        ],
        wait=True,
    )


def delete_collection(collection: str):
    client.delete_collection(collection_name=collection)


def list_collections():
    return [c.name for c in client.get_collections().collections]


def _convert_results(results):
    out = []
    for m in results:
        payload = m.payload or {}
        out.append(
            {
                "id": str(m.id),
                "text": payload.get("text"),
                "int_filter": payload.get("int_filter"),
                "keyword_filter": payload.get("keyword_filter"),
            }
        )
    return out


def close():
    pass
