import pytest
from topk_sdk import error
from topk_sdk.query import field, filter, select, fn

from . import ProjectContext
from .utils import dataset

def test_query_topk_clamping(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
            select(
                summary_distance=fn.vector_distance("summary_embedding", [2.0, 16]),
                bm25_score=fn.bm25_score()
            ).topk(
                (field("bm25_score").max(3).min(10)) 
                + (field("summary_distance") * 0.5),
                2,
                True,
            ),
            None,
            None
        )


def test_query_topk_pow_sqrt(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
        select(
            summary_distance=fn.vector_distance("summary_embedding", [2.0, 16]),
            bm25_score=fn.bm25_score()
        ).topk(
            (
                (field("bm25_score")**1.5) +
                pow(field("summary_distance"), 2)
            ).sqrt(),
            2,
            True,
        ),
        None,
        None
    )

def test_query_topk_exp_precision(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
            filter(
                abs(field("published_year").exp().ln() - 1988) < 10e-6
            ).topk(
                field("published_year"),
                2,
                True,
            ),
            None,
            None
        )

# python specific: modulo pow should fail
def test_modulo_pow_fail():
    with pytest.raises(NotImplementedError):
        pow(field("test"), 1.5, 5)
