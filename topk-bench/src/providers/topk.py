import os
from topk_sdk import Client
from topk_sdk import schema
from topk_sdk.query import filter, field, select, fn
from topk_sdk.error import CollectionAlreadyExistsError


client = Client(
    api_key=os.environ["TOPK_API_KEY"],
    region=os.environ["TOPK_REGION"],
    host=os.environ.get("TOPK_HOST", "topk.io"),
    https=bool(os.environ.get("TOPK_HTTPS", "1") == "1"),
)


def setup(collection: str):
    try:
        client.collections().create(
            collection,
            schema={
                "text": schema.text().required(),
                "dense_embedding": schema.f32_vector(dimension=768).index(
                    schema.vector_index(metric="cosine")
                ),
                "int_filter": schema.int().required(),
                "keyword_filter": schema.text()
                .required()
                .index(schema.keyword_index()),
            },
        )
    except CollectionAlreadyExistsError:
        pass
    except Exception as e:
        raise e


def ping(collection: str):
    client.collection(collection).query(select().limit(1))


def upsert(collection: str, docs: list[dict]):
    client.collection(collection).upsert(
        [
            {
                "_id": doc["id"],
                "text": doc["text"],
                "dense_embedding": doc["dense_embedding"],
                "int_filter": doc["int_filter"],
                "keyword_filter": doc["keyword_filter"],
            }
            for doc in docs
        ]
    )


def query_by_id(collection: str, id: str):
    return client.collection(collection).query(filter(field("_id").eq(id)).limit(1))


def delete_by_id(collection: str, ids: list[str]):
    client.collection(collection).delete(ids)


def query(
    collection: str,
    vector: list[float],
    top_k: int,
    int_filter: int | None,
    keyword_filter: str | None,
):
    query = select(
        "text",
        "int_filter",
        "keyword_filter",
        vector_distance=fn.vector_distance("dense_embedding", vector),
    ).topk(field("vector_distance"), top_k)

    if int_filter:
        query = query.filter(field("int_filter").lte(int_filter))
    if keyword_filter:
        query = query.filter(field("keyword_filter").match_all(keyword_filter))

    return client.collection(collection).query(
        query,
        # consistency=ConsistencyLevel.STRONG
    )


def delete_collection(collection: str):
    client.collections().delete(collection)


def list_collections():
    return [collection.name for collection in client.collections().list()]


def close():
    pass
