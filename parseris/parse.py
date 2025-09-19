"""Parser module for extracting information from Python AST nodes."""

import ast
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional, Union


@dataclass
class TypeAnnotation:
    """Represents a type annotation."""

    name: str
    is_optional: bool = False
    is_generic: bool = False
    generic_args: List["TypeAnnotation"] = None

    def __post_init__(self):
        if self.generic_args is None:
            self.generic_args = []


@dataclass
class Parameter:
    """Represents a function parameter."""

    name: str
    type_annotation: Optional[TypeAnnotation]
    default_value: Optional[str] = None
    is_required: bool = True


@dataclass
class Method:
    """Represents a class method."""

    name: str
    parameters: List[Parameter]
    return_type: Optional[TypeAnnotation]
    docstring: Optional[str] = None
    is_constructor: bool = False


@dataclass
class Property:
    """Represents a class property/attribute."""

    name: str
    type_annotation: TypeAnnotation


@dataclass
class EnumValue:
    """Represents an enum value."""

    name: str
    value: str


@dataclass
class Class:
    """Represents a class definition."""

    name: str
    docstring: Optional[str]
    is_enum: bool = False
    is_data_class: bool = False
    methods: List[Method] = None
    properties: List[Property] = None
    enum_values: List[EnumValue] = None

    def __post_init__(self):
        if self.methods is None:
            self.methods = []
        if self.properties is None:
            self.properties = []
        if self.enum_values is None:
            self.enum_values = []


@dataclass
class Function:
    """Represents a standalone function."""

    name: str
    parameters: List[Parameter]
    return_type: Optional[TypeAnnotation]
    docstring: Optional[str] = None


@dataclass
class Module:
    """Represents a parsed module."""

    file_path: Path
    classes: List[Class]
    functions: List[Function]


def format_type_annotation(annotation) -> TypeAnnotation:
    """Convert AST type annotation to TypeAnnotation object."""
    if annotation is None:
        return TypeAnnotation("None")

    if isinstance(annotation, ast.Name):
        # Clean up builtins references
        if annotation.id.startswith("builtins."):
            name = annotation.id.replace("builtins.", "")
        else:
            name = annotation.id
        return TypeAnnotation(name)

    elif isinstance(annotation, ast.Constant):
        return TypeAnnotation(repr(annotation.value))

    elif isinstance(annotation, ast.Attribute):
        # Clean up builtins references
        if isinstance(annotation.value, ast.Name):
            if annotation.value.id == "builtins":
                name = annotation.attr
            elif annotation.value.id == "typing":
                name = annotation.attr
            else:
                name = f"{annotation.value.id}.{annotation.attr}"
        else:
            # Handle nested attributes recursively
            value_name = format_type_annotation(annotation.value).name
            name = f"{value_name}.{annotation.attr}"
        return TypeAnnotation(name)

    elif isinstance(annotation, ast.Subscript):
        # Handle things like List[str], Optional[int], etc.
        if isinstance(annotation.value, ast.Name):
            base = annotation.value.id
        elif isinstance(annotation.value, ast.Attribute):
            # Handle nested attributes like typing.Union, builtins.str, etc.
            if isinstance(annotation.value.value, ast.Name):
                if annotation.value.value.id == "builtins":
                    base = annotation.value.attr
                elif annotation.value.value.id == "typing":
                    base = annotation.value.attr
                else:
                    base = f"{annotation.value.value.id}.{annotation.value.attr}"
            else:
                # Handle deeper nesting like typing.Union
                base = f"{annotation.value.value.id}.{annotation.value.attr}"
        else:
            base = "Unknown"

        is_optional = base == "Optional"

        if isinstance(annotation.slice, ast.Tuple):
            args = [format_type_annotation(el) for el in annotation.slice.elts]
            return TypeAnnotation(
                base, is_optional=is_optional, is_generic=True, generic_args=args
            )
        else:
            arg = format_type_annotation(annotation.slice)
            return TypeAnnotation(
                base, is_optional=is_optional, is_generic=True, generic_args=[arg]
            )

    else:
        return TypeAnnotation("Unknown")


def extract_docstring(node) -> Optional[str]:
    """Extract docstring from a node."""
    if (
        isinstance(node, (ast.FunctionDef, ast.ClassDef))
        and node.body
        and isinstance(node.body[0], ast.Expr)
        and isinstance(node.body[0].value, ast.Constant)
        and isinstance(node.body[0].value.value, str)
    ):
        return node.body[0].value.value.strip()
    return None


def parse_parameter(arg, default_value=None) -> Parameter:
    """Parse a function parameter."""
    param_name = arg.arg
    type_annotation = None

    if arg.annotation:
        type_annotation = format_type_annotation(arg.annotation)

    is_required = default_value is None

    return Parameter(
        name=param_name,
        type_annotation=type_annotation,
        default_value=default_value,
        is_required=is_required,
    )


def parse_method(method_node) -> Method:
    """Parse a method definition."""
    parameters = []
    defaults = method_node.args.defaults
    default_args = method_node.args.args[-len(defaults) :] if defaults else []

    for i, arg in enumerate(method_node.args.args):
        default_val = None
        if i >= len(method_node.args.args) - len(defaults):
            default_idx = i - (len(method_node.args.args) - len(defaults))
            if default_idx < len(defaults):
                default = defaults[default_idx]
                if isinstance(default, ast.Constant):
                    if isinstance(default.value, str):
                        default_val = f'"{default.value}"'
                    else:
                        default_val = repr(default.value)
                elif isinstance(default, ast.NameConstant):
                    default_val = repr(default.value)
                elif isinstance(default, ast.Str):  # Python 3.7 compatibility
                    default_val = f'"{default.s}"'
                elif isinstance(default, ast.Name):
                    default_val = default.id
                else:
                    default_val = "..."

        param = parse_parameter(arg, default_val)
        parameters.append(param)

    return_type = None
    if method_node.returns:
        return_type = format_type_annotation(method_node.returns)

    docstring = extract_docstring(method_node)

    return Method(
        name=method_node.name,
        parameters=parameters,
        return_type=return_type,
        docstring=docstring,
        is_constructor=(method_node.name == "__init__"),
    )


def parse_class(class_node) -> Class:
    """Parse a class definition."""
    name = class_node.name
    docstring = extract_docstring(class_node)

    # Check if it's an enum
    is_enum = any(
        isinstance(sub, ast.Assign)
        and isinstance(sub.value, ast.Constant)
        and isinstance(sub.value.value, str)
        for sub in class_node.body
    )

    if is_enum:
        enum_values = []
        for sub_node in class_node.body:
            if isinstance(sub_node, ast.Assign) and len(sub_node.targets) == 1:
                if isinstance(sub_node.targets[0], ast.Name) and isinstance(
                    sub_node.value, ast.Constant
                ):
                    enum_values.append(
                        EnumValue(
                            name=sub_node.targets[0].id, value=sub_node.value.value
                        )
                    )
        return Class(
            name=name, docstring=docstring, is_enum=True, enum_values=enum_values
        )

    # Check if it's a data class (has attributes but no methods)
    methods = [sub for sub in class_node.body if isinstance(sub, ast.FunctionDef)]
    attributes = [sub for sub in class_node.body if isinstance(sub, ast.AnnAssign)]

    if attributes and not methods:
        properties = []
        for attr in attributes:
            if isinstance(attr.target, ast.Name):
                type_annotation = (
                    format_type_annotation(attr.annotation)
                    if attr.annotation
                    else TypeAnnotation("Any")
                )
                properties.append(
                    Property(name=attr.target.id, type_annotation=type_annotation)
                )

        return Class(
            name=name, docstring=docstring, is_data_class=True, properties=properties
        )

    # Regular class with methods
    parsed_methods = []
    for method_node in methods:
        # Skip __new__ methods for data classes
        if method_node.name == "__new__" and len(methods) == 1:
            continue
        parsed_methods.append(parse_method(method_node))

    return Class(name=name, docstring=docstring, methods=parsed_methods)


def parse_function(func_node) -> Function:
    """Parse a standalone function."""
    parameters = []
    defaults = func_node.args.defaults
    default_args = func_node.args.args[-len(defaults) :] if defaults else []

    for i, arg in enumerate(func_node.args.args):
        default_val = None
        if i >= len(func_node.args.args) - len(defaults):
            default_idx = i - (len(func_node.args.args) - len(defaults))
            if default_idx < len(defaults):
                default = defaults[default_idx]
                if isinstance(default, ast.Constant):
                    if isinstance(default.value, str):
                        default_val = f'"{default.value}"'
                    else:
                        default_val = repr(default.value)
                elif isinstance(default, ast.NameConstant):
                    default_val = repr(default.value)
                elif isinstance(default, ast.Str):  # Python 3.7 compatibility
                    default_val = f'"{default.s}"'
                elif isinstance(default, ast.Name):
                    default_val = default.id
                else:
                    default_val = "..."

        param = parse_parameter(arg, default_val)
        parameters.append(param)

    return_type = None
    if func_node.returns:
        return_type = format_type_annotation(func_node.returns)

    docstring = extract_docstring(func_node)

    return Function(
        name=func_node.name,
        parameters=parameters,
        return_type=return_type,
        docstring=docstring,
    )


def parse_module(file_path: Path) -> Module:
    """Parse a Python module file."""
    code = file_path.read_text()
    tree = ast.parse(code, filename=file_path.name)

    classes = []
    functions = []

    for node in tree.body:
        if isinstance(node, ast.ClassDef):
            classes.append(parse_class(node))
        elif isinstance(node, ast.FunctionDef):
            functions.append(parse_function(node))

    return Module(file_path=file_path, classes=classes, functions=functions)
