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


def test_alternative_syntax_operators():
    # Test explicit method calls
    assert field("a").add(1)._expr_eq(field("a").add(literal(1)))
    assert field("a").sub(1)._expr_eq(field("a").sub(literal(1)))
    assert field("a").mul(1)._expr_eq(field("a").mul(literal(1)))
    assert field("a").div(1)._expr_eq(field("a").div(literal(1)))

    # Test explicit comparison methods
    assert field("a").eq(1)._expr_eq(field("a").eq(literal(1)))
    assert field("a").ne(1)._expr_eq(field("a").ne(literal(1)))
    assert field("a").lt(1)._expr_eq(field("a").lt(literal(1)))
    assert field("a").lte(1)._expr_eq(field("a").lte(literal(1)))
    assert field("a").gt(1)._expr_eq(field("a").gt(literal(1)))
    assert field("a").gte(1)._expr_eq(field("a").gte(literal(1)))


def test_complex_expressions():
    # Test nested operations
    expr1 = field("a") + field("b") * 2
    expr2 = field("a").add(field("b").mul(2))
    assert expr1._expr_eq(expr2)

    # Test mixed operations
    expr5 = (field("a") + field("b")) > (field("c") * 2)
    expr6 = field("a").add(field("b")).gt(field("c").mul(2))
    assert expr5._expr_eq(expr6)


def test_expression_string_representation():
    # Test basic expressions
    assert str(field("a")) == "field(a)"
    assert str(literal(1)) == "literal(Int(1))"
    assert str(literal("test")) == "literal('test')"

    # Test compound expressions
    assert "add" in str(field("a") + field("b"))
    assert "mul" in str(field("a") * field("b"))
    assert "eq" in str(field("a") == field("b"))


def test_more_invalid_operations():
    with pytest.raises(TypeError):
        field("a") + object()  # type: ignore

    with pytest.raises(TypeError):
        field("a") & "string"  # type: ignore

    with pytest.raises(TypeError):
        field("a") | 3.14  # type: ignore

    with pytest.raises(TypeError):
        literal(1) + field("a") + "string"  # type: ignore

    with pytest.raises(TypeError):
        field("a") + (1, 2, 3)  # type: ignore

    with pytest.raises(TypeError):
        field("a") + {"key": "value"}  # type: ignore
