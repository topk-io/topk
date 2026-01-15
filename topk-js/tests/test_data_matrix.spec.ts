import { Matrix, matrix } from "../lib/data";

describe("Matrix Data", () => {
  test("matrix f32 defaults", () => {
    const m = matrix([
      [1.0, 2.0],
      [3.0, 4.0],
    ]);
    expect(m).toBeInstanceOf(Matrix);
    expect(m.toString()).toBe("Matrix(2, F32([1.0, 2.0, 3.0, 4.0]))");
  });

  test.each(["f32", "f16", "f8"] as const)("matrix float valueType=%s", (valueType) => {
    const m = matrix(
      [
        [1.0, 2.0],
        [3.0, 4.0],
      ],
      valueType
    );
    expect(m).toBeInstanceOf(Matrix);
    if (valueType === "f16") {
      expect(m.toString()).toBe("Matrix(2, F16([1.0, 2.0, 3.0, 4.0]))");
    } else if (valueType === "f8") {
      expect(m.toString()).toBe("Matrix(2, F8([1.0, 2.0, 3.0, 4.0]))");
    } else {
      expect(m.toString()).toBe("Matrix(2, F32([1.0, 2.0, 3.0, 4.0]))");
    }
  });

  test.each(["u8", "i8"] as const)("matrix int valueType=%s", (valueType) => {
    const m = matrix(
      [
        [0, 1, 2],
        [3, 4, 5],
      ],
      valueType
    );
    expect(m).toBeInstanceOf(Matrix);
    if (valueType === "u8") {
      expect(m.toString()).toBe("Matrix(3, U8([0, 1, 2, 3, 4, 5]))");
    } else {
      expect(m.toString()).toBe("Matrix(3, I8([0, 1, 2, 3, 4, 5]))");
    }
  });

  test("matrix rejects empty outer list", () => {
    expect(() => matrix([] as unknown as number[][])).toThrow(
      /Cannot create matrix from empty list/
    );
  });

  test("matrix rejects first row empty but later rows non-empty", () => {
    expect(() => matrix([[], [1.0, 2.0]] as unknown as number[][])).toThrow(
      /Cannot create matrix from empty list/
    );
  });

  test("matrix rejects mismatched row lengths", () => {
    expect(() => matrix([[1.0, 2.0], [3.0]] as unknown as number[][])).toThrow(
      /All rows must have the same length/
    );
  });

  test.each([
    [[[1.0, 2.0], []] as number[][], 1],
    [[[1.0, 2.0], [], [3.0, 4.0]] as number[][], 1],
    [[[1.0, 2.0], [3.0, 4.0], []] as number[][], 2],
  ])("matrix rejects empty row", (testData, expectedRowIdx) => {
    expect(() => matrix(testData as unknown as number[][])).toThrow(
      new RegExp(`All rows must have the same length.*Row ${expectedRowIdx} has length 0`)
    );
  });

  test("matrix u8 rejects floats", () => {
    expect(() => matrix([[0.0, 1.5]] as unknown as number[][], "u8")).toThrow(
      /cannot be interpreted as an integer/i
    );
  });
});
