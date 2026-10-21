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
        assert isinstance(doc["image"], Image.Image)

    # Encoded bytes are stored as-is
    jpeg_bytes = docs["jpeg_bytes"]["image"]
    assert jpeg_bytes.format == "JPEG"
    assert jpeg_bytes.tobytes() == Image.open(IMAGES / "cat.jpg").tobytes()

    png_bytes = docs["png_bytes"]["image"]
    assert png_bytes.format == "PNG"
    assert png_bytes.tobytes() == Image.open(IMAGES / "form.png").tobytes()

    # JPEGs stay JPEG and keep their metadata
    original = Image.open(IMAGES / "trafficjam.jpg")
    jpeg_pil = docs["jpeg_pil"]["image"]
    assert jpeg_pil.format == "JPEG"
    assert jpeg_pil.size == original.size
    assert jpeg_pil.info["icc_profile"] == original.info["icc_profile"]
    assert jpeg_pil.getexif() == original.getexif()

    # PNGs are lossless
    png_pil = docs["png_pil"]["image"]
    assert png_pil.format == "PNG"
    assert png_pil.tobytes() == Image.open(IMAGES / "form.png").tobytes()

    # In-memory images are encoded as PNG
    in_memory = docs["in_memory"]["image"]
    assert in_memory.format == "PNG"
    assert in_memory.mode == "RGBA"
    assert in_memory.getpixel((0, 0)) == (255, 0, 0, 128)


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

    result = ctx.client.collection(collection.name).query(
        select("image").sort(field("rank"), True).limit(10), lsn=lsn
    )

    assert [doc["_id"] for doc in result] == ["cat", "form"]
    assert [doc["image"].format for doc in result] == ["JPEG", "PNG"]


def test_image_reupsert(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"image": image()},
    )

    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "form", "image": Image.open(IMAGES / "form.png")}]
    )
    doc = ctx.client.collection(collection.name).get(["form"], lsn=lsn)["form"]

    # Images read from TopK can be written back as-is
    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "copy", "image": doc["image"]}]
    )
    copy = ctx.client.collection(collection.name).get(["copy"], lsn=lsn)["copy"]

    assert copy["image"].format == "PNG"
    assert copy["image"].tobytes() == doc["image"].tobytes()


def test_image_corrupt(ctx: ProjectContext):
    collection = ctx.client.collections().create(
        ctx.scope("test"),
        schema={"image": image()},
    )

    corrupt = b"\xff\xd8\xff" + b"\x00" * 64
    lsn = ctx.client.collection(collection.name).upsert(
        [{"_id": "corrupt", "title": "Corrupt", "image": data_image(corrupt)}]
    )

    # The image is returned undecoded, the rest of the document is unaffected
    with pytest.warns(UserWarning, match="Cannot decode image value"):
        docs = ctx.client.collection(collection.name).get(["corrupt"], lsn=lsn)

    assert docs["corrupt"]["title"] == "Corrupt"
    assert isinstance(docs["corrupt"]["image"], data.Image)
    assert docs["corrupt"]["image"].format == "jpeg"
    assert docs["corrupt"]["image"].data == corrupt


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

    with pytest.warns(UserWarning, match="Pillow is not installed"):
        docs = ctx.client.collection(collection.name).get(["form"], lsn=lsn)

    assert docs["form"]["title"] == "Form"
    assert isinstance(docs["form"]["image"], data.Image)
    assert docs["form"]["image"].format == "png"
    assert docs["form"]["image"].data == png


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

    assert docs["cat"]["image"].format == "JPEG"
    assert docs["cat"]["image"].size == (474, 474)
