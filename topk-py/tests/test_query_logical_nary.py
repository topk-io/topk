from topk_sdk.query import field, filter, not_, select, all, any

from . import ProjectContext
from .utils import dataset, doc_ids


def test_any_codes_vec(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            any([
                field("codes").contains("DOI 10.1000/182"),
                field("codes").contains("Barcode 0618346252"),
                field("codes").contains("UPC 025192354670"),
            ])
        ).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"1984", "lotr", "mockingbird"}


def test_all_codes_vec(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            all([
                field("tags").contains("wizard"),
                field("tags").contains("school"),
                field("tags").contains("magic"),
            ])
        ).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"harry"}


def test_select_any_flag(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(
            has_code=any([
                field("codes").contains("DOI 10.1000/182"),
                field("codes").contains("OCLC 934546789"),
            ])
        )
        .filter(
            (field("_id") == "1984") | (field("_id") == "pride") | (field("_id") == "lotr")
        )
        .topk(field("published_year"), 100, True)
    )

    results.sort(key=lambda d: d["_id"])

    assert results == [
        {"_id": "1984", "has_code": True},
        {"_id": "lotr", "has_code": False},
        {"_id": "pride", "has_code": True},
    ]


def test_select_all_flag(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    results = ctx.client.collection(collection.name).query(
        select(
            all_match=all([
                field("codes").contains("UPC 074327356709"),
                field("codes").contains("ASIN B000FC0SIS"),
            ])
        )
        .filter(field("_id").in_(["gatsby", "pride"]))
        .topk(field("published_year"), 100, True)
    )

    results.sort(key=lambda d: d["_id"])

    assert len(results) == 2
    assert results[0]["_id"] == "gatsby"
    assert results[0]["all_match"] is True
    assert results[1]["_id"] == "pride"
    assert results[1]["all_match"] is False


def test_nested_any_all(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    expr = any([
        all([
            field("tags").contains("wizard"),
            field("tags").contains("magic"),
        ]),
        all([
            field("codes").contains("UPC 074327356709"),
            field("codes").contains("ASIN B000FC0SIS"),
        ]),
    ])

    result = ctx.client.collection(collection.name).query(
        filter(expr).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"gatsby", "harry", "lotr"}


def test_non_nested_any_and_all(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    codes_any = any([
        field("codes").contains("Barcode 0618346252"),
        field("codes").contains("UPC 043970818909"),
    ])

    tags_all = all([
        field("tags").contains("wizard"),
        field("tags").contains("magic"),
    ])

    result = ctx.client.collection(collection.name).query(
        filter(codes_any & tags_all).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"harry", "lotr"}


def test_any_mixed_exprs(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            any([
                field("title").starts_with("The Great"),
                field("tags").contains("romance"),
                field("published_year") < 1900,
            ])
        ).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"pride", "moby", "gatsby"}


def test_all_mixed_exprs(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    result = ctx.client.collection(collection.name).query(
        filter(
            all([
                field("published_year") > 1900,
                field("title").contains("The"),
                not_(field("tags").contains("romance")),
            ])
        ).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"alchemist", "catcher", "hobbit", "lotr"}


def test_all_large_arity(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    expr = all([field("tags").contains("wizard") for _ in range(32)])

    result = ctx.client.collection(collection.name).query(
        filter(expr).topk(field("published_year"), 100, True)
    )

    assert doc_ids(result) == {"harry", "lotr"}


def test_all_max_arity(ctx: ProjectContext):
    collection = dataset.books.setup(ctx)

    expr = all([field("tags").contains("wizard") for _ in range(33)])

    # This should fail due to max arity
    try:
        ctx.client.collection(collection.name).query(
            filter(expr).topk(field("published_year"), 100, True)
        )
        assert False, "Should have failed due to max arity"
    except Exception as e:
        assert "N-ary expression has too many operands" in str(e)
