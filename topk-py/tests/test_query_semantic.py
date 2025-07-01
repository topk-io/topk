import pytest
from topk_sdk import error
from topk_sdk.query import field, fn, match, select
from topk_sdk.schema import semantic_index, text

from . import ProjectContext
from .utils import dataset


def test_semantic_index_schema(ctx: ProjectContext):
    collection = dataset.semantic.setup(ctx)

    for f in collection.schema:
        assert not f.startswith("_"), f"Schema contains reserved field: {field}"


def test_semantic_index_create_with_invalid_model(ctx: ProjectContext):
    schema = {
        "title": text()
        .required()
        .index(semantic_index(model="definitely-does-not-exist"))
    }

    with pytest.raises(error.SchemaValidationError):
        ctx.client.collections().create(ctx.scope("semantic"), schema)


def test_semantic_index_write_docs(ctx: ProjectContext):
    collection = dataset.semantic.setup(ctx)

    result = ctx.client.collection(collection.name).count()
    assert result == 10


def test_semantic_index_query(ctx: ProjectContext):
    collection = dataset.semantic.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(sim=fn.semantic_similarity("title", "dummy")).topk(
            field("sim"), 3, True
        )
    )

    assert len(result) == 3


def test_semantic_index_query_with_text_filter(ctx: ProjectContext):
    collection = dataset.semantic.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(sim=fn.semantic_similarity("title", "dummy"))
        .filter(match("love", "summary"))
        .topk(field("sim"), 3, True)
    )

    # order is not guaranteed, since we're using a "dummy" embedder
    assert {doc["_id"] for doc in result} == {"gatsby", "pride"}


def test_semantic_index_query_with_missing_index(ctx: ProjectContext):
    collection = dataset.semantic.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
            select(sim=fn.semantic_similarity("published_year", "dummy")).topk(
                field("sim"), 3, True
            )
        )


def test_semantic_index_query_multiple_fields(ctx: ProjectContext):
    collection = dataset.semantic.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            title_sim=fn.semantic_similarity("title", "dummy"),
            summary_sim=fn.semantic_similarity("summary", "query"),
        ).topk(field("title_sim") + field("summary_sim"), 5, True)
    )

    assert len(result) == 5


def test_semantic_index_query_and_rerank_with_missing_model(ctx: ProjectContext):
    collection = dataset.semantic.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
            select(sim=fn.semantic_similarity("title", "dummy"))
            .topk(field("sim"), 3, True)
            .rerank("definitely-does-not-exist")
        )


def test_semantic_index_query_and_rerank(ctx: ProjectContext):
    collection = dataset.semantic.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(sim=fn.semantic_similarity("title", "dummy"))
        .topk(field("sim"), 3, True)
        .rerank("dummy")
    )

    assert len(result) == 3


def test_semantic_index_query_and_rerank_multiple_semantic_sim_explicit(
    ctx: ProjectContext,
):
    collection = dataset.semantic.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            title_sim=fn.semantic_similarity("title", "dummy"),
            summary_sim=fn.semantic_similarity("summary", "query"),
        )
        .topk(field("title_sim") + field("summary_sim"), 5, True)
        .rerank("dummy", "query string", ["title", "summary"])
    )

    assert len(result) == 5


def test_semantic_index_query_and_rerank_multiple_semantic_sim_implicit(
    ctx: ProjectContext,
):
    collection = dataset.semantic.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
            select(
                title_sim=fn.semantic_similarity("title", "dummy"),
                summary_sim=fn.semantic_similarity("summary", "query"),
            )
            .topk(field("title_sim") + field("summary_sim"), 5, True)
            .rerank("dummy")
        )
