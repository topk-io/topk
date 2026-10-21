import { readFileSync } from "fs";
import { join } from "path";
import { image, Image } from "../lib/data";

const IMAGES = join(__dirname, "../../assets/test/img");

const UNSUPPORTED_FORMAT = "Unsupported image format, expected JPEG or PNG data";
const INVALID_DATA = "Invalid image data, must be `Buffer` or `Uint8Array`";

describe("image", () => {
  test("jpeg", () => {
    const data = readFileSync(join(IMAGES, "cat.jpg"));
    const img = image(data);

    expect(img).toBeInstanceOf(Image);
    expect(img.format).toBe("jpeg");
    expect(img.data).toEqual(data);
    expect(img.toString()).toBe(`Image(jpeg, ${data.length} bytes)`);
  });

  test("png", () => {
    const data = readFileSync(join(IMAGES, "form.png"));
    const img = image(data);

    expect(img.format).toBe("png");
    expect(img.data).toEqual(data);
  });

  test("data is not copied on access", () => {
    const img = image(readFileSync(join(IMAGES, "cat.jpg")));
    expect(img.data).toBe(img.data);
  });

  test("data is a snapshot of the input", () => {
    const data = readFileSync(join(IMAGES, "cat.jpg"));
    const img = image(data);

    data[100] ^= 0xff;
    expect(img.data[100]).not.toBe(data[100]);
  });

  test("uint8array", () => {
    const data = readFileSync(join(IMAGES, "form.png"));
    const img = image(new Uint8Array(data));

    expect(img.format).toBe("png");
    expect(img.data).toEqual(data);
  });

  test("unsupported format", () => {
    expect(() => image(Buffer.from("GIF89a"))).toThrow(UNSUPPORTED_FORMAT);
    expect(() => image(Buffer.alloc(0))).toThrow(UNSUPPORTED_FORMAT);
    // Truncated PNG signature
    expect(() => image(Buffer.from([0x89, 0x50, 0x4e, 0x47]))).toThrow(UNSUPPORTED_FORMAT);
  });

  test("invalid arguments", () => {
    expect(() => image("cat.jpg" as any)).toThrow(INVALID_DATA);
    expect(() => image([0xff, 0xd8, 0xff] as any)).toThrow(INVALID_DATA);
    expect(() => image(new Float32Array(4) as any)).toThrow(INVALID_DATA);
    expect(() => image(null as any)).toThrow(INVALID_DATA);
    expect(() => image(undefined as any)).toThrow(INVALID_DATA);
  });
});
