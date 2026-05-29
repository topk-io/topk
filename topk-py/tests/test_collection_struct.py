import pytest
from topk_sdk import error
from topk_sdk.data import struct as data_struct
from topk_sdk.query import field, fn, select
from topk_sdk.schema import (
    bool,
    float,
    int,
    list,
    semantic_index,
    struct as schema_struct,
    text,
)

from . import ProjectContext


def test_struct_round_trip(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "outer": schema_struct(
                {"inner": schema_struct({"leaf": text(), "sibling": text()})}
            )
        },
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {
                "_id": "one",
                "outer": data_struct(
                    {
                        "inner": data_struct(
                            {
                                "leaf": "v",
                                "sibling": "s",
                            }
                        )
                    }
                ),
            }
        ]
    )

    docs = ctx.client.collection(collection.name).get(["one"], lsn=lsn)

    assert docs["one"]["outer"]["inner"]["leaf"] == "v"
    assert docs["one"]["outer"]["inner"]["sibling"] == "s"


def test_implicit_struct_schema(ctx: ProjectContext):
    ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "outer": {
                "inner": {
                    "leaf": text(),
                    "sibling": text(),
                }
            }
        },
    )

    collection = ctx.client.collections().get(ctx.scope("test"))

    outer_str = str(collection.schema["outer"])
    assert "Struct" in outer_str
    assert "inner" in outer_str
    assert "leaf" in outer_str
    assert "sibling" in outer_str


def test_struct_query_with_naked_dict(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "meta": schema_struct(
                {
                    "author": text(),
                    "year": int(),
                    "tag": text(),
                }
            )
        },
    )

    ctx.client.collection(collection.name).upsert(
        [
            {
                "_id": "old",
                "meta": {"author": "alice", "year": 1999, "tag": "classic"},
            },
            {
                "_id": "new",
                "meta": {"author": "bob", "year": 2024, "tag": "fresh"},
            },
        ]
    )

    results = ctx.client.collection(collection.name).query(
        select(
            **{
                "meta.author": field("meta.author"),
                "meta.tag": field("meta.tag"),
            }
        )
        .filter(field("meta.year") > 2020)
        .topk(field("meta.year"), 10, True)
    )

    assert len(results) == 1
    assert results[0]["_id"] == "new"
    assert results[0]["meta.author"] == "bob"
    assert results[0]["meta.tag"] == "fresh"
    assert "meta.year" not in results[0]
    assert "meta" not in results[0]


def test_struct_semantic_index_on_subfield(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "meta": schema_struct(
                {"description": text().index(semantic_index())}
            )
        },
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {
                "_id": "rust",
                "meta": {"description": "a systems programming language"},
            },
            {
                "_id": "python",
                "meta": data_struct({"description": "a snake"}),
            },
        ]
    )

    results = ctx.client.collection(collection.name).query(
        select(sim=fn.semantic_similarity("meta.description", "programming")).topk(
            field("sim"), 2, True
        ),
        lsn=lsn,
    )

    assert len(results) == 2
    assert all("sim" in doc for doc in results)


def test_struct_update_deep_merge(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "meta": schema_struct(
                {
                    "author": text(),
                    "title": text(),
                }
            )
        },
    )

    ctx.client.collection(collection.name).upsert(
        [
            {
                "_id": "one",
                "meta": data_struct({"author": "alice", "title": "v1"}),
            }
        ]
    )

    lsn = ctx.client.collection(collection.name).update(
        [{"_id": "one", "meta": {"title": "v2"}}], True
    )

    docs = ctx.client.collection(collection.name).get(["one"], lsn=lsn)

    assert docs["one"]["meta"]["title"] == "v2"
    assert docs["one"]["meta"]["author"] == "alice"


def test_struct_deeply_nested_naked_dicts(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "outer": schema_struct(
                {"middle": schema_struct({"leaf": text()})}
            )
        },
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "one", "outer": {"middle": {"leaf": "deep"}}}]
    )

    docs = ctx.client.collection(collection.name).get(["one"], lsn=lsn)

    assert docs["one"]["outer"]["middle"]["leaf"] == "deep"


def test_struct_naked_dict_all_primitive_types(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "meta": schema_struct(
                {"s": text(), "i": int(), "f": float(), "b": bool()}
            )
        },
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "one", "meta": {"s": "hello", "i": 42, "f": 3.14, "b": True}}]
    )

    docs = ctx.client.collection(collection.name).get(["one"], lsn=lsn)

    assert docs["one"]["meta"]["s"] == "hello"
    assert docs["one"]["meta"]["i"] == 42
    assert abs(docs["one"]["meta"]["f"] - 3.14) < 0.01
    assert docs["one"]["meta"]["b"] is True


def test_struct_naked_dict_with_list_subfield(ctx: ProjectContext):
    # A plain Python list inside a naked dict should round-trip as a list value.
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={
            "meta": schema_struct({"tags": list("text"), "count": int()})
        },
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "one", "meta": {"tags": ["rust", "systems"], "count": 2}}]
    )

    docs = ctx.client.collection(collection.name).get(["one"], lsn=lsn)

    assert docs["one"]["meta"]["tags"] == ["rust", "systems"]
    assert docs["one"]["meta"]["count"] == 2


def test_struct_naked_dict_and_explicit_struct_are_equivalent(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"meta": schema_struct({"author": text(), "year": int()})},
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "naked", "meta": {"author": "alice", "year": 2024}},
            {"_id": "explicit", "meta": data_struct({"author": "alice", "year": 2024})},
        ]
    )

    docs = ctx.client.collection(collection.name).get(["naked", "explicit"], lsn=lsn)

    assert docs["naked"]["meta"]["author"] == docs["explicit"]["meta"]["author"]
    assert docs["naked"]["meta"]["year"] == docs["explicit"]["meta"]["year"]


def test_struct_schema_rejects_dotted_field_name(ctx: ProjectContext):
    with pytest.raises(error.SchemaValidationError):
        ctx.client.collections().create(
            ctx.scope("test"),
            schema={"meta": schema_struct({"bad.name": text()})},
        )
