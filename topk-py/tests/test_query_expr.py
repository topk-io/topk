import pytest
from topk_sdk.query import field, literal


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

    assert (field("a") & True)._expr_eq(field("a").and_(True))
    assert (True & field("a"))._expr_eq(field("a").and_(True))

    assert (field("a") | False)._expr_eq(field("a").or_(False))
    assert (False | field("a"))._expr_eq(field("a").or_(False))

    assert (field("a") & field("b"))._expr_eq(field("a").and_(field("b")))
    assert (field("a") | field("b"))._expr_eq(field("a").or_(field("b")))


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
    assert (literal("a") + literal("b"))._expr_eq(literal("a") + literal("b"))
    assert not (literal("a")._expr_eq(literal("b")))
    assert field("a")._expr_eq(field("a"))
    assert not (field("a")._expr_eq(field("b")))
    assert not (field("a")._expr_eq(literal("a")))
    assert not (literal("a")._expr_eq(field("a")))


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


def test_invalid_truthiness():
    error_msg = "Using `and` or `or` keywords with Logical expressions is not supported. Please use `&` or `|` instead."

    # `and`
    with pytest.raises(TypeError) as e:
        field("a") == 1 and field("b") == 2
    assert error_msg in str(e.value)

    # `or`
    with pytest.raises(TypeError) as e:
        field("a") == 1 or field("b") == 2
    assert error_msg in str(e.value)
