#!/usr/bin/env python3
"""Main script for generating API documentation from Python type stubs."""

import sys
from pathlib import Path

from parse import parse_module
from render import render_module


def generate_docs_for_file(input_file: Path, output_dir: Path, base_dir: Path):
    """Generate documentation for a single .pyi file."""
    # Calculate relative path from base directory
    rel_path = input_file.relative_to(base_dir)

    # Create output file path (replace .pyi with .md)
    # Use the parent folder name as the file name
    parent_folder_name = rel_path.parent.name if rel_path.parent.name else rel_path.stem
    output_file = output_dir / f"{parent_folder_name}.mdx"

    if rel_path == Path("__init__.pyi"):
        output_file = output_dir / "index.mdx"

    # Create output directory if it doesn't exist
    output_file.parent.mkdir(parents=True, exist_ok=True)

    # Parse the module
    module = parse_module(input_file)

    # Render to file instead of stdout
    with open(output_file, "w") as f:
        import sys
        from io import StringIO

        # Capture the output
        old_stdout = sys.stdout
        sys.stdout = StringIO()

        try:
            render_module(module)
            output = sys.stdout.getvalue()
        finally:
            sys.stdout = old_stdout

        f.write(output)

    print(f"Generated: {output_file}")


def main():
    """Main entry point."""
    if len(sys.argv) < 2 or len(sys.argv) > 3:
        print("Usage: python main.py <path_to_pyi_files> [output_directory]")
        print(
            "  path_to_pyi_files: Path to .pyi file or directory containing .pyi files"
        )
        print("  output_directory: Optional output directory (default: 'out')")
        sys.exit(1)

    input_path = Path(sys.argv[1])
    output_dir = Path(sys.argv[2]) if len(sys.argv) == 3 else Path("docs")

    if not input_path.exists():
        print(f"Error: Path {input_path} does not exist")
        sys.exit(1)

    # Find all .pyi files
    if input_path.is_file() and input_path.suffix == ".pyi":
        # Single file
        files = [input_path]
        base_dir = input_path.parent
    else:
        # Directory - find all .pyi files
        files = list(input_path.rglob("**/*.pyi"))
        base_dir = input_path

    if not files:
        print("No .pyi files found")
        sys.exit(1)

    print(f"Found {len(files)} .pyi files")
    print(f"Output directory: {output_dir}")
    print(f"Base directory: {base_dir}")
    print()

    # Generate documentation for each file
    for file in files:
        try:
            generate_docs_for_file(file, output_dir, base_dir)
        except Exception as e:
            print(f"Error processing {file}: {e}")
            continue

    print(f"\nDocumentation generation complete! Check the '{output_dir}' directory.")


if __name__ == "__main__":
    main()
