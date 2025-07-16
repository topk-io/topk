import pytest
from topk_sdk import error
from topk_sdk.query import field, fn, literal, match, select, filter

from . import ProjectContext
from .utils import dataset, doc_ids


# TODO assert either sdk error or None value
@pytest.mark.skip(reason="TODO fix")
def test_zero_division(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    results = ctx.client.collection(collection.name).query(
        select(
            "title",
            hello=field("published_year") / 0,
        )
        .filter(field("title") == "To Kill a Mockingbird")
        .topk(field("published_year"), 1, True)
    )

# TODO more descriptive error from sdk than TypeError: unsupported operand type(s) for +: 'builtins.LogicalExpr_Field' and 'int'
@pytest.mark.skip(reason="TODO fix")
# python specific: 
def test_large_int(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    results = ctx.client.collection(collection.name).query(
        select(
            "title",
            hello=field("published_year") + (2**(10000)),
        )
        .filter(field("title") == "To Kill a Mockingbird")
        .topk(field("published_year"), 100, True)
    )

@pytest.mark.skip(reason="TODO fix")
def test_query_abs_delta(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError):
        ctx.client.collection(collection.name).query(
            filter(
                abs(field("published_year")-1949) <= 1
            ).topk(field("published_year"), 100, True)
        )

# TODO: more descriptive error than error.InvalidArgumentError: Missing expr
@pytest.mark.skip(reason="TODO fix")
def test_sparse_query_dense_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    ctx.client.collection(collection.name).query(
        select(
            score=fn.vector_distance("summary_embedding",  {0: 1.0, 1: 2.0, 2: 3.0})
        ).topk(field("score"), 10)
    )

@pytest.mark.skip(reason="TODO fix")
def test_int_overflow(ctx: ProjectContext):
    pass # TODO

@pytest.mark.skip(reason="TODO fix")
def test_float_special_values(ctx: ProjectContext):
    pass # TODO test NaN, Inf, -Inf in both input and output