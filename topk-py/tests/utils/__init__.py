# Utils package for tests


def doc_ids(result):
    return {doc["_id"] for doc in result}
