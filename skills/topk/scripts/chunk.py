#!/usr/bin/env python3
"""Split files into search-sized chunks, one JSON object per line, ready for `topk import`.

    python scripts/chunk.py docs/ > chunks.jsonl
    python scripts/chunk.py --root docs docs/**/*.md > chunks.jsonl
    python scripts/chunk.py --partition-from-dir tenants/ > chunks.jsonl

Each line is {"_id", "source", "chunk_index", "location", "text"}, plus "tenant" with
--partition-from-dir. `source` is the path relative to --root (default: the directory given,
or the current directory), so it is unique per file and stable across runs.

In a directory, only text files (.md, .mdx, .markdown, .txt, .rst, .adoc, .html, .htm), PDF and Word
files are read; others are listed on stderr, and dot-folders such as .git are skipped. Text files need
nothing else. PDF and Word files need
`pip install "pypdf>=5" "python-docx>=1.1"`. Pages or sections with no text (for example
scanned PDF pages) are listed on stderr: run OCR on them (e.g. `ocrmypdf`) and chunk again.
"""

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

CHUNK_CHARS = 3000  # well under the 32,768-character semantic_index() limit; one topic per chunk
OVERLAP_CHARS = 200
SUFFIXES = {".md", ".mdx", ".markdown", ".txt", ".rst", ".adoc", ".html", ".htm", ".pdf", ".docx"}
EDGE_LINES = 2  # PDF headers and footers sit in the first and last lines of a page


def chunk(body: str) -> list[str]:
    """Split on paragraph boundaries into chunks of at most CHUNK_CHARS."""
    chunks, current = [], ""
    for para in body.split("\n\n"):
        if len(para) > CHUNK_CHARS and current.strip():
            chunks.append(current)  # flush first, so chunks stay in document order
            current = ""
        while len(para) > CHUNK_CHARS:
            chunks.append(para[:CHUNK_CHARS])
            para = para[CHUNK_CHARS - OVERLAP_CHARS :]
        if current and len(current) + len(para) + 2 > CHUNK_CHARS:
            chunks.append(current)
            current = current[-OVERLAP_CHARS:]
        current = f"{current}\n\n{para}" if current else para
    if current.strip():
        chunks.append(current)
    return chunks


def shape(line: str) -> str:
    """Digits ignored, so "Page 3 of 9" and "Page 4 of 9" have the same shape."""
    return re.sub(r"\d+", "#", line.strip())


def pdf_units(path: Path) -> list[tuple[str, str]]:
    """[(location, text)] per page, with repeated headers and footers removed."""
    from pypdf import PdfReader

    pages = [(page.extract_text() or "").splitlines() for page in PdfReader(path).pages]

    def is_edge(lines: list[str], j: int) -> bool:
        return j < EDGE_LINES or j >= len(lines) - EDGE_LINES

    # Only edge lines with letters can be headers/footers, so numbers in tables are never stripped.
    counts = Counter(
        s
        for lines in pages
        for s in {shape(l) for j, l in enumerate(lines) if is_edge(lines, j) and re.search(r"[A-Za-z]", l)}
    )
    repeated = {s for s, n in counts.items() if len(pages) > 2 and n > len(pages) / 2}
    return [
        (f"page {i}", "\n".join(
            l for j, l in enumerate(lines) if not (is_edge(lines, j) and shape(l) in repeated)
        ).strip())
        for i, lines in enumerate(pages, 1)
    ]


def docx_units(path: Path) -> list[tuple[str, str]]:
    """[(heading, text)] per section; tables stay under their heading, one row per line."""
    from docx import Document

    units, heading, parts = [], "(before first heading)", []
    for block in Document(path).iter_inner_content():  # paragraphs and tables in document order
        if hasattr(block, "rows"):  # a table
            for row in block.rows:
                # A merged cell is returned once per column it spans; keep it once.
                cells = {id(cell._tc): cell.text.strip() for cell in row.cells}
                parts.append(" | ".join(cells.values()))
        elif (block.style.name if block.style else "").startswith(("Heading", "Title")):
            if parts:
                units.append((heading, "\n".join(parts)))
            heading, parts = block.text.strip(), []
        elif block.text.strip():
            parts.append(block.text.strip())
    if parts:
        units.append((heading, "\n".join(parts)))
    return units


def units(path: Path) -> list[tuple[str, str]]:
    suffix = path.suffix.lower()
    if suffix == ".pdf":
        return pdf_units(path)
    if suffix == ".docx":
        return docx_units(path)
    return [("", path.read_text(errors="replace"))]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("paths", nargs="+", type=Path, help="files or directories (searched recursively)")
    parser.add_argument("--root", type=Path, help="make `source` relative to this directory")
    parser.add_argument(
        "--partition-from-dir",
        action="store_true",
        help="add a `tenant` field from each file's first directory under --root (tenants/acme/x.md -> acme)",
    )
    args = parser.parse_args()
    root = args.root or (args.paths[0] if len(args.paths) == 1 and args.paths[0].is_dir() else Path("."))

    def walk(directory: Path):
        for f in directory.rglob("*"):
            if not f.is_file() or any(part.startswith(".") for part in f.relative_to(directory).parts):
                continue
            if f.suffix.lower() in SUFFIXES:
                yield f
            else:
                print(f"{f}: skipped (not a text, PDF or Word file)", file=sys.stderr)

    # A file named on the command line is read as given.
    files = sorted(f for p in args.paths for f in ([p] if p.is_file() else walk(p)))
    out = sys.stdout
    for path in files:
        source = path.relative_to(root).as_posix() if path.is_relative_to(root) else path.as_posix()
        doc = {"source": source}
        if args.partition_from_dir:
            tenant, _, rest = source.partition("/")
            if not rest:
                sys.exit(f"{source}: --partition-from-dir needs files under <root>/<tenant>/")
            doc["tenant"] = tenant
        empty = []
        index = 0
        for n, (location, text) in enumerate(units(path)):
            if not text.strip():
                empty.append(location or source)
                continue
            for part in chunk(text):
                # Number units and chunks: two sections can share a heading, so it can't be the id.
                out.write(json.dumps({"_id": f"{source}#{n}#{index}", **doc, "chunk_index": index,
                                      "location": location, "text": part}) + "\n")
                index += 1
        if empty:
            print(f"{source}: no text in {', '.join(empty)} (scanned? run OCR)", file=sys.stderr)


if __name__ == "__main__":
    main()
