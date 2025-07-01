import { f32SparseVector, u8SparseVector } from "../lib/data";

const TYPE_ERROR = "Invalid sparse vector, must be `Record<number, number>`";

describe("f32SparseVector", () => {
  test("valid", () => {
    f32SparseVector({ 1: 1.1 });
    f32SparseVector({ 1: 1 });
  });

  test("empty case", () => {
    f32SparseVector({});
  });

  test("toString", () => {
    expect(f32SparseVector({ 1: 1.1 }).toString()).toEqual(
      "SparseVector(Float { vector: SparseVectorData { indices: [1], values: [1.1] } })"
    );
  });

  test("invalid key", () => {
    expect(() => f32SparseVector({ foo: 1 } as any)).toThrow(TYPE_ERROR);
  });

  test("invalid value", () => {
    expect(() => f32SparseVector({ 1: "foo" } as any)).toThrow(TYPE_ERROR);
  });

  test("invalid types", () => {
    expect(() => f32SparseVector(0 as any)).toThrow(TYPE_ERROR);
    expect(() => f32SparseVector([] as any)).toThrow(TYPE_ERROR);
    expect(() => f32SparseVector(null as any)).toThrow(TYPE_ERROR);
    expect(() => f32SparseVector(undefined as any)).toThrow(TYPE_ERROR);
    expect(() => f32SparseVector(false as any)).toThrow(TYPE_ERROR);
    expect(() => f32SparseVector(NaN as any)).toThrow(TYPE_ERROR);
    expect(() => f32SparseVector(Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => f32SparseVector(-Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => f32SparseVector(Symbol("foo") as any)).toThrow(TYPE_ERROR);
    expect(() => f32SparseVector(BigInt(1) as any)).toThrow(TYPE_ERROR);
  });
});

describe("u8SparseVector", () => {
  test("valid", () => {
    u8SparseVector({ 1: 1 });
    u8SparseVector({ 1: 1, 2: 2 });
  });

  test("empty case", () => {
    u8SparseVector({});
  });

  test("toString", () => {
    expect(u8SparseVector({ 1: 1 }).toString()).toEqual(
      "SparseVector(Byte { vector: SparseVectorData { indices: [1], values: [1] } })"
    );
  });

  test("invalid key", () => {
    expect(() => u8SparseVector({ foo: 1 } as any)).toThrow(TYPE_ERROR);
  });

  test("invalid value", () => {
    expect(() => u8SparseVector({ 1: "foo" } as any)).toThrow(TYPE_ERROR);
  });

  test("invalid number range", () => {
    expect(() => u8SparseVector({ 1: 256 } as any)).toThrow(TYPE_ERROR);
    expect(() => u8SparseVector({ 1: -1 } as any)).toThrow(TYPE_ERROR);
  });

  test("invalid arguments", () => {
    expect(() => u8SparseVector(0 as any)).toThrow(TYPE_ERROR);
    expect(() => u8SparseVector([] as any)).toThrow(TYPE_ERROR);
    expect(() => u8SparseVector(null as any)).toThrow(TYPE_ERROR);
    expect(() => u8SparseVector(undefined as any)).toThrow(TYPE_ERROR);
    expect(() => u8SparseVector(false as any)).toThrow(TYPE_ERROR);
    expect(() => u8SparseVector(NaN as any)).toThrow(TYPE_ERROR);
    expect(() => u8SparseVector(Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => u8SparseVector(-Infinity as any)).toThrow(TYPE_ERROR);
    expect(() => u8SparseVector(Symbol("foo") as any)).toThrow(TYPE_ERROR);
    expect(() => u8SparseVector(BigInt(1) as any)).toThrow(TYPE_ERROR);
  });
});
