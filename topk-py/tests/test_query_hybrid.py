from topk_sdk.query import field, fn, select, match

from . import ProjectContext
from .utils import dataset, doc_ids_ordered, is_sorted


def test_query_hybrid_vector_bm25(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance("summary_embedding", [2.0] * 16),
            bm25_score=fn.bm25_score(),
        )
        .filter(
            match("love", None, 30.0, False) | (match("young", None, 10.0, False))
        )
        .topk(
            field("bm25_score") + (field("summary_distance") * 100), 2, True
        )
    )

    assert len(result) == 2
    assert doc_ids_ordered(result) == ["mockingbird", "pride"]


def test_query_hybrid_keyword_boost(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # Multiply summary_distance by 0.1 if the summary matches "racial injustice", otherwise
    # multiply by 1.0 (leave unchanged).
    for score_expr in [
        field("summary_distance")
        * (field("summary").match_all("racial injustice").choose(0.1, 1.0)),
        field("summary_distance").boost(field("summary").match_all("racial injustice"), 0.1),
    ]:
        result = ctx.client.collection(collection.name).query(
            select(
                summary_distance=fn.vector_distance("summary_embedding", [2.3] * 16)
            )
            .topk(score_expr, 3, True)
        )

        # Keyword boosting swaps the order of results so we expect [1984, mockingbird, pride]
        # instead of [1984, pride, mockingbird].
        assert doc_ids_ordered(result) == ["1984", "mockingbird", "pride"]

        # We use a modified scoring expression so the results are not sorted by summary_distance.
        assert not is_sorted(result, "summary_distance")

def test_boost_helper_same_expression():
    no_helper = field("summary_distance") * (field("summary").match_all("racial injustice").choose(0.1, 1.0))
    with_helper = field("summary_distance").boost(field("summary").match_all("racial injustice"), 0.1)
    assert no_helper == with_helper

def test_query_hybrid_coalesce_score(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            summary_score=fn.vector_distance("summary_embedding", [4.1] * 16),
            nullable_score=fn.vector_distance("nullable_embedding", [4.1] * 16),
        )
        .topk(
            field("summary_score") + field("nullable_score").coalesce(0.0), 3, True
        )
    )

    # Adding the nullable_score without coalescing would exclude "pride" and "gatsby" from
    # the result set, even though they are the closest candidates based on summary_score.
    assert doc_ids_ordered(result) == ["gatsby", "pride", "catcher"]
