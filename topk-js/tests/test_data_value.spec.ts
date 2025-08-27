import { bytes, f64List, i32List, i64List, stringList, u32List } from "../lib/data";

const TYPE_ERROR_BYTES = "Invalid bytes value, must be `number[]` or `Buffer`";
const TYPE_ERROR_LIST = "Given napi value is not an array";
const TYPE_ERROR_STRING_TO_INT32 =
  "Failed to convert napi value String into rust type `i32`";
const TYPE_ERROR_STRING_TO_INT64 =
  "Failed to convert napi value String into rust type `i64`";
const TYPE_ERROR_STRING_TO_UINT32 =
  "Failed to convert napi value String into rust type `u32`";
const TYPE_ERROR_STRING_TO_FLOAT =
  "Failed to convert napi value String into rust type `f64`";

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
    expect(() => bytes([256] as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes([-1] as any)).toThrow(TYPE_ERROR_BYTES);
  });

  test("invalid arguments", () => {
    expect(() => bytes(0 as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes(null as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes(undefined as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes(false as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes(NaN as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes(Infinity as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes(-Infinity as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes(Symbol("foo") as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes(BigInt(1) as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes({ 1: 256 } as any)).toThrow(TYPE_ERROR_BYTES);
    expect(() => bytes({ 1: -1 } as any)).toThrow(TYPE_ERROR_BYTES);
  });
});

describe("list", () => {
  test("valid", () => {
    i32List([1, 2, 3]);
    u32List([1, 2, 3]);
    i64List([1, 2, 3]);
    f64List([1.0, 2.0, 3.0]);
    stringList(["1", "2", "3"]);
  });

  test("empty case", () => {
    i32List([]);
    u32List([]);
    i64List([]);
    f64List([]);
    stringList([]);
  });

  test("invalid arguments", () => {
    expect(() => i32List(0 as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List(null as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List(undefined as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List(false as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List(NaN as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List(Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List(-Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List(Symbol("foo") as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List(BigInt(1) as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List({ 1: 256 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List({ 1: -1 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i32List(["1", "2", "3"] as any)).toThrow(
      TYPE_ERROR_STRING_TO_INT32
    );
    expect(() => u32List(0 as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List(null as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List(undefined as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List(false as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List(NaN as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List(Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List(-Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List(Symbol("foo") as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List(BigInt(1) as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List({ 1: 256 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List({ 1: -1 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => u32List(["1", "2", "3"] as any)).toThrow(
      TYPE_ERROR_STRING_TO_UINT32
    );
    expect(() => i64List(0 as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List(null as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List(undefined as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List(false as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List(NaN as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List(Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List(-Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List(Symbol("foo") as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List(BigInt(1) as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List({ 1: 256 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List({ 1: -1 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => i64List(["1", "2", "3"] as any)).toThrow(
      TYPE_ERROR_STRING_TO_INT64
    );
    expect(() => f64List(0 as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List(null as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List(undefined as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List(false as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List(NaN as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List(Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List(-Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List(Symbol("foo") as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List(BigInt(1) as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List({ 1: 256 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List({ 1: -1 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => f64List(["1", "2", "3"] as any)).toThrow(
      TYPE_ERROR_STRING_TO_FLOAT
    );
    expect(() => stringList(0 as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => stringList(null as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => stringList(undefined as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => stringList(false as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => stringList(NaN as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => stringList(Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => stringList(-Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => stringList(Symbol("foo") as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => stringList(BigInt(1) as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => stringList({ 1: 256 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => stringList({ 1: -1 } as any)).toThrow(TYPE_ERROR_LIST);
  });
});
