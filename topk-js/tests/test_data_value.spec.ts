import { bytes, listI32, listU32, listI64, listF64 } from "../lib/data";

const TYPE_ERROR_BYTES = "Invalid bytes value, must be `number[]` or `Buffer`";
const TYPE_ERROR_LIST = "Given napi value is not an array";
const TYPE_ERROR_STRING_TO_INT32 = "Failed to convert napi value String into rust type `i32`";
const TYPE_ERROR_STRING_TO_INT64 = "Failed to convert napi value String into rust type `i64`";
const TYPE_ERROR_STRING_TO_UINT32 = "Failed to convert napi value String into rust type `u32`";
const TYPE_ERROR_STRING_TO_FLOAT = "Failed to convert napi value String into rust type `f64`";

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
    listI32([1, 2, 3]);
    listU32([1, 2, 3]);
    listI64([1, 2, 3]);
    listF64([1.0, 2.0, 3.0]);
  });

  test("empty case", () => {
    listI32([]);
    listU32([]);
    listI64([]);
    listF64([]);
  });

  test("invalid arguments", () => {
    expect(() => listI32(0 as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32(null as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32(undefined as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32(false as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32(NaN as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32(Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32(-Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32(Symbol("foo") as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32(BigInt(1) as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32({ 1: 256 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32({ 1: -1 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI32(["1", "2", "3"] as any)).toThrow(TYPE_ERROR_STRING_TO_INT32);
    expect(() => listU32(0 as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32(null as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32(undefined as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32(false as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32(NaN as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32(Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32(-Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32(Symbol("foo") as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32(BigInt(1) as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32({ 1: 256 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32({ 1: -1 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listU32(["1", "2", "3"] as any)).toThrow(TYPE_ERROR_STRING_TO_UINT32);
    expect(() => listI64(0 as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64(null as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64(undefined as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64(false as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64(NaN as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64(Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64(-Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64(Symbol("foo") as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64(BigInt(1) as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64({ 1: 256 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64({ 1: -1 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listI64(["1", "2", "3"] as any)).toThrow(TYPE_ERROR_STRING_TO_INT64);
    expect(() => listF64(0 as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64(null as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64(undefined as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64(false as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64(NaN as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64(Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64(-Infinity as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64(Symbol("foo") as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64(BigInt(1) as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64({ 1: 256 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64({ 1: -1 } as any)).toThrow(TYPE_ERROR_LIST);
    expect(() => listF64(["1", "2", "3"] as any)).toThrow(TYPE_ERROR_STRING_TO_FLOAT);
  });
});