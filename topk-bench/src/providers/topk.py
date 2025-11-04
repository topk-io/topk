import os
from topk_sdk import Client
from topk_sdk.query import filter, field
from topk_sdk.error import CollectionNotFoundError, CollectionAlreadyExistsError
from topk_sdk.schema import text, f32_vector, vector_index


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
                "text": text(),
                "dense_embedding": f32_vector(dimension=768).index(
                    vector_index(metric="cosine")
                ),
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
