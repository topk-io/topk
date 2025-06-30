import { binaryVector, f32Vector, u8Vector } from "../../../lib/data";

const TYPE_ERROR = "Invalid vector value, must be `number[]`";

describe("f32Vector", () => {
  test("valid", () => {
    f32Vector([1, 2, 3]);
  });

  test("empty case", () => {
    f32Vector([]);
  });

  test("toString", () => {
    expect(f32Vector([1, 2, 3]).toString()).toEqual(
      "Vector(Float { values: [1.0, 2.0, 3.0] })"
    );
  });

  test("invalid arguments", () => {
    expect(() => f32Vector(0 as any)).toThrow(TYPE_ERROR);
    expect(() => f32Vector(null as any)).toThrow(TYPE_ERROR);
    expect(() => f32Vector(undefined as any)).toThrow(TYPE_ERROR);
    expect(() => f32Vector(false as any)).toThrow(TYPE_ERROR);
    expect(() => f32Vector(NaN as any)).toThrow(TYPE_ERROR);
    expect(() => f32Vector(Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => f32Vector(-Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => f32Vector(Symbol("foo") as any)).toThrow(TYPE_ERROR);
    expect(() => f32Vector(BigInt(1) as any)).toThrow(TYPE_ERROR);
    expect(() => f32Vector({ 1: 256 } as any)).toThrow(TYPE_ERROR);
    expect(() => f32Vector({ 1: -1 } as any)).toThrow(TYPE_ERROR);
  });
});

describe("u8Vector", () => {
  test("valid", () => {
    u8Vector([1, 2, 3]);
  });

  test("empty case", () => {
    u8Vector([]);
  });

  test("toString", () => {
    expect(u8Vector([1, 2, 3]).toString()).toEqual(
      "Vector(Byte { values: [1, 2, 3] })"
    );
  });

  test("invalid number range", () => {
    expect(() => u8Vector([256] as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector([-1] as any)).toThrow(TYPE_ERROR);
  });

  test("invalid arguments", () => {
    expect(() => u8Vector(0 as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector(null as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector(undefined as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector(false as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector(NaN as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector(Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector(-Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector(Symbol("foo") as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector(BigInt(1) as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector({ 1: 256 } as any)).toThrow(TYPE_ERROR);
    expect(() => u8Vector({ 1: -1 } as any)).toThrow(TYPE_ERROR);
  });
});

describe("binaryVector", () => {
  test("valid", () => {
    binaryVector([1, 2, 3]);
  });

  test("empty case", () => {
    binaryVector([]);
  });

  test("toString", () => {
    expect(binaryVector([1, 2, 3]).toString()).toEqual(
      "Vector(Byte { values: [1, 2, 3] })"
    );
  });

  test("invalid number range", () => {
    expect(() => binaryVector([256] as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector([-1] as any)).toThrow(TYPE_ERROR);
  });

  test("invalid arguments", () => {
    expect(() => binaryVector(0 as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector(null as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector(undefined as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector(false as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector(NaN as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector(Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector(-Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector(Symbol("foo") as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector(BigInt(1) as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector({ 1: 256 } as any)).toThrow(TYPE_ERROR);
    expect(() => binaryVector({ 1: -1 } as any)).toThrow(TYPE_ERROR);
  });
});
