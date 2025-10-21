"""Renderer module for converting parsed data structures to markdown."""

from typing import List

from parse import (
    Class,
    EnumValue,
    Function,
    Method,
    Module,
    Parameter,
    Property,
    TypeAlias,
    TypeAnnotation,
)

import re

from urllib.parse import urlparse

ESCAPED_UNION_DELIMITER = " &#124; " # &#124; is the HTML entity for | to escape the pipe character when rendering to markdown table

def format_type_annotation_str(
    type_annotation: TypeAnnotation, with_links: bool = True, union_delimiter: str = " | "
) -> str:
    """Format a TypeAnnotation to a string."""
    if type_annotation.is_generic:
        if type_annotation.generic_args:
            if type_annotation.name == "Union":
                # Format union types as A | B instead of Union[A, B]
                args_str = union_delimiter.join(
                    format_type_annotation_str(arg, with_links, union_delimiter)
                    for arg in type_annotation.generic_args
                )
                return args_str
            else:
                args_str = ", ".join(
                    format_type_annotation_str(arg, with_links, union_delimiter)
                    for arg in type_annotation.generic_args
                )
                base = f"{type_annotation.name}[{args_str}]"
        else:
            base = type_annotation.name
    else:
        base = type_annotation.name

    if with_links and should_link_type(base):
        return create_type_link(base)
    else:
        return base


def should_link_type(type_str: str) -> bool:
    """Check if a type should be linked."""
    # Skip generic types
    if any(
        generic in type_str
        for generic in [
            "[",
            "]",
            "|",
            "Union",
            "Optional",
            "Sequence",
            "Mapping",
            "Awaitable",
        ]
    ):
        return False

    # Skip built-in types
    builtin_types = {
        "str",
        "int",
        "float",
        "bool",
        "None",
        "Any",
        "dict",
        "list",
        "tuple",
        "set",
    }
    if type_str in builtin_types:
        return False

    # Link topk_sdk module references (both full and shortened paths)
    if type_str.startswith("topk_sdk."):
        return True

    # Link shortened module references (data., query., schema., error.)
    if any(
        type_str.startswith(prefix)
        for prefix in ["data.", "query.", "schema.", "error."]
    ):
        return True

    # Only link if it looks like a class name (starts with capital letter)
    return bool(type_str and type_str[0].isupper())


def create_type_link(type_str: str) -> str:
    """Create a markdown link for a type."""
    # Handle cross-module references
    if "." in type_str:
        parts = type_str.split(".")

        # Handle full module path: topk_sdk.module.Class
        if type_str.startswith("topk_sdk.") and len(parts) >= 3:
            module_name = parts[1]  # e.g., "data"
            class_name = parts[-1]  # e.g., "SparseVector"
            return f"[`{type_str}`](/sdk/topk-py/{module_name}#{class_name.lower()})"

        # Handle shortened module path: module.Class (data.SparseVector, query.Expr, etc.)
        elif len(parts) == 2 and parts[0] in ["data", "query", "schema", "error"]:
            module_name = parts[0]  # e.g., "data"
            class_name = parts[1]  # e.g., "SparseVector"
            return f"[`{type_str}`](/sdk/topk-py/{module_name}#{class_name.lower()})"

        # Handle main module: topk_sdk.Class
        elif type_str.startswith("topk_sdk.") and len(parts) == 2:
            class_name = parts[1]
            return f"[`{type_str}`](/sdk/topk-py/index#{class_name.lower()})"

    # For local references, create anchor link
    link = type_str.replace(".", "#").lower()
    return f"[`{type_str}`](#{link})"


def format_function_signature(
    method: Method, class_name: str = "", with_links: bool = True
) -> str:
    """Format a method signature."""
    args: List[str] = []
    for param in method.parameters:
        arg_str = param.name
        if param.type_annotation:
            type_str = format_type_annotation_str(param.type_annotation, with_links)
            arg_str += f": {type_str}"
        if param.default_value:
            arg_str += f" = {param.default_value}"
        args.append(arg_str)

    # For constructors, format more nicely
    if method.is_constructor:
        # Remove 'self' from constructor signature
        args = args[1:] if args and args[0].startswith("self") else args
        signature = f"{class_name or 'Class'}({', '.join(args)})"
    else:
        signature = f"{method.name}({', '.join(args)})"

    if method.return_type:
        return_type_str = format_type_annotation_str(method.return_type, with_links)
        signature += f" -> {return_type_str}"

    # Format as multi-line if signature is long
    if len(signature) > 100 and len(args) > 2:
        if method.is_constructor:
            base = f"{class_name or 'Class'}("
        else:
            base = f"{method.name}("

        lines = [base]
        for i, arg in enumerate(args):
            if i == 0:
                lines.append(f"   {arg},")
            elif i == len(args) - 1:
                lines.append(f"   {arg}")
            else:
                lines.append(f"   {arg},")
        lines.append(")")
        signature = "\n".join(lines)

    return signature


def render_enum_values(enum_values: List[EnumValue]):
    """Render enum values as a table."""
    print("**Values**")
    print()
    print("| Value | Description |")
    print("| ----- | ----------- |")
    for value in enum_values:
        print(f"| `{value.name}` | `{value.value}` |")


def render_data_class_properties(properties: List[Property]):
    """Render data class properties as a table."""
    print("**Properties**")
    print()
    print("| Property | Type |             |")
    print("| -------- | ---- | ----------- |")
    for prop in properties:
        type_str = format_type_annotation_str(prop.type_annotation, with_links=True, union_delimiter=ESCAPED_UNION_DELIMITER)
        docstring_str = transform_docstring(prop.docstring or "")
        print(f"| `{prop.name}` | {type_str} | {docstring_str} |")

    print()

def render_constructor(method: Method, class_name: str):
    """Render constructor method."""
    print("**Constructor**")
    print()
    print("```python")
    signature = format_function_signature(method, class_name, with_links=False)
    print(signature)
    print("```")
    print()
    if method.docstring:
        print(transform_docstring(method.docstring))
        print()

    # Add Parameters section
    if method.parameters and not (
        len(method.parameters) == 1 and method.parameters[0].name == "self"
    ):
        render_parameters(method.parameters)

def render_method(method: Method):
    """Render a regular method."""

    escaped_name = method.name.replace("_", r"\_")
    print(f"#### {escaped_name}()")
    print()

    print("```python")
    signature = format_function_signature(method, with_links=False)
    print(signature)
    print("```")
    print()

    if method.docstring:
        print(transform_docstring(method.docstring))
        print()

    # Add Parameters section
    if method.parameters and not (
        len(method.parameters) == 1 and method.parameters[0].name == "self"
    ):
        render_parameters(method.parameters)

    # Add Returns section
    if method.return_type:
        return_type_str = format_type_annotation_str(
            method.return_type, with_links=True
        )
        print("**Returns**")
        print()
        print(return_type_str)
        print()

    print("***")
    print()

def render_parameters(parameters: List[Parameter]):
    """Render parameters as a table."""
    print("**Parameters**")
    print()
    print("| Parameter | Type |")
    print("| --------- | ---- |")
    for param in parameters:
        if param.name == "self":
            continue
        type_str = (
            format_type_annotation_str(param.type_annotation, with_links=True, union_delimiter=ESCAPED_UNION_DELIMITER)
            if param.type_annotation
            else "Any"
        )
        print(f"| `{param.name}` | {type_str} |")
    print()


def render_class_methods(methods: List[Method], class_name: str):
    """Render all methods in a class."""
    print("**Methods**")
    print()

    for method in methods:
        if method.is_constructor:
            render_constructor(method, class_name)
        else:
            render_method(method)


def render_class(cls: Class, file_path):
    """Render a complete class."""
    print(f"### {cls.name}")
    print()

    if cls.docstring:
        print(transform_docstring(cls.docstring))
        print()

    if cls.is_enum and cls.enum_values:
        render_enum_values(cls.enum_values)

    if cls.properties:
        render_data_class_properties(cls.properties)

    if cls.methods:
        render_class_methods(cls.methods, cls.name)


def render_function(func: Function, file_path):
    """Render a standalone function."""
    print(f"### {func.name}()")
    print()

    print("```python")
    # Create a temporary method object for formatting
    temp_method = Method(
        name=func.name,
        parameters=func.parameters,
        return_type=func.return_type,
        docstring=func.docstring,
    )
    signature = format_function_signature(temp_method, with_links=False)
    print(signature)
    print("```")
    print()

    if func.docstring:
        print(transform_docstring(func.docstring))
        print()

    # Add Parameters section
    if func.parameters:
        print("**Parameters**")
        print()
        print("| Parameter | Type |")
        print("| --------- | ---- |")
        for param in func.parameters:
            type_str = (
                format_type_annotation_str(param.type_annotation, with_links=True, union_delimiter=ESCAPED_UNION_DELIMITER)
                if param.type_annotation
                else "Any"
            )
            print(f"| `{param.name}` | {type_str} |")
        print()

    # Add Returns section
    if func.return_type:
        return_type_str = format_type_annotation_str(func.return_type, with_links=True)
        print("**Returns**")
        print()
        print(return_type_str)
        print()

    print("***")
    print()


def render_type_alias(type_alias: TypeAlias, file_path):
    """Render a type alias."""
    print(f"### {type_alias.name}")
    print()

    if type_alias.docstring:
        print(transform_docstring(type_alias.docstring))
        print()

    print("```python")
    type_str = format_type_annotation_str(type_alias.type_annotation, with_links=False)
    print(f"{type_alias.name} = {type_str}")
    print("```")
    print()

    print("**Type**")
    print()
    type_str_with_links = format_type_annotation_str(
        type_alias.type_annotation, with_links=True
    )
    print(type_str_with_links)
    print()

    print("***")
    print()


def render_module(module: Module):
    """Render a complete module."""
    print("---")

    if module.file_path.parent.name == "topk_sdk":
        print("title: topk_sdk")
    else:
        print(f"title: topk_sdk.{module.file_path.parent.name}")

    print("---")
    print()

    print("## Classes")
    print()

    for cls in module.classes:
        render_class(cls, module.file_path)

    if module.functions.__len__() > 0:
        print("## Functions")
        print()

        for func in module.functions:
            render_function(func, module.file_path)

    if module.type_aliases and module.type_aliases.__len__() > 0:
        print("## Type Aliases")
        print()

        for type_alias in module.type_aliases:
            render_type_alias(type_alias, module.file_path)

def transform_docstring(docstring: str) -> str:
    return rewrite_doc_links(docstring)

def rewrite_doc_links(docstring: str, base_domain: str = "docs.topk.io") -> str:
    """
    Rewrites full URLs pointing to the given base domain into relative Markdown links.

    Example:
        Input:  "See https://docs.topk.io/regions for details."
        Output: "See [regions](/regions) for details."

    Args:
        text: The input docstring or text.
        base_domain: The domain to match (default: 'docs.topk.io').

    Returns:
        The text with rewritten links.
    """
    def replace_link(match: re.Match[str]) -> str:
        url = match.group(0)

        parsed = urlparse(url)
        if parsed.netloc != base_domain:
            return url  # leave other domains untouched

        # Check if the URL is already inside parentheses (part of a markdown link)
        start_pos = match.start()
        if start_pos > 0 and docstring[start_pos - 1] == '(':
            # For links inside parentheses, just return the relative path with fragment
            path = parsed.path.rstrip('/')
            if parsed.fragment:
                path += f"#{parsed.fragment}"
            return path

        # Extract just the path (e.g., "/regions") and use the last segment as link text
        path = parsed.path.rstrip('/')
        if parsed.fragment:
            path += f"#{parsed.fragment}"
        link_text = path.split('/')[-1].split('#')[0] or path
        return f"[{link_text}]({path})"

    return re.sub(r"https?://[^\s)]+", replace_link, docstring)
