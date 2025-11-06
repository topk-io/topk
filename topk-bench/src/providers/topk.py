import os
from topk_sdk import Client, ConsistencyLevel
from topk_sdk.query import filter, field, select, fn
from topk_sdk.error import CollectionNotFoundError, CollectionAlreadyExistsError
from topk_sdk import schema


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
                "numerical_filter": schema.int().required(),
                "categorical_filter": schema.text()
                .required()
                .index(schema.keyword_index()),
            },
        )
    except CollectionAlreadyExistsError:
        pass
    except Exception as e:
        raise e


def ping():
    try:
        client.collection("non-existing-collection").get(["non-existing-id"])
        raise RuntimeError("get should have failed")
    except CollectionNotFoundError:
        pass
    except Exception as e:
        raise e


def upsert(collection: str, docs: list[dict]):
    client.collection(collection).upsert(
        [
            {
                "_id": doc["id"],
                "text": doc["text"],
                "dense_embedding": doc["dense_embedding"],
                "numerical_filter": doc["numerical_filter"],
                "categorical_filter": doc["categorical_filter"],
            }
            for doc in docs
        ]
    )


def query_by_id(collection: str, id: str):
    return client.collection(collection).query(filter(field("_id").eq(id)).limit(1))


def query(
    collection: str,
    vector: list[float],
    top_k: int,
    num_filter: int | None,
    keyword_filter: str | None,
):
    query = select(
        "text",
        vector_distance=fn.vector_distance("dense_embedding", vector),
    ).topk(field("vector_distance"), top_k)

    if num_filter:
        query = query.filter(field("numerical_filter").lte(num_filter))
    if keyword_filter:
        query = query.filter(field("categorical_filter").match_all(keyword_filter))

    return client.collection(collection).query(
        query,
        # consistency=ConsistencyLevel.STRONG
    )


def close():
    pass
