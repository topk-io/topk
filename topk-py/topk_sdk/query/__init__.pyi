import builtins
import typing
from enum import Enum

import topk_sdk.data

class LogicalExpr(Enum):
    """
    *Internal*

    Instances of the `LogicalExpr` class are used to represent logical expressions in TopK.
    Usually created using logical constructors such as [`field()`](#field), [`literal()`](#literal), etc.
    """

    def __repr__(self) -> builtins.str:
        ...
    def _expr_eq(self, other: LogicalExpr) -> LogicalExpr:
        ...
    def is_null(self) -> LogicalExpr:
        """
        Check if the expression is null.
        """
        ...
    def is_not_null(self) -> LogicalExpr:
        """
        Check if the expression is not null.
        """
        ...
    def abs(self) -> LogicalExpr:
        """
        Compute the absolute value of the expression.
        """
        ...
    def __abs__(self) -> LogicalExpr:
        """
        Compute the absolute value of the expression.
        """
        ...
    def ln(self) -> LogicalExpr:
        """
        Compute the natural logarithm of the expression.
        """
        ...
    def exp(self) -> LogicalExpr:
        """
        Compute the exponential of the expression.
        """
        ...
    def sqrt(self) -> LogicalExpr:
        """
        Compute the square root of the expression.
        """
        ...
    def square(self) -> LogicalExpr:
        """
        Compute the square of the expression.
        """
        ...
    def eq(self, other: FlexibleExpr) -> LogicalExpr:
        """
        Check if the expression is equal to another expression.
        """
        ...
    def __eq__(self, other: FlexibleExpr) -> LogicalExpr:
        """
        Check if the expression is equal to another expression using the `==` operator.
        """
        ...
    def ne(self, other: FlexibleExpr) -> LogicalExpr:
        """
        Check if the expression is not equal to another expression.
        """
        ...
    def __ne__(self, other: FlexibleExpr) -> LogicalExpr:
        """
        Check if the expression is not equal to another expression using the `!=` operator.
        """
        ...
    def lt(self, other: Ordered) -> LogicalExpr:
        """
        Check if the expression is less than another expression.
        """
        ...
    def __lt__(self, other: Ordered) -> LogicalExpr:
        """
        Check if the expression is less than another expression using the `<` operator.
        """
        ...
    def __rlt__(self, other: Ordered) -> LogicalExpr:
        """
        Check if the other expression is less than the self expression using the `>` operator.
        """
        ...
    def lte(self, other: Ordered) -> LogicalExpr:
        """
        Check if the expression is less than or equal to another expression.
        """
        ...
    def __le__(self, other: Ordered) -> LogicalExpr:
        """
        Check if the expression is less than or equal to another expression using the `<=` operator.
        """
        ...
    def __rle__(self, other: Ordered) -> LogicalExpr:
        """
        Check if the other expression is less than or equal to the self expression using the `<=` operator.
        """
        ...
    def gt(self, other: Ordered) -> LogicalExpr:
        """
        Check if the expression is greater than another expression.
        """
        ...
    def __gt__(self, other: Ordered) -> LogicalExpr:
        """
        Check if the expression is greater than another expression using the `>` operator.
        """
        ...
    def __rgt__(self, other: Ordered) -> LogicalExpr:
        """
        Check if the expression is greater than another expression using the `<` operator.
        """
        ...
    def gte(self, other: Ordered) -> LogicalExpr:
        """
        Check if the expression is greater than or equal to another expression.
        """
        ...
    def __ge__(self, other: Ordered) -> LogicalExpr:
        """
        Check if the expression is greater than or equal to another expression using the `>=` operator.
        """
        ...
    def __rge__(self, other: Ordered) -> LogicalExpr:
        """
        Check if the other expression is greater than or equal to the self expression using the `<=` operator.
        """
        ...
    def add(self, other: Numeric) -> LogicalExpr:
        """
        Add another value to the expression.
        """
        ...
    def __add__(self, other: Numeric) -> LogicalExpr:
        """
        Add another value to the expression using the `+` operator.
        """
        ...
    def __radd__(self, other: Numeric) -> LogicalExpr:
        """
        Add the other value to the self expression using the `+` operator.
        """
        ...
    def sub(self, other: Numeric) -> LogicalExpr:
        """
        Subtract another value from the expression.
        """
        ...
    def __sub__(self, other: Numeric) -> LogicalExpr:
        """
        Subtract another value from the expression using the `-` operator.
        """
        ...
    def __rsub__(self, other: Numeric) -> LogicalExpr:
        """
        Subtract the other value from the self expression using the `-` operator.
        """
        ...
    def mul(self, other: Numeric) -> LogicalExpr:
        """
        Multiply the expression by another value.
        """
        ...
    def __mul__(self, other: Numeric) -> LogicalExpr:
        """
        Multiply the expression by another value using the `*` operator.
        """
        ...
    def __rmul__(self, other: Numeric) -> LogicalExpr:
        """
        Multiply the other value by the self expression using the `*` operator.
        """
        ...
    def div(self, other: Numeric) -> LogicalExpr:
        """
        Divide the expression by another value.
        """
        ...
    def __div__(self, other: Numeric) -> LogicalExpr:
        """
        Divide the expression by another value using the `/` operator.
        """
        ...
    def __truediv__(self, other: Numeric) -> LogicalExpr:
        """
        Divide the expression by another value using the `/` operator.
        """
        ...
    def __rdiv__(self, other: Numeric) -> LogicalExpr:
        """
        Divide the other value by the self expression using the `/` operator.
        """
        ...
    def __rtruediv__(self, other: Numeric) -> LogicalExpr:
        """
        Divide the other value by the self expression using the `/` operator.
        """
        ...
    def min(self, other: Ordered) -> LogicalExpr:
        """
        Compute the minimum of the expression and another value.
        """
        ...
    def max(self, other: Ordered) -> LogicalExpr:
        """
        Compute the maximum of the expression and another value.
        """
        ...
    def and_(self, other: Boolish) -> LogicalExpr:
        """
        Compute the logical AND of the expression and another expression.
        """
        ...
    def __and__(self, other: Boolish) -> LogicalExpr:
        """
        Compute the logical AND of the expression and another expression using the `&` operator.
        """
        ...
    def __rand__(self, other: Boolish) -> LogicalExpr:
        """
        Compute the logical AND of the other expression and the self expression using the `&` operator.
        """
        ...
    def or_(self, other: Boolish) -> LogicalExpr:
        """
        Compute the logical OR of the expression and another expression.
        """
        ...
    def __or__(self, other: Boolish) -> LogicalExpr:
        """
        Compute the logical OR of the expression and another expression using the `|` operator.
        """
        ...
    def __ror__(self, other: Boolish) -> LogicalExpr:
        """
        Compute the logical OR of the other expression and the self expression using the `|` operator.
        """
        ...
    def starts_with(self, other: Stringy) -> LogicalExpr:
        """
        Check if the expression starts with the provided string expression.
        """
        ...
    def contains(self, other: FlexibleExpr) -> LogicalExpr:
        """
        Check if the expression contains another value.
        """
        ...
    def in_(self, other: Iterable) -> LogicalExpr:
        """
        Check if the expression is in the provided iterable expression.
        """
        ...
    def match_all(self, other: StringyWithList) -> LogicalExpr:
        """
        Check if the expression matches all terms against the field with keyword index.
        """
        ...
    def match_any(self, other: StringyWithList) -> LogicalExpr:
        """
        Check if the expression matches any term against the field with keyword index.
        """
        ...
    def coalesce(self, other: Numeric) -> LogicalExpr:
        """
        Coalesce nulls in the expression with another value.
        """
        ...
    def choose(self, x: FlexibleExpr, y: FlexibleExpr) -> LogicalExpr:
        """
        Choose between two values based on the expression.
        """
        ...
    def boost(self, condition: FlexibleExpr, boost: Numeric) -> LogicalExpr:
        """
        Multiply the scoring expression by the provided `boost` value if the `condition` is true.
        """
        ...

FlexibleExpr = typing.Union[str, int, float, bool, None, LogicalExpr]
Numeric = typing.Union[int, float, LogicalExpr]
Ordered = typing.Union[int, float, str, LogicalExpr]
Boolish = typing.Union[bool, LogicalExpr]
Stringy = typing.Union[str, LogicalExpr]
StringyWithList = typing.Union[str, builtins.list[str], LogicalExpr]
Iterable = typing.Union[str, builtins.list[int], builtins.list[float], builtins.list[str], topk_sdk.data.List, LogicalExpr]

class FunctionExpr:
    """
    *Internal*

    Instances of the `FunctionExpr` class are used to represent function expressions in TopK.
    Usually created using function constructors such as [`fn.vector_distance()`](#vector-distance), [`fn.semantic_similarity()`](#semantic-similarity) or [`fn.bm25_score()`](#bm25-score).
    """
    ...

class TextExpr(Enum):
    """
    *Internal*

    Instances of the `TextExpr` class are used to represent text expressions in TopK.
    """

    def __and__(self, other: TextExpr) -> TextExpr:
        """
        Combine the expression with another text expression using the `&` operator.
        """
        ...
    def __rand__(self, other: TextExpr) -> TextExpr:
        """
        Combine the other text expression with the self expression using the `&` operator.
        """
        ...
    def __or__(self, other: TextExpr) -> TextExpr:
        """
        Combine the expression with another text expression using the `|` operator.
        """
        ...
    def __ror__(self, other: TextExpr) -> TextExpr:
        """
        Combine the other text expression with the self expression using the `|` operator.
        """
        ...


class Query:
    def select(
        self,
        *args: builtins.str,
        **kwargs: typing.Union[LogicalExpr, FunctionExpr],
    ) -> Query: ...
    def filter(self, expr: LogicalExpr | TextExpr) -> Query: ...
    def topk(
        self, expr: LogicalExpr, k: builtins.int, asc: builtins.bool = False
    ) -> Query: ...
    def rerank(
        self,
        model: typing.Optional[builtins.str] = None,
        query: typing.Optional[builtins.str] = None,
        fields: typing.Sequence[builtins.str] = [],
        topk_multiple: typing.Optional[builtins.int] = None,
    ) -> Query: ...
    def count(self) -> Query: ...


def field(name: builtins.str) -> LogicalExpr:
    """
    Select a field from the document.
    """
    ...


def select(
    *args: builtins.str,
    **kwargs: typing.Union[LogicalExpr, FunctionExpr],
) -> Query:
    """
    # Example:

    Create a select stage of a query.

    ```python
    # Example:

    from topk_sdk.query import select, field

    client.collection("books").query(
      select("title", year=field("published_year"))
    )
    ```
    """
...


def filter(expr: LogicalExpr | TextExpr) -> Query:
    """
    Create a filter stage of a query.

    ```python
    # Example:

    from topk_sdk.query import filter, field

    client.collection("books").query(
      filter(field("published_year") > 1980)
    )
    ```
    """
    ...


def literal(value: typing.Any) -> LogicalExpr:
    """
    Create a literal expression.
    """
    ...


def match(
    token: builtins.str,
    field: builtins.str | None = None,
    weight: builtins.float = 1.0,
    all: builtins.bool = False,
) -> LogicalExpr:
    """
    Perform a keyword search for documents that contain specific keywords or phrases.

    This function should be used in the filter stage of a query. You can configure
    the match() function to:
    - Match on multiple terms
    - Match only on specific fields
    - Use weights to prioritize certain terms
    """
...


def not_(expr: LogicalExpr) -> LogicalExpr:
    """
    Negate a logical expression.

    ```python
    # Example:

    from topk_sdk.query import field, not_

    .filter(
        not_(field("title").contains("Catcher"))
    )
    ```
    """
    ...


def abs(expr: LogicalExpr) -> LogicalExpr:
    """
    Compute the absolute value of a logical expression.

    ```python
    # Example:

    from topk_sdk.query import field, abs

    client.collection("books").query(
      filter(abs(field("rating")) > 4.5)
    )
    ```
    """


def all(exprs: typing.Sequence[LogicalExpr]) -> LogicalExpr:
    """
    Create a logical AND expression.

    ```python
    # Example:

    from topk_sdk.query import field, all

    client.collection("books").query(
      filter(all([
        field("published_year") >= 1900,
        field("published_year") <= 2000,
        field("title").is_not_null()
      ]))
    )
    ```
    """


def any(exprs: typing.Sequence[LogicalExpr]) -> LogicalExpr:
    """
    Create a logical OR expression.

    ```python
    # Example:

    from topk_sdk.query import field, any

    client.collection("books").query(
      filter(any([
        field("genre") == "fiction",
        field("genre") == "mystery",
        field("genre") == "thriller"
      ]))
    )
    ```
    """
    ...


def min(left: Ordered, right: Ordered) -> LogicalExpr:
    """
    Create a logical MIN expression.

    ```python
    # Example:

    from topk_sdk.query import field, min

    client.collection("books").query(
      filter(min(field("rating"), field("published_year")))
    )
    ```
    """
    ...


def max(left: Ordered, right: Ordered) -> LogicalExpr:
    """
    Create a logical MAX expression.

    ```python
    from topk_sdk.query import field, max

    client.collection("books").query(
      filter(max(field("rating"), field("published_year")))
    )
    ```
    """
    ...


class fn:
    """
    The `query.fn` submodule exposes functions for creating function expressions such as [`fn.vector_distance()`](#vector-distance), [`fn.semantic_similarity()`](#semantic-similarity) or [`fn.bm25_score()`](#bm25-score).
    """
    ...

    @staticmethod
    def vector_distance(
        field: builtins.str,
        vector: typing.Union[
            list[int],
            list[float],
            dict[int, float],
            dict[int, int],
            topk_sdk.data.SparseVector,
            topk_sdk.data.List,
        ],
        skip_refine: builtins.bool = False,
    ) -> FunctionExpr:
        """
        Calculate the vector distance between a field and a query vector.

        ```python
        # Example:

        from topk_sdk.query import field, fn, select

        client.collection("books").query(
          select(
            "title",
            title_similarity=fn.vector_distance(
              "title_embedding",
              [0.1, 0.2, 0.3, ...] # embedding for "animal"
            )
          )
          .topk(field("title_similarity"), 10)
        )
        ```
        """
        ...
    @staticmethod
    def semantic_similarity(
        field: builtins.str,
        query: builtins.str,
    ) -> FunctionExpr:
        """
        Calculate the semantic similarity between a field and a query string.

        ```python
        # Example:

        from topk_sdk.query import field, fn, select

        client.collection("books").query(
          select(
            "title",
            title_similarity=fn.semantic_similarity("title", "animal")
          )
          .topk(field("title_similarity"), 10)
        )
        ```
        """
        ...
    @staticmethod
    def bm25_score() -> FunctionExpr:
        """
        Calculate the BM25 score for a keyword search.

        ```python
        # Example:

        from topk_sdk.query import field, fn, select

        client.collection("books").query(
          select(
            "title",
            text_score=fn.bm25_score()
          )
          .filter(match("animal"))
          .topk(field("text_score"), 10)
        )
        ```
        """
        ...
