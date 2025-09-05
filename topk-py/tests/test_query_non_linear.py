import math
import pytest
from topk_sdk import error
from topk_sdk.query import field, filter, select, fn, match

from . import ProjectContext
from .utils import dataset, doc_ids, doc_ids_ordered


def test_query_exp_ln(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(bm25_score=fn.bm25_score())
        .select(
            bm25_score_scale=(field("bm25_score") * 1.5).exp(),
            bm25_score_smooth=(field("bm25_score") + 1).ln(),
        )
        .filter(match("millionaire love consequences dwarves", field="summary", weight=1.0, all=False))
        .topk(field("bm25_score_scale"), 2, False)
    )

    assert doc_ids_ordered(result) == ["gatsby", "hobbit"]

    for doc in result:
        bm25_score = doc["bm25_score"]
        bm25_score_scale = doc["bm25_score_scale"]
        bm25_score_smooth = doc["bm25_score_smooth"]
        assert abs(math.exp(bm25_score * 1.5) - bm25_score_scale) < 1e-4
        assert abs(math.log(bm25_score + 1.0) - bm25_score_smooth) < 1e-4


def test_query_float_inf(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(to_infinity=field("published_year").exp()).topk(
            field("published_year"), 2, True
        )
    )

    assert len(result) == 2

    for doc in result:
        to_infinity = doc["to_infinity"]
        assert to_infinity == float("inf")


def test_query_sqrt_square(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(
            published_year=field("published_year"),
            published_year_2=field("published_year").sqrt().square(),
        ).topk(field("published_year_2"), 2, True)
    )

    assert doc_ids_ordered(result) == ["pride", "moby"]

    for doc in result:
        year_2 = doc["published_year_2"]
        year_orig = doc["published_year"]
        assert round(year_2) == year_orig


def test_query_sqrt_filter(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"))
        .filter(field("published_year").sqrt() > math.sqrt(1990))
        .topk(field("published_year"), 2, True)
    )

    assert result == [{"_id": "harry", "title": "Harry Potter and the Sorcerer's Stone"}]
