import io

import pytest
from PIL import Image
from topk_sdk import error
from topk_sdk.data import image

from .utils.dataset import IMAGES


def test_image_from_jpeg_bytes():
    data = (IMAGES / "cat.jpg").read_bytes()
    assert repr(image(data)) == f"Image(jpeg, {len(data)} bytes)"


def test_image_from_png_bytes():
    data = (IMAGES / "form.png").read_bytes()
    assert repr(image(data)) == f"Image(png, {len(data)} bytes)"


def test_image_from_pil_jpeg():
    assert repr(image(Image.open(IMAGES / "cat.jpg"))).startswith("Image(jpeg, ")


def test_image_from_pil_png():
    assert repr(image(Image.open(IMAGES / "form.png"))).startswith("Image(png, ")


def test_image_from_pil_in_memory():
    assert repr(image(Image.new("RGB", (4, 4), "red"))).startswith("Image(png, ")


def test_image_from_pil_modified_jpeg():
    # Derived images lose their source format and are encoded losslessly
    resized = Image.open(IMAGES / "cat.jpg").resize((16, 16))
    assert repr(image(resized)).startswith("Image(png, ")


@pytest.mark.parametrize("mode", ["CMYK", "YCbCr", "PA"])
def test_image_from_pil_converts_unsupported_png_modes(mode: str):
    assert repr(image(Image.new(mode, (4, 4)))).startswith("Image(png, ")


def test_image_from_pil_other_format():
    buf = io.BytesIO()
    Image.new("RGB", (4, 4), "red").save(buf, "WEBP")

    assert repr(image(Image.open(buf))).startswith("Image(png, ")


def test_image_unsupported_bytes():
    buf = io.BytesIO()
    Image.new("RGB", (4, 4), "red").save(buf, "WEBP")

    with pytest.raises(error.InvalidArgumentError, match="expected JPEG or PNG"):
        image(buf.getvalue())

    with pytest.raises(error.InvalidArgumentError, match="expected JPEG or PNG"):
        image(b"")


def test_image_invalid_input():
    with pytest.raises(TypeError, match="Expected bytes or PIL.Image.Image"):
        image("cat.jpg")  # type: ignore

    with pytest.raises(TypeError, match="Expected bytes or PIL.Image.Image"):
        image([0xFF, 0xD8, 0xFF])  # type: ignore
