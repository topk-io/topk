import pytest
from topk_sdk.query import field, fn, match, select
from topk_sdk.schema import keyword_index, text, f32_vector, u8_vector, binary_vector, vector_index

from . import ProjectContext


def test_upsert(ctx: ProjectContext):
    ctx.client.collections().create(ctx.scope("books"), schema={})

    lsn = ctx.client.collection(ctx.scope("books")).upsert(
        [
            {"_id": "one", "name": "one", "rank": 1},
            {"_id": "two", "name": "two", "extra": 1, "rank": 2},
        ],
    )
    assert lsn == 1

    docs = ctx.client.collection(ctx.scope("books")).query(
        select("name")
        .filter(field("name").eq("two"))
        .top_k(field("rank"), k=10),
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
        .filter(match("red") | match("blue"))
        .top_k(field("text_score"), k=5),
        lsn=lsn,
    )

    assert {d["_id"] for d in docs} == {"doc1", "doc10", "doc3", "doc5", "doc8"}


def test_vector_search_f32(ctx: ProjectContext):
    ctx.client.collections().create(
        ctx.scope("books"),
        schema={
            "f32_embedding": f32_vector(3).required().index(vector_index(metric="euclidean")),
        },
    )

    lsn = ctx.client.collection(ctx.scope("books")).upsert(
        [
            {"_id": "doc1", "f32_embedding": [1.0, 2.0, 3.0]},
            {"_id": "doc2", "f32_embedding": [4.0, 5.0, 6.0]},
            {"_id": "doc3", "f32_embedding": [7.0, 8.0, 9.0]},
        ],
    )
    assert lsn == 1

    docs = ctx.client.collection(ctx.scope("books")).query(
        select(
            vector_distance=fn.vector_distance("f32_embedding", [7.0, 8.0, 9.0]),
        ).top_k(field("vector_distance"), k=2, asc=True),
        lsn=lsn,
    )
    docs.sort(key=lambda d: d["vector_distance"])

    assert [d["_id"] for d in docs] == ["doc3", "doc2"]

def test_vector_search_u8(ctx: ProjectContext):
    ctx.client.collections().create(
        ctx.scope("books"),
        schema={
            "u8_embedding": u8_vector(3).required().index(vector_index(metric="euclidean")),
        },
    )

    lsn = ctx.client.collection(ctx.scope("books")).upsert(
        [
            {"_id": "doc1", "u8_embedding": [1, 2, 3]},
            {"_id": "doc2", "u8_embedding": [4, 5, 6]},
            {"_id": "doc3", "u8_embedding": [7, 8, 9]},
        ],
    )
    assert lsn == 1

    docs = ctx.client.collection(ctx.scope("books")).query(
        select(
            vector_distance=fn.vector_distance("u8_embedding", [7, 8, 9]),
        ).top_k(field("vector_distance"), k=2, asc=True),
        lsn=lsn,
    )
    docs.sort(key=lambda d: d["vector_distance"])

    assert [d["_id"] for d in docs] == ["doc3", "doc2"]

def test_vector_search_binary(ctx: ProjectContext):
    ctx.client.collections().create(
        ctx.scope("books"),
        schema={
            "binary_embedding": binary_vector(3).required().index(vector_index(metric="hamming")),
        },
    )

    lsn = ctx.client.collection(ctx.scope("books")).upsert(
        [
            {"_id": "doc1", "binary_embedding": [0, 0, 1]},
            {"_id": "doc2", "binary_embedding": [0, 1, 1]},
            {"_id": "doc3", "binary_embedding": [1, 1, 1]},
        ],
    )
    assert lsn == 1

    docs = ctx.client.collection(ctx.scope("books")).query(
        select(
            vector_distance=fn.vector_distance("binary_embedding", [1, 1, 1]),
        ).top_k(field("vector_distance"), k=2, asc=True),
        lsn=lsn,
    )
    docs.sort(key=lambda d: d["vector_distance"])

    assert [d["_id"] for d in docs] == ["doc3", "doc2"]


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
        select("name").filter(field("name").eq("one")).count(),
        lsn=lsn,
    )
    assert docs == [{"_count": 0}]

def test_count(ctx: ProjectContext):
    ctx.client.collections().create(ctx.scope("books"), schema={})

    lsn = ctx.client.collection(ctx.scope("books")).upsert(
        [
            {"_id": "doc1", "name": "one"},
            {"_id": "doc2", "name": "two"},
        ],
    )
    assert lsn == 1

    count = ctx.client.collection(ctx.scope("books")).count(lsn=lsn)
    assert count == 2

    lsn = ctx.client.collection(ctx.scope("books")).delete(["doc1"])
    assert lsn == 2

    count = ctx.client.collection(ctx.scope("books")).count(lsn=lsn)
    assert count == 1


@pytest.fixture
def ctx():
    from . import new_project_context

    return new_project_context()
