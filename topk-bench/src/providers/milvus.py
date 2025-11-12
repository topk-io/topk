import os

from pymilvus import DataType, MilvusClient

client = MilvusClient(
    uri=os.environ["MILVUS_URI"],
    token=os.environ["MILVUS_TOKEN"],
)


def setup(collection_name: str):
    schema = MilvusClient.create_schema(
        enable_dynamic_field=False,
    )

    schema.add_field(
        "id",
        DataType.VARCHAR,
        max_length=256,
        is_primary=True,
    )
    schema.add_field(
        "text",
        DataType.VARCHAR,
        max_length=4096,
        enable_analyzer=True,
        enable_match=True,
    )
    schema.add_field(
        "dense_embedding",
        DataType.FLOAT_VECTOR,
        dim=768,
        enable_index=True,
    )
    schema.add_field(
        "int_filter",
        DataType.INT64,
        enable_index=True,
    )
    schema.add_field(
        "keyword_filter",
        DataType.VARCHAR,
        enable_analyzer=True,
        enable_match=True,
        max_length=256,
    )

    client.create_collection(
        collection_name=collection_name,
        schema=schema,
    )

    index_params = client.prepare_index_params()
    index_params.add_index(
        field_name="dense_embedding",
        metric_type="COSINE",
        index_type="IVF_FLAT",
        param={"nlist": 1024},
    )
    index_params.add_index(
        field_name="keyword_filter",
        index_params={"index_type": "INVERTED"},
    )

    client.create_collection(
        collection_name=collection_name,
        schema=schema,
        index_params=index_params,
    )

    client.load_collection(collection_name)


def ping(collection_name: str):
    result = client.search(
        collection_name=collection_name,
        filter="int_filter == 1 and TEXT_MATCH(keyword_filter, 'sample')",
        data=[[0.1] * 768],
        output_fields=["int_filter", "keyword_filter", "text"],
    )

    return _convert_result(result)


def upsert(collection_name: str, docs: list[dict]):
    client.supert(
        collection_name=collection_name,
        data=docs,
    )


def query_by_id(collection_name: str, id: str):
    result = client.search(
        collection_name=collection_name,
        filter=f'id == "{id}"',
        output_fields=["text", "int_filter", "keyword_filter"],
    )
    return _convert_result(result)


def delete_by_id(collection_name: str, ids: list[str]):
    client.delete(collection_name=collection_name, filter=f"id in {ids}")


def query(
    collection_name: str,
    vector: list[float],
    top_k: int,
    int_filter: int | None = None,
    keyword_filter: str | None = None,
):
    filters = []

    if int_filter:
        filters.append(f"int_filter <= {int_filter}")
    if keyword_filter:
        filters.append(f"TEXT_MATCH(keyword_filter, '{keyword_filter}')")

    results = client.search(
        collection_name=collection_name,
        data=[vector],
        anns_field="dense_embedding",
        param={"metric_type": "COSINE"},
        limit=top_k,
        filter=" and ".join(filters),
        output_fields=["text", "int_filter", "keyword_filter"],
    )
    return _convert_result(results)


def delete_collection(collection_name: str):
    client.delete_collection(collection_name=collection_name)


def list_collections():
    return client.list_collections()


def close():
    pass


def _convert_result(result: list[dict]) -> list[dict]:
    out = []
    for hits in result:
        for hit in hits:
            out.append(
                {
                    "id": hit["id"],
                    "text": hit["entity"]["text"],
                    "int_filter": hit["entity"]["int_filter"],
                    "keyword_filter": hit["entity"]["keyword_filter"],
                }
            )
    return out
