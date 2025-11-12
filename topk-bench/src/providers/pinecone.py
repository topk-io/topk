import os
from pinecone import Pinecone, QueryResponse, ServerlessSpec

pc = Pinecone(api_key=os.environ["PINECONE_API_KEY"])

__cache = {}


def _get_index(collection: str):
    if collection not in __cache:
        __cache[collection] = pc.Index(collection)
    return __cache[collection]


def setup(collection: str):
    if not pc.has_index(collection):
        pc.create_index(
            name=collection,
            dimension=768,
            metric="cosine",
            spec=ServerlessSpec(
                cloud="aws",
                region="us-east-1",
                # region="eu-central-1",
            ),
        )


def ping(collection: str):
    index = _get_index(collection)
    index.query(
        vector=[0.1] * 768,
        top_k=1,
        include_metadata=True,
    )


def query_by_id(collection: str, id: str):
    index = _get_index(collection)
    results = index.fetch(ids=[id])

    return _convert_results(results.vectors.values())


def delete_by_id(collection: str, ids: list[str]):
    index = _get_index(collection)
    index.delete(ids=ids)


def query(
    collection: str,
    vector: list[float],
    top_k: int,
    int_filter: int | None,
    keyword_filter: str | None,
):
    index = _get_index(collection)

    # Build filter
    filt = {}
    if int_filter is not None:
        filt["int_filter"] = {"$lte": int_filter}
    if keyword_filter is not None:
        filt["keyword_filter"] = {"$in": [keyword_filter]}

    results: QueryResponse = index.query(
        vector=vector,
        top_k=top_k,
        filter=None if not filt else filt,
        include_metadata=True,
    )

    return _convert_results(results["matches"])


def upsert(collection: str, docs: list[dict]):
    index = _get_index(collection)

    index.upsert(
        vectors=[
            (
                doc["id"],
                doc["dense_embedding"],
                {
                    "text": doc["text"],
                    "int_filter": doc["int_filter"],
                    "keyword_filter": doc["keyword_filter"].split(" "),
                },
            )
            for doc in docs
        ]
    )


def delete_collection(collection: str):
    if collection in pc.list_indexes():
        pc.delete_index(collection)


def list_collections():
    return [index.name for index in pc.list_indexes()]


def _convert_results(results: list[dict]):
    out = []
    for m in results:
        out.append(
            {
                "id": m.id,
                "text": m.metadata["text"],
                "int_filter": m.metadata["int_filter"],
                "keyword_filter": m.metadata["keyword_filter"],
            }
        )
    return out


def close():
    pass
