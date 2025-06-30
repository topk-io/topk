import { bytes } from "../../lib/data";

const TYPE_ERROR = "Invalid bytes value, must be `number[]` or `Buffer`";

describe("bytes", () => {
  test("valid", () => {
    bytes([1, 2, 3]);
    bytes(Buffer.from([1, 2, 3]));
  });

  test("empty case", () => {
    bytes([]);
    bytes(Buffer.from([]));
  });

  test("invalid number range", () => {
    expect(() => bytes([256] as any)).toThrow(TYPE_ERROR);
    expect(() => bytes([-1] as any)).toThrow(TYPE_ERROR);
  });

  test("invalid arguments", () => {
    expect(() => bytes(0 as any)).toThrow(TYPE_ERROR);
    expect(() => bytes(null as any)).toThrow(TYPE_ERROR);
    expect(() => bytes(undefined as any)).toThrow(TYPE_ERROR);
    expect(() => bytes(false as any)).toThrow(TYPE_ERROR);
    expect(() => bytes(NaN as any)).toThrow(TYPE_ERROR);
    expect(() => bytes(Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => bytes(-Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => bytes(Symbol("foo") as any)).toThrow(TYPE_ERROR);
    expect(() => bytes(BigInt(1) as any)).toThrow(TYPE_ERROR);
    expect(() => bytes({ 1: 256 } as any)).toThrow(TYPE_ERROR);
    expect(() => bytes({ 1: -1 } as any)).toThrow(TYPE_ERROR);
  });
});
