import os
import time
import chromadb


api_key = os.environ.get("CHROMA_API_KEY")
tenant = os.environ.get("CHROMA_TENANT")
database = os.environ.get("CHROMA_DATABASE")

client = chromadb.CloudClient(
    api_key=api_key,
    tenant=tenant,
    database=database,
)


def setup(collection: str):
    try:
        # Create collection with custom embedding function (we'll provide embeddings directly)
        # Chroma doesn't require schema definition like TopK, but we can specify distance metric
        client.get_or_create_collection(
            name=collection,
            metadata={"hnsw:space": "cosine"},  # Use cosine similarity
        )
    except Exception as e:
        # If collection already exists, that's fine
        if "already exists" not in str(e).lower():
            raise e


def ping(collection: str):
    # Simple query to test connectivity
    coll = client.get_collection(name=collection)
    coll.query(
        query_embeddings=[[0.0] * 768],
        n_results=1,
    )


def upsert(collection: str, docs: list[dict]):
    coll = client.get_collection(name=collection)

    ids = [doc["id"] for doc in docs]
    # NOTE: we swap `text` and `keyword_filter` because chroma
    # allows only one keyword index on the `documents` field.
    documents = [doc["keyword_filter"] for doc in docs]
    embeddings = [doc["dense_embedding"] for doc in docs]
    metadatas = [
        {
            "int_filter": doc["int_filter"],
            "text": doc["text"],
        }
        for doc in docs
    ]

    # Chroma's add() method will overwrite existing documents with the same ID,
    # effectively providing upsert behavior
    coll.add(
        ids=ids,
        documents=documents,
        embeddings=embeddings,
        metadatas=metadatas,
    )


def query_by_id(collection: str, id: str):
    coll = client.get_collection(name=collection)
    results = coll.get(ids=[id])

    return [
        {
            "id": results["ids"][i],
            "text": results["documents"][i],
            "int_filter": results["metadatas"][i]["int_filter"],
            "keyword_filter": results["metadatas"][i]["keyword_filter"],
        }
        for i in range(len(results["ids"]))
    ]


def delete_by_id(collection: str, ids: list[str]):
    coll = client.get_collection(name=collection)
    coll.delete(ids=ids)


CACHED_COLLECTIONS = {}


def get_collection(collection: str):
    if collection not in CACHED_COLLECTIONS:
        CACHED_COLLECTIONS[collection] = client.get_collection(name=collection)
    return CACHED_COLLECTIONS[collection]


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
            "text": results["documents"][0][i],
            "int_filter": results["metadatas"][0][i]["int_filter"],
            "keyword_filter": results["metadatas"][0][i]["keyword_filter"],
        }
        for i in range(len(results["ids"][0]))
    ]


def delete_collection(collection: str):
    try:
        client.delete_collection(name=collection)
    except Exception:
        # Collection might not exist, which is fine
        pass


def list_collections():
    return [coll.name for coll in client.list_collections()]


def close():
    # Chroma PersistentClient doesn't need explicit closing
    pass
