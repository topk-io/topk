import pytest
from topk_sdk.query import field, fn, match, select
from topk_sdk.schema import keyword_index, text, vector, vector_index

from . import ProjectContext


def test_upsert(ctx: ProjectContext):
    ctx.client.collections().create(ctx.scope("books"), schema={})

    lsn = ctx.client.collection(ctx.scope("books")).upsert(
        [
            {"_id": "one", "name": "one"},
            {"_id": "two", "name": "two", "extra": 1},
        ],
    )
    assert lsn == 1

    docs = ctx.client.collection(ctx.scope("books")).query(
        select("name").filter(field("name").eq("two")),
        lsn=lsn,
    )

    assert docs == [{"_id": "two", "name": "two"}]


def test_keyword_search(ctx: ProjectContext):
    ctx.client.collections().create(
        ctx.scope("books"),
        schema={
            "title": text().required().index(keyword_index()),
        },
    )

    lsn = ctx.client.collection(ctx.scope("books")).upsert(
        [
            {"_id": "doc1", "title": "red purple green"},
            {"_id": "doc2", "title": "yellow purple pink"},
            {"_id": "doc3", "title": "orange red blue"},
            {"_id": "doc4", "title": "green yellow purple"},
            {"_id": "doc5", "title": "pink orange red"},
            {"_id": "doc6", "title": "black green yellow"},
            {"_id": "doc7", "title": "purple pink orange"},
            {"_id": "doc8", "title": "red yello green"},
            {"_id": "doc9", "title": "yellow purple pink"},
            {"_id": "doc10", "title": "orange red blue"},
        ],
    )
    assert lsn == 1

    docs = ctx.client.collection(ctx.scope("books")).query(
        select(
            text_score=fn.keyword_score(),
        )
        .filter(
            match("title", token="blue", weight=10.0)
            | match("title", token="red", weight=50.0),
        )
        .top_k(field("text_score"), k=5),
        lsn=lsn,
    )

    assert {d["_id"] for d in docs} == {"doc1", "doc10", "doc3", "doc5", "doc8"}


def test_vector_search(ctx: ProjectContext):
    ctx.client.collections().create(
        ctx.scope("books"),
        schema={
            "embedding": vector(3).required().index(vector_index(metric="cosine")),
        },
    )

    lsn = ctx.client.collection(ctx.scope("books")).upsert(
        [
            {"_id": "doc1", "embedding": [1.0, 2.0, 3.0]},
            {"_id": "doc2", "embedding": [4.0, 5.0, 6.0]},
            {"_id": "doc3", "embedding": [7.0, 8.0, 9.0]},
        ],
    )
    assert lsn == 1

    docs = ctx.client.collection(ctx.scope("books")).query(
        select(
            vector_distance=fn.vector_distance("embedding", [1.0, 2.0, 3.0]),
        ).top_k(field("vector_distance"), k=5, asc=True),
        lsn=lsn,
    )
    docs.sort(key=lambda d: d["vector_distance"])

    assert [d["_id"] for d in docs] == ["doc3", "doc2", "doc1"]


def test_delete(ctx: ProjectContext):
    ctx.client.collections().create(ctx.scope("books"), schema={})

    lsn = ctx.client.collection(ctx.scope("books")).upsert(
        [
            {"_id": "doc1", "name": "one"},
        ],
    )
    assert lsn == 1

    lsn = ctx.client.collection(ctx.scope("books")).delete(["doc1"])
    assert lsn == 2

    docs = ctx.client.collection(ctx.scope("books")).query(
        select("name").filter(field("name").eq("one")),
        lsn=lsn,
    )
    assert docs == []


@pytest.fixture
def ctx():
    from . import new_project_context

    return new_project_context()
