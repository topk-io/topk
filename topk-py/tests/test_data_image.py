import io
import sys

import pytest
from PIL import Image
from topk_sdk import error
from topk_sdk.data import image

from .utils.dataset import IMAGES


def test_image_from_jpeg_bytes():
    data = (IMAGES / "cat.jpg").read_bytes()
    img = image(data)

    assert repr(img) == f"Image(jpeg, {len(data)} bytes)"
    assert img.format == "jpeg"
    assert img.data == data


def test_image_from_png_bytes():
    data = (IMAGES / "form.png").read_bytes()
    img = image(data)

    assert repr(img) == f"Image(png, {len(data)} bytes)"
    assert img.format == "png"
    assert img.data == data


def test_image_data_is_not_copied():
    img = image((IMAGES / "cat.jpg").read_bytes())
    assert img.data is img.data


def test_image_from_pil_jpeg():
    img = image(Image.open(IMAGES / "cat.jpg"))

    assert repr(img).startswith("Image(jpeg, ")
    assert img.format == "jpeg"
    assert Image.open(io.BytesIO(img.data)).format == "JPEG"


def test_image_equality():
    data = (IMAGES / "cat.jpg").read_bytes()

    assert image(data) == image(data)
    assert image(data) != image((IMAGES / "form.png").read_bytes())


def test_image_to_pil():
    decoded = image((IMAGES / "form.png").read_bytes()).to_pil()

    assert decoded.format == "PNG"
    assert decoded.tobytes() == Image.open(IMAGES / "form.png").tobytes()


def test_image_to_pil_corrupt():
    with pytest.raises(OSError):
        image(b"\xff\xd8\xff" + b"\x00" * 64).to_pil()


def test_image_to_pil_without_pillow(monkeypatch: pytest.MonkeyPatch):
    img = image((IMAGES / "form.png").read_bytes())

    # Make `import PIL.Image` fail
    monkeypatch.setitem(sys.modules, "PIL.Image", None)

    with pytest.raises(ImportError, match="pip install pillow"):
        img.to_pil()


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

    # Truncated PNG signature
    with pytest.raises(error.InvalidArgumentError, match="expected JPEG or PNG"):
        image(b"\x89PNG")


def test_image_invalid_input():
    with pytest.raises(TypeError, match="Expected bytes or PIL.Image.Image"):
        image("cat.jpg")  # type: ignore

    with pytest.raises(TypeError, match="Expected bytes or PIL.Image.Image"):
        image([0xFF, 0xD8, 0xFF])  # type: ignore
