import pytest
from topk_sdk import error
from topk_sdk.query import agg, field, filter, group_by, select

from . import ProjectContext
from .utils import dataset

# published_year per book in the `books` dataset:
#   mockingbird 1960, 1984 1949, pride 1813, gatsby 1925, catcher 1951,
#   moby 1851, hobbit 1937, harry 1997, lotr 1954, alchemist 1988
#
# `published_year < 1940` splits them into:
#   old  (4): pride 1813, gatsby 1925, moby 1851, hobbit 1937
#   new  (6): mockingbird 1960, 1984 1949, catcher 1951, harry 1997, lotr 1954, alchemist 1988


def test_group_by_bool_key_expr(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        group_by(
            {"is_old": field("published_year") < 1940},
            {"count": agg.count()},
        )
    )

    assert len(result) == 2

    for row in result:
        if row["is_old"]:
            assert row["count"] == 4
        else:
            assert row["count"] == 6


def test_group_by_count(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        group_by(
            {"is_old": field("published_year") < 1940},
            {"count": agg.count()},
        )
    )

    counts = {row["is_old"]: row["count"] for row in result}
    assert counts == {True: 4, False: 6}


def test_group_by_count_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        group_by(
            {"is_old": field("published_year") < 1940},
            {
                "total": agg.count(),
                "with_importance": agg.count("nullable_importance"),
            },
        )
    )

    by_group = {
        row["is_old"]: (row["total"], row["with_importance"]) for row in result
    }
    assert by_group == {True: (4, 1), False: (6, 0)}


def test_group_by_sum(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        group_by(
            {"is_old": field("published_year") < 1940},
            {"total_year": agg.sum("published_year")},
        )
    )

    sums = {row["is_old"]: row["total_year"] for row in result}

    # old: 1813 + 1925 + 1851 + 1937 = 7526
    # new: 1960 + 1949 + 1951 + 1997 + 1954 + 1988 = 11799
    assert sums == {True: 7526, False: 11799}


def test_group_by_min_max(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        group_by(
            {"is_old": field("published_year") < 1940},
            {
                "oldest": agg.min("published_year"),
                "newest": agg.max("published_year"),
            },
        )
    )

    by_group = {row["is_old"]: (row["oldest"], row["newest"]) for row in result}

    # old: min 1813 (pride), max 1937 (hobbit)
    # new: min 1949 (1984), max 1997 (harry)
    assert by_group == {True: (1813, 1937), False: (1949, 1997)}


def test_group_by_avg(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        group_by(
            {"is_old": field("published_year") < 1940},
            {"avg_year": agg.avg("published_year")},
        )
    )

    avgs = {row["is_old"]: row["avg_year"] for row in result}

    # old: 7526 / 4 = 1881.5, new: 11799 / 6 = 1966.5
    assert abs(avgs[True] - 1881.5) < 1e-9
    assert abs(avgs[False] - 1966.5) < 1e-9


def test_group_by_multiple_aggregations(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        group_by(
            {"is_old": field("published_year") < 1940},
            {
                "count": agg.count(),
                "total_year": agg.sum("published_year"),
                "oldest": agg.min("published_year"),
                "newest": agg.max("published_year"),
                "avg_year": agg.avg("published_year"),
            },
        )
    )

    assert len(result) == 2

    for row in result:
        if row["is_old"]:
            assert row["count"] == 4
            assert row["total_year"] == 7526
            assert row["oldest"] == 1813
            assert row["newest"] == 1937
            assert abs(row["avg_year"] - 1881.5) < 1e-9
        else:
            assert row["count"] == 6
            assert row["total_year"] == 11799
            assert row["oldest"] == 1949
            assert row["newest"] == 1997
            assert abs(row["avg_year"] - 1966.5) < 1e-9


def test_group_by_multiple_keys(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # Two independent key expressions:
    #   is_old  = published_year < 1940
    #   is_19th = published_year < 1900
    #
    #   pride 1813:  (old, 19th)
    #   moby  1851:  (old, 19th)
    #   gatsby 1925: (old, !19th)
    #   hobbit 1937: (old, !19th)
    #   the other 6: (!old, !19th)
    result = ctx.client.collection(collection.name).query(
        group_by(
            {
                "is_old": field("published_year") < 1940,
                "is_19th": field("published_year") < 1900,
            },
            {"count": agg.count()},
        )
    )

    counts = {(row["is_old"], row["is_19th"]): row["count"] for row in result}

    assert counts == {(True, True): 2, (True, False): 2, (False, False): 6}


def test_group_by_with_filter(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # Filter to books published in 1940 or later (drops the 4 "old" books),
    # then group the remaining 6 by whether they were published after 1980.
    #   after 1980: harry 1997, alchemist 1988          -> 2
    #   otherwise:  mockingbird, 1984, catcher, lotr     -> 4
    result = ctx.client.collection(collection.name).query(
        filter(field("published_year") >= 1940).group_by(
            {"recent": field("published_year") > 1980},
            {"count": agg.count()},
        )
    )

    counts = {row["recent"]: row["count"] for row in result}

    assert counts == {True: 2, False: 4}


def test_group_by_with_projected_columns(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # A preceding `select` projects computed columns (`year`, `old`) which the
    # group_by stage then references in both its key and its aggregations.
    result = ctx.client.collection(collection.name).query(
        select(
            year=field("published_year"),
            old=field("published_year") < 1940,
        ).group_by(
            {"old": field("old")},
            {
                "count": agg.count(),
                "total_year": agg.sum("year"),
            },
        )
    )

    by_group = {row["old"]: (row["count"], row["total_year"]) for row in result}

    assert by_group == {True: (4, 7526), False: (6, 11799)}


def test_group_by_then_filter(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # Group into old (4) / new (6), then keep only groups with more than 4 members.
    result = ctx.client.collection(collection.name).query(
        group_by(
            {"is_old": field("published_year") < 1940},
            {"count": agg.count()},
        ).filter(field("count") > 4)
    )

    assert len(result) == 1
    assert result[0]["is_old"] is False
    assert result[0]["count"] == 6


def test_group_by_then_sort_limit(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # Group, then take the single largest group by count.
    result = ctx.client.collection(collection.name).query(
        group_by(
            {"is_old": field("published_year") < 1940},
            {"count": agg.count()},
        )
        .sort(field("count"), False)
        .limit(1)
    )

    assert len(result) == 1
    assert result[0]["is_old"] is False
    assert result[0]["count"] == 6


def test_group_by_then_select(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    # A `select` after group_by projects a subset / renaming of the grouped output.
    result = ctx.client.collection(collection.name).query(
        group_by(
            {"is_old": field("published_year") < 1940},
            {"count": agg.count()},
        ).select(n=field("count"))
    )

    assert len(result) == 2

    ns = sorted(row["n"] for row in result)
    assert ns == [4, 6]


def test_group_by_empty_keys(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError, match="at least one key"):
        ctx.client.collection(collection.name).query(
            group_by({}, {"count": agg.count()})
        )


def test_group_by_empty_aggregations(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    with pytest.raises(error.InvalidArgumentError, match="at least one aggregation"):
        ctx.client.collection(collection.name).query(
            group_by({"is_old": field("published_year") < 1940}, {})
        )
