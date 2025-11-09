import os
from traceback import print_tb
import chromadb


api_key = os.environ.get("CHROMA_API_KEY")
tenant = os.environ.get("CHROMA_TENANT")
database = os.environ.get("CHROMA_DATABASE")

client = chromadb.CloudClient(
    api_key=api_key,
    tenant=tenant,
    database=database,
)


CACHED_COLLECTIONS = {}


def get_collection(collection: str):
    if collection not in CACHED_COLLECTIONS:
        CACHED_COLLECTIONS[collection] = client.get_collection(name=collection)
    return CACHED_COLLECTIONS[collection]


def setup(collection: str):
    try:
        client.get_or_create_collection(
            name=collection,
            metadata={"hnsw:space": "cosine"},  # Use cosine similarity
        )
    except Exception as e:
        # If collection already exists, that's fine
        if "already exists" not in str(e).lower():
            raise e


def ping(collection: str):
    coll = get_collection(collection)
    coll.query(
        query_embeddings=[[0.0] * 768],
        n_results=1,
    )


def upsert(collection: str, docs: list[dict]):
    coll = get_collection(collection)

    coll.add(
        ids=[doc["id"] for doc in docs],
        documents=[doc["keyword_filter"] for doc in docs],
        embeddings=[doc["dense_embedding"] for doc in docs],
        metadatas=[
            {
                "int_filter": doc["int_filter"],
                "text": doc["text"],
            }
            for doc in docs
        ],
    )


def query_by_id(collection: str, id: str):
    coll = get_collection(collection)

    results = coll.get(ids=[id])

    return [
        {
            "id": results["ids"][i],
            "text": results["metadatas"][i]["text"],
            "int_filter": results["metadatas"][i]["int_filter"],
            "keyword_filter": results["documents"][i],
        }
        for i in range(len(results["ids"]))
    ]


def delete_by_id(collection: str, ids: list[str]):
    coll = get_collection(collection)
    coll.delete(ids=ids)


def query(
    collection: str,
    vector: list[float],
    top_k: int,
    int_filter: int | None,
    keyword_filter: str | None,
):
    coll = get_collection(collection)

    results = coll.query(
        query_embeddings=[vector],
        # Top K
        n_results=top_k,
        # Metadata filter
        where={"int_filter": {"$lte": int_filter}} if int_filter is not None else None,
        # Keyword filter
        where_document={"$contains": keyword_filter}
        if keyword_filter is not None
        else None,
    )

    return [
        {
            "id": results["ids"][0][i],
            "text": results["metadatas"][0][i]["text"],
            "int_filter": results["metadatas"][0][i]["int_filter"],
            "keyword_filter": results["documents"][0][i],
        }
        for i in range(len(results["ids"][0]))
    ]


def delete_collection(collection: str):
    client.delete_collection(name=collection)
    CACHED_COLLECTIONS.pop(collection, None)


def list_collections():
    return [coll.name for coll in client.list_collections()]


def close():
    CACHED_COLLECTIONS.clear()
