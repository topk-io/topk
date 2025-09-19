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
    TypeAnnotation,
)


def format_type_annotation_str(
    type_annotation: TypeAnnotation, with_links: bool = True
) -> str:
    """Format a TypeAnnotation to a string."""
    if type_annotation.is_generic:
        if type_annotation.generic_args:
            args_str = ", ".join(
                format_type_annotation_str(arg, with_links)
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

    # Only link if it looks like a class name (starts with capital letter)
    return type_str and type_str[0].isupper()


def create_type_link(type_str: str) -> str:
    """Create a markdown link for a type."""
    link = type_str.replace(".", "#").lower()
    return f"[`{type_str}`](#{link})"


def format_function_signature(
    method: Method, class_name: str = None, with_links: bool = True
) -> str:
    """Format a method signature."""
    args = []
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
    print("| Property | Type |")
    print("| -------- | ---- |")
    for prop in properties:
        type_str = format_type_annotation_str(prop.type_annotation, with_links=True)
        print(f"| `{prop.name}` | {type_str} |")


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
        print(method.docstring)
        print()


def render_method(method: Method):
    """Render a regular method."""
    print(f"##### {method.name}()")
    print()
    print("```python")
    signature = format_function_signature(method, with_links=False)
    print(signature)
    print("```")
    print()

    # Add Parameters section
    if method.parameters and not (
        len(method.parameters) == 1 and method.parameters[0].name == "self"
    ):
        print("**Parameters**")
        print()
        print("| Parameter | Type |")
        print("| --------- | ---- |")
        for param in method.parameters:
            if param.name == "self":
                continue
            type_str = (
                format_type_annotation_str(param.type_annotation, with_links=True)
                if param.type_annotation
                else "Any"
            )
            print(f"| `{param.name}` | {type_str} |")
        print()

    # Add Returns section
    if method.return_type:
        return_type_str = format_type_annotation_str(
            method.return_type, with_links=True
        )
        print("**Returns**")
        print()
        print(return_type_str)
        print()

    if method.docstring:
        print(method.docstring)
        print()
    print("***")
    print()


def render_class_methods(methods: List[Method], class_name: str):
    """Render all methods in a class."""
    print("**Methods**")
    print()
    for method in methods:
        # Skip __new__ methods for data classes
        if method.name == "__new__" and len(methods) == 1:
            continue

        if method.is_constructor:
            render_constructor(method, class_name)
        else:
            render_method(method)


def render_class(cls: Class, file_path):
    """Render a complete class."""
    print(f"### {cls.name}")
    print()
    print(f"Defined in: [{file_path.name}]({file_path})")
    print()

    if cls.docstring:
        print(cls.docstring)
        print()

    if cls.is_enum:
        render_enum_values(cls.enum_values)
    elif cls.is_data_class:
        render_data_class_properties(cls.properties)
    elif cls.methods:
        render_class_methods(cls.methods, cls.name)


def render_function(func: Function, file_path):
    """Render a standalone function."""
    print(f"### {func.name}")
    print()
    print(f"Defined in: [{file_path.name}]({file_path})")
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

    # Add Parameters section
    if func.parameters:
        print("**Parameters**")
        print()
        print("| Parameter | Type |")
        print("| --------- | ---- |")
        for param in func.parameters:
            type_str = (
                format_type_annotation_str(param.type_annotation, with_links=True)
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

    if func.docstring:
        print(func.docstring)
        print()
    print("***")
    print()


def render_module(module: Module):
    """Render a complete module."""
    print("---")
    print("title: topk-py")
    print("---")
    print()

    print("## Classes")
    print()

    for cls in module.classes:
        render_class(cls, module.file_path)

    for func in module.functions:
        render_function(func, module.file_path)
