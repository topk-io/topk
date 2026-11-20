#!/usr/bin/env python3
"""Render PDF pages to JPEG images, one JSON object per line, ready for `topk import`.

    pip install "pypdfium2>=4" "pillow>=10"
    python scripts/pdf_pages.py handbook.pdf scans/ > pages.jsonl
    topk import ./pages.jsonl --to handbook_pages -f source=keyword,image=image:semantic --yes

Each line is {"_id", "source", "page", "image"}, where `image` is a JPEG data URI. A semantic
index on an image field embeds the page as a picture, so scanned pages, charts, forms and
multi-column layouts are searchable without OCR or text extraction. Search it with
`semantic_similarity(image, 'question')`; BM25 (`match`, `should`) doesn't work on images.
"""

import argparse
import base64
import io
import json
import sys
from pathlib import Path

MAX_IMAGE_BYTES = 500_000  # under TopK's 512KB per image
SCALE = 1.5  # about 108 DPI: readable text, small files
QUALITIES = (80, 65, 50, 35)


def jpeg(image) -> bytes | None:
    """The best-quality JPEG under MAX_IMAGE_BYTES, or None if even the lowest is too big."""
    for quality in QUALITIES:
        buf = io.BytesIO()
        image.save(buf, "JPEG", quality=quality)
        if buf.tell() <= MAX_IMAGE_BYTES:
            return buf.getvalue()
    return None


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("paths", nargs="+", type=Path, help="PDF files or directories (searched recursively)")
    parser.add_argument("--root", type=Path, help="make `source` relative to this directory")
    args = parser.parse_args()

    import pypdfium2 as pdfium

    root = args.root or (args.paths[0] if len(args.paths) == 1 and args.paths[0].is_dir() else Path("."))
    files = sorted(
        f
        for p in args.paths
        for f in ([p] if p.is_file() else p.rglob("*"))
        if f.is_file() and f.suffix.lower() == ".pdf"
    )
    for path in files:
        source = path.relative_to(root).as_posix() if path.is_relative_to(root) else path.as_posix()
        for number, page in enumerate(pdfium.PdfDocument(path), 1):
            data = jpeg(page.render(scale=SCALE).to_pil().convert("RGB"))
            if data is None:
                print(f"{source}: page {number} is over {MAX_IMAGE_BYTES} bytes even at low quality; skipped",
                      file=sys.stderr)
                continue
            uri = "data:image/jpeg;base64," + base64.b64encode(data).decode()
            print(json.dumps({"_id": f"{source}#{number}", "source": source, "page": number, "image": uri}))


if __name__ == "__main__":
    main()
