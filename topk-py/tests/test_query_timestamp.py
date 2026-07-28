from datetime import datetime, timezone

from topk_sdk.query import agg, field, filter, group_by, literal

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


def test_naive_datetime_is_interpreted_as_utc(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)
    c = ctx.client.collection(collection.name)

    c.upsert(
        [
            {
                "_id": "naive",
                "title": "Naive",
                "published_year": 2023,
                # naive datetime - must be interpreted as UTC, not host local time
                "published_ts": datetime(2023, 1, 1),
                "summary": "A test document.",
                "summary_embedding": [1.0] * 16,
            }
        ]
    )

    docs = c.get(["naive"])
    assert docs["naive"]["published_ts"] == 1672531200000  # 2023-01-01T00:00:00Z


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
