import { f32SparseVector } from "../lib/data";

describe("sparse vector", () => {
  it("constructor", () => {
    expect(f32SparseVector({ 1: 1.1 }).toString()).toEqual(
      "SparseVector(Float { vector: SparseVectorData { indices: [1], values: [1.1] } })"
    );

    expect(f32SparseVector({ 1: 1 }).toString()).toEqual(
      "SparseVector(Float { vector: SparseVectorData { indices: [1], values: [1.0] } })"
    );

    expect(f32SparseVector({}).toString()).toEqual(
      "SparseVector(Float { vector: SparseVectorData { indices: [], values: [] } })"
    );
  });

  it("invalid types", () => {
    expect(() => f32SparseVector({ foo: 1 } as any)).toThrow(
      "Invalid sparse vector key, must be u32"
    );

    expect(() => f32SparseVector({ 1: "foo" } as any)).toThrow(
      "Invalid sparse vector value, must be f64"
    );
  });

  it("invalid arguments", () => {
    expect(() => f32SparseVector(0 as any)).toThrow("Invalid sparse vector");
    expect(() => f32SparseVector([] as any)).toThrow("Invalid sparse vector");
    expect(() => f32SparseVector(null as any)).toThrow("Invalid sparse vector");
    expect(() => f32SparseVector(undefined as any)).toThrow(
      "Invalid sparse vector"
    );
    expect(() => f32SparseVector(false as any)).toThrow(
      "Invalid sparse vector"
    );
    expect(() => f32SparseVector(NaN as any)).toThrow("Invalid sparse vector");
    expect(() => f32SparseVector(Infinity as any)).toThrow(
      "Invalid sparse vector"
    );
    expect(() => f32SparseVector(-Infinity as any)).toThrow(
      "Invalid sparse vector"
    );
    expect(() => f32SparseVector(Symbol("foo") as any)).toThrow(
      "Invalid sparse vector"
    );
    expect(() => f32SparseVector(BigInt(1) as any)).toThrow(
      "Invalid sparse vector"
    );
  });
});
