import pytest
from topk_sdk import data, error
from topk_sdk.query import field, filter, select, literal, not_

from . import ProjectContext
from .utils import dataset, doc_ids


def test_string_contains_literal(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").contains("ob")).topk(field("published_year"), 100, False)
    )

    assert doc_ids(result) == {"moby", "hobbit"}


def test_string_contains_literal_no_match(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").contains("rubbish")).topk(
            field("published_year"), 100, False
        )
    )

    assert len(result) == 0


def test_string_contains_literal_empty(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").contains("")).topk(field("published_year"), 100, False)
    )

    assert doc_ids(result) == {
        "gatsby",
        "catcher",
        "moby",
        "mockingbird",
        "alchemist",
        "harry",
        "lotr",
        "pride",
        "1984",
        "hobbit",
    }


def test_string_contains_literal_with_keyword_index(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("summary").contains("to h")).topk(
            field("published_year"), 100, False
        )
    )

    assert doc_ids(result) == {"moby", "hobbit"}


def test_string_contains_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("title").contains(field("_id"))).topk(
            field("published_year"), 100, False
        )
    )

    assert doc_ids(result) == {"1984"}


def test_string_in_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").in_(field("title"))).topk(
            field("published_year"), 100, False
        )
    )

    assert doc_ids(result) == {"1984"}


def test_string_contains_field_self(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(not_(field("title").contains(field("title")))).topk(
            field("published_year"), 100, False
        )
    )

    assert result == []


def test_list_match_any_with_keyword_index(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"), tags=field("tags"))
        .filter(field("tags").match_any("love"))
        .topk(field("published_year"), 100, True)
    )

    assert result == [
        {"_id": "pride", "title": "Pride and Prejudice", "tags": [
            "pride", "love", "romance", "class", "marriage", "prejudice"
        ]},
        {"_id": "gatsby", "title": "The Great Gatsby", "tags": [
            "love", "romance", "wealth", "marriage"
        ]},
    ]


def test_list_match_any_all_without_keyword_index(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    for filter_expr in [
        field("codes").match_any("ISBN 0-547-92821-2"),
        field("codes").match_all("ISBN 0-547-92821-2"),
    ]:
        with pytest.raises(error.InvalidArgumentError):
            ctx.client.collection(collection.name).query(
                select(title=field("title"), codes=field("codes"))
                .filter(filter_expr)
                .topk(field("published_year"), 100, True)
            )


def test_list_contains_with_keyword_index(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"), tags=field("tags"))
        .filter(field("tags").contains("love"))
        .topk(field("published_year"), 100, True)
    )

    assert result == [
        {"_id": "pride", "title": "Pride and Prejudice", "tags": [
            "pride", "love", "romance", "class", "marriage", "prejudice"
        ]},
        {"_id": "gatsby", "title": "The Great Gatsby", "tags": [
            "love", "romance", "wealth", "marriage"
        ]},
    ]


def test_list_contains_literal(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"), codes=field("codes"))
        .filter(field("codes").contains("ISBN 0-547-92821-2"))
        .topk(field("published_year"), 100, True)
    )

    assert result == [
        {
            "_id": "lotr",
            "title": "The Lord of the Rings: The Fellowship of the Ring",
            "codes": [
                "ISBN 978-0-547-92821-0",
                "ISBN 0-547-92821-2",
                "OCLC 434394005",
                "LCCN 2004558654",
                "Barcode 0618346252",
            ],
        }
    ]


def test_list_contains_int_literal(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"), reprint_years=field("reprint_years"))
        .filter(field("reprint_years").contains(1999))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"mockingbird", "harry"}


def test_list_contains_int_literal_different_type(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"), reprint_years=field("reprint_years"))
        .filter(field("reprint_years").contains(literal(1999)))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"mockingbird", "harry"}


def test_list_contains_int_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"), reprint_years=field("reprint_years"))
        .filter(field("reprint_years").contains(field("published_year") + 1))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"harry", "1984"}


def test_list_in_int_field(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"), reprint_years=field("reprint_years"))
        .filter((field("published_year") + 1).in_(field("reprint_years")))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"harry", "1984"}


def test_list_contains_string_field_with_keyword_index(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"), tags=field("tags"))
        .filter(field("tags").contains(field("_id")))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"pride", "hobbit"}


def test_list_in_string_field_with_keyword_index(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"), tags=field("tags"))
        .filter(field("_id").in_(field("tags")))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"pride", "hobbit"}


def test_list_contains_string_field_without_keyword_index(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"), codes=field("codes"))
        .filter(field("codes").contains(field("_id")))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"1984"}


def test_list_contains_invalid_types(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    for filter_expr in [
        field("codes").contains(978),
        field("codes").contains(True),
        field("codes").contains(field("published_year")),
        field("reprint_years").contains(field("title")),
        field("published_year").contains(field("reprint_years")),
    ]:
        with pytest.raises(error.InvalidArgumentError):
            ctx.client.collection(collection.name).query(
                select(title=field("title"), codes=field("codes"))
                .filter(filter_expr)
                .topk(field("published_year"), 100, True)
            )

    with pytest.raises(TypeError):
        field("codes").contains([978]),  # type: ignore
        field("codes").contains([  # type: ignore
            "ISBN 978-0-547-92821-0",
            "ISBN 0-547-92821-2",
        ])


def test_string_in(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(field("_id").in_("harryhobbitlotr")).topk(
            field("published_year"), 100, False
        )
    )

    assert doc_ids(result) == {"harry", "hobbit", "lotr"}


def test_in_list_literal_int(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"))
        .filter(field("published_year").in_([1999, 1988, 1997]))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"alchemist", "harry"}


def test_in_list_literal_int_u32(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"))
        .filter(field("published_year").in_(data.u32_list([1999, 1988, 1997])))
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"alchemist", "harry"}


def test_in_list_literal_string(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        select(title=field("title"))
        .filter(
            field("title").in_(
                [
                    "The Great Gatsby",
                    "The Catcher in the Rye",
                    "The Lord of the Rings: NOT THIS ONE",
                    "The",
                    "something 123",
                ]
            )
        )
        .topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"gatsby", "catcher"}
