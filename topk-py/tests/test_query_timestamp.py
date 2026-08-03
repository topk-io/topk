from datetime import date, datetime, timedelta, timezone
from zoneinfo import ZoneInfo

import pytest

from topk_sdk import error
from topk_sdk.query import agg, field, filter, group_by, literal, select

from . import ProjectContext
from .utils import dataset, doc_ids

# published_ts per book in the `books` dataset:
#   mockingbird 1960-07-11, 1984 1949-06-08, pride 1813-01-28, gatsby 1925-04-10,
#   catcher 1951-07-16, moby 1851-10-18, hobbit 1937-09-21, harry 1997-06-26,
#   lotr 1954-07-29, alchemist 1988-01-01


def test_query_filter_timestamp(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            field("published_ts") < literal(datetime(1929, 1, 1, tzinfo=timezone.utc))
        ).limit(20)
    )

    assert doc_ids(result) == {"pride", "moby", "gatsby"}


def test_datetime_truncates_to_epoch_millis():
    assert literal(
        datetime(1970, 1, 1, 0, 0, 0, 999500, tzinfo=timezone.utc)
    )._expr_eq(literal(999))


def test_query_date_part_eq_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            field("published_ts").date_part("year") == field("published_year")
        ).count()
    )

    assert result[0]["_count"] == 10


def test_query_date_part_lt_literal(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("published_ts").date_part("month") < 6).limit(10)
    )

    assert doc_ids(result) == {"gatsby", "pride", "alchemist"}


def test_naive_datetime_is_rejected(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    c = ctx.client.collection(collection.name)

    with pytest.raises(error.InvalidArgumentError, match="timezone-naive"):
        c.upsert(
            [
                {
                    "_id": "naive",
                    "title": "Naive",
                    "published_year": 2023,
                    "published_ts": datetime(2023, 1, 1), # naive datetime
                    "summary": "A test document.",
                    "summary_embedding": [1.0] * 16,
                }
            ]
        )


def test_fixed_offset_datetime_uses_utc_offset():
    # 12:00 at UTC-5 == 17:00 UTC
    dt = datetime(2023, 1, 1, 12, 0, tzinfo=timezone(timedelta(hours=-5)))
    assert literal(dt)._expr_eq(literal(1672592400000))


def test_zoneinfo_datetime_uses_utc_offset():
    # 12:00 in New York (UTC-5 in January) == 17:00 UTC
    dt = datetime(2023, 1, 1, 12, 0, tzinfo=ZoneInfo("America/New_York"))
    assert literal(dt)._expr_eq(literal(1672592400000))


def test_date_is_padded_to_midnight_utc():
    assert literal(date(1970, 1, 2))._expr_eq(literal(24 * 60 * 60 * 1000))


def test_upsert_date_is_padded_to_midnight_utc(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    c = ctx.client.collection(collection.name)

    c.upsert(
        [
            {
                "_id": "dateonly",
                "title": "Date Only",
                "published_year": 2023,
                "published_ts": date(2023, 1, 1),
                "summary": "A test document.",
                "summary_embedding": [1.0] * 16,
            }
        ]
    )

    docs = c.get(["dateonly"])
    assert docs["dateonly"]["published_ts"] == 1672531200000  # 2023-01-01T00:00:00Z


@pytest.mark.parametrize(
    "interval,expected",
    [
        ("millisecond", 604_800_000),
        ("second", 604_800),
        ("minute", 10_080),
        ("hour", 168),
        ("day", 7),
        ("week", 1),
    ],
)
def test_query_elapsed(ctx: ProjectContext, interval, expected: int):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            field("published_ts").elapsed(
                literal(datetime(1988, 1, 8, tzinfo=timezone.utc)),
                interval,  # pyright: ignore[reportUnknownArgumentType]
            )
            == expected
        ).limit(10)
    )

    assert doc_ids(result) == {"alchemist"}


@pytest.mark.parametrize(
    "part,expected",
    [
        ("year", 1997),
        ("month", 6),
        ("week", 26),
        ("day", 26),
        ("day_of_year", 177),
        ("day_of_week", 3),
        ("hour", 12),
        ("minute", 34),
        ("second", 56),
        ("millisecond", 789),
    ],
)
def test_query_date_part_all_parts(ctx: ProjectContext, part, expected: int):
    collection = dataset.books.setup(ctx)
    c = ctx.client.collection(collection.name)

    c.upsert(
        [
            {
                "_id": "precise",
                "title": "Precise",
                "published_year": 1997,
                "published_ts": datetime(
                    1997, 6, 26, 12, 34, 56, 789000, tzinfo=timezone.utc
                ),
                "summary": "A test document.",
                "summary_embedding": [1.0] * 16,
            }
        ]
    )

    result = c.query(
        select(value=field("published_ts").date_part(part))  # pyright: ignore[reportUnknownArgumentType]
        .filter(field("_id") == "precise")
        .limit(1)
    )

    assert result == [{"_id": "precise", "value": expected}]


def test_query_date_part_group_by(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        group_by(
            {"published_month": field("published_ts").date_part("month")},
            {"count": agg.count()},
        )
    )

    rows = sorted((row["published_month"], row["count"]) for row in result)
    assert rows == [(1, 2), (4, 1), (6, 2), (7, 3), (9, 1), (10, 1)]
