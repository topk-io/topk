import sys

import pytest
from PIL import Image
from topk_sdk import data
from topk_sdk.data import image as data_image
from topk_sdk.query import field, select
from topk_sdk.schema import image

from . import AsyncProjectContext, ProjectContext
from .utils.dataset import IMAGES


def test_image_round_trip(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"image": image()},
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "jpeg_bytes", "image": data_image((IMAGES / "cat.jpg").read_bytes())},
            {"_id": "png_bytes", "image": data_image((IMAGES / "form.png").read_bytes())},
            {"_id": "jpeg_pil", "image": Image.open(IMAGES / "trafficjam.jpg")},
            {"_id": "png_pil", "image": Image.open(IMAGES / "form.png")},
            {"_id": "in_memory", "image": Image.new("RGBA", (4, 2), (255, 0, 0, 128))},
        ]
    )

    docs = ctx.client.collection(collection.name).get(
        ["jpeg_bytes", "png_bytes", "jpeg_pil", "png_pil", "in_memory"], lsn=lsn
    )

    for doc in docs.values():
        assert isinstance(doc["image"], data.Image)

    # Encoded bytes are stored as-is
    jpeg_bytes = docs["jpeg_bytes"]["image"]
    assert jpeg_bytes.format == "jpeg"
    assert jpeg_bytes.data == (IMAGES / "cat.jpg").read_bytes()

    png_bytes = docs["png_bytes"]["image"]
    assert png_bytes.format == "png"
    assert png_bytes.data == (IMAGES / "form.png").read_bytes()

    # JPEGs stay JPEG and keep their metadata
    original = Image.open(IMAGES / "trafficjam.jpg")
    jpeg_pil = docs["jpeg_pil"]["image"]
    assert jpeg_pil.format == "jpeg"
    decoded = jpeg_pil.to_pil()
    assert decoded.size == original.size
    assert decoded.info["icc_profile"] == original.info["icc_profile"]
    assert decoded.getexif() == original.getexif()

    # PNGs are lossless
    png_pil = docs["png_pil"]["image"]
    assert png_pil.format == "png"
    assert png_pil.to_pil().tobytes() == Image.open(IMAGES / "form.png").tobytes()

    # In-memory images are encoded as PNG
    in_memory = docs["in_memory"]["image"]
    assert in_memory.format == "png"
    assert in_memory.to_pil().mode == "RGBA"
    assert in_memory.to_pil().getpixel((0, 0)) == (255, 0, 0, 128)


def test_image_query(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"image": image()},
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [
            {"_id": "cat", "rank": 1, "image": Image.open(IMAGES / "cat.jpg")},
            {"_id": "form", "rank": 2, "image": Image.open(IMAGES / "form.png")},
        ]
    )

    # Images can't be selected, they are fetched by the ids returned from the query
    result = ctx.client.collection(collection.name).query(
        select("rank").sort(field("rank"), True).limit(10), lsn=lsn
    )
    ids = [doc["_id"] for doc in result]
    assert ids == ["cat", "form"]

    docs = ctx.client.collection(collection.name).get(ids, lsn=lsn)
    assert [docs[id]["image"].format for id in ids] == ["jpeg", "png"]


def test_image_get_update_upsert(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"image": image()},
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "traffic", "title": "Traffic", "image": Image.open(IMAGES / "trafficjam.jpg")}]
    )
    doc = ctx.client.collection(collection.name).get(["traffic"], lsn=lsn)["traffic"]

    # Images read from TopK are written back without re-encoding
    lsn = ctx.client.collection(collection.name).upsert([{**doc, "title": "Updated"}])
    updated = ctx.client.collection(collection.name).get(["traffic"], lsn=lsn)["traffic"]

    assert updated["title"] == "Updated"
    assert updated["image"].format == "jpeg"
    assert updated["image"].data == doc["image"].data


def test_image_corrupt(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"image": image()},
    )

    corrupt = b"\xff\xd8\xff" + b"\x00" * 64
    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "corrupt", "title": "Corrupt", "image": data_image(corrupt)}]
    )

    # Images are only decoded on request, so reads aren't affected
    docs = ctx.client.collection(collection.name).get(["corrupt"], lsn=lsn)

    assert docs["corrupt"]["title"] == "Corrupt"
    assert docs["corrupt"]["image"].data == corrupt
    with pytest.raises(OSError):
        docs["corrupt"]["image"].to_pil()


def test_image_without_pillow(ctx: ProjectContext, monkeypatch: pytest.MonkeyPatch):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"image": image()},
    )

    png = (IMAGES / "form.png").read_bytes()
    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "form", "title": "Form", "image": data_image(png)}]
    )

    # Make `import PIL.Image` fail
    monkeypatch.setitem(sys.modules, "PIL.Image", None)

    docs = ctx.client.collection(collection.name).get(["form"], lsn=lsn)

    assert docs["form"]["title"] == "Form"
    assert docs["form"]["image"].data == png
    with pytest.raises(ImportError, match="pip install pillow"):
        docs["form"]["image"].to_pil()


@pytest.mark.asyncio
async def test_async_image_round_trip(async_ctx: AsyncProjectContext):
    collection = await async_ctx.client.collections().create(
        async_ctx.scope("test"),
        schema={"image": image()},
    )

    lsn = await async_ctx.client.collection(collection.name).upsert(
        [{"_id": "cat", "image": Image.open(IMAGES / "cat.jpg")}]
    )
    docs = await async_ctx.client.collection(collection.name).get(["cat"], lsn=lsn)

    assert docs["cat"]["image"].format == "jpeg"
    assert docs["cat"]["image"].to_pil().size == (474, 474)
