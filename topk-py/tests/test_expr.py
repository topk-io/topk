import pytest
from topk_sdk.query import field, literal, not_


def test_query_expr_with_flexible_expr():
    assert (field("a") + 1)._expr_eq(field("a") + literal(1))
    assert (1 + field("a"))._expr_eq(field("a") + literal(1))

    assert (field("a") - 1)._expr_eq(field("a") - literal(1))
    assert (1 - field("a"))._expr_eq(literal(1) - field("a"))

    assert (field("a") * 1)._expr_eq(field("a") * literal(1))
    assert (1 * field("a"))._expr_eq(field("a") * literal(1))

    assert (field("a") / 1)._expr_eq(field("a") / literal(1))
    assert (1 / field("a"))._expr_eq(literal(1) / field("a"))

    assert (field("a") & True)._expr_eq(field("a") & literal(True))
    assert (True & field("a"))._expr_eq(field("a") & literal(True))

    assert (field("a") | False)._expr_eq(field("a") | literal(False))
    assert (False | field("a"))._expr_eq(field("a") | literal(False))


def test_comparison_operators():
    assert (field("a") == 1)._expr_eq(field("a") == literal(1))
    assert (1 == field("a"))._expr_eq(field("a") == literal(1))  # type: ignore

    assert (field("a") != 1)._expr_eq(field("a") != literal(1))
    assert (1 != field("a"))._expr_eq(field("a") != literal(1))  # type: ignore

    assert (field("a") < 1)._expr_eq(field("a") < literal(1))
    assert (1 > field("a"))._expr_eq(field("a") < literal(1))

    assert (field("a") <= 1)._expr_eq(field("a") <= literal(1))
    assert (1 >= field("a"))._expr_eq(field("a") <= literal(1))

    assert (field("a") > 1)._expr_eq(field("a") > literal(1))
    assert (1 < field("a"))._expr_eq(field("a") > literal(1))

    assert (field("a") >= 1)._expr_eq(field("a") >= literal(1))
    assert (1 <= field("a"))._expr_eq(field("a") >= literal(1))


def test_query_expr_eq():
    assert literal("a") + literal("b") == literal("a") + literal("b")
    assert literal("a") != literal("b")
    assert field("a") == field("a")
    assert field("a") != field("b")
    assert field("a") != literal("a")
    assert literal("a") != field("a")


def test_query_literal():
    field("foo").eq(literal(1))
    field("foo").eq(1)

    field("foo").ne(literal(1))
    field("foo").ne(1)


def test_invalid():
    with pytest.raises(TypeError):
        literal(1) + "string"  # type: ignore

    with pytest.raises(TypeError):
        field("a") & 1  # type: ignore

    with pytest.raises(TypeError):
        field("a") | 1  # type: ignore

    with pytest.raises(TypeError):
        field("a") + None  # type: ignore

    with pytest.raises(TypeError):
        field("a") + "string"  # type: ignore

    with pytest.raises(TypeError):
        field("a") + [1, 2, 3]  # type: ignore

    with pytest.raises(TypeError):
        field("a") + {"a": 1}  # type: ignore
