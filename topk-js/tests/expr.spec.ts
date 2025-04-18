import { field, literal } from "../lib/query";

describe("Expression Tests", () => {
  it("should handle flexible expressions", () => {
    expect(field("a").add(1).expr).toEqual(field("a").add(literal(1)).expr);

    expect(field("a").sub(1).expr).toEqual(field("a").sub(literal(1)).expr);

    expect(field("a").mul(1).expr).toEqual(field("a").mul(literal(1)).expr);

    expect(field("a").div(1).expr).toEqual(field("a").div(literal(1)).expr);

    expect(field("a").and(true).expr).toEqual(
      field("a").and(literal(true)).expr
    );

    expect(field("a").or(false).expr).toEqual(
      field("a").or(literal(false)).expr
    );
  });

  it("should handle comparison operators", () => {
    expect(field("a").eq(1).expr).toEqual(field("a").eq(literal(1)).expr);

    expect(field("a").ne(1).expr).toEqual(field("a").ne(literal(1)).expr);

    expect(field("a").lt(1).expr).toEqual(field("a").lt(literal(1)).expr);

    expect(field("a").lte(1).expr).toEqual(field("a").lte(literal(1)).expr);

    expect(field("a").gt(1).expr).toEqual(field("a").gt(literal(1)).expr);

    expect(field("a").gte(1).expr).toEqual(field("a").gte(literal(1)).expr);
  });

  it("should handle expression equality", () => {
    expect(literal("a").add(literal("b")).expr).toEqual(
      literal("a").add(literal("b")).expr
    );
    expect(literal("a").expr).not.toEqual(literal("b").expr);
    expect(field("a").expr).toEqual(field("a").expr);
    expect(field("a").expr).not.toEqual(field("b").expr);
    expect(field("a").expr).not.toEqual(literal("a").expr);
    expect(literal("a").expr).not.toEqual(field("a").expr);
  });

  it("should handle query literals", () => {
    field("foo").eq(literal(1));
    field("foo").eq(1);

    field("foo").ne(literal(1));
    field("foo").ne(1);
  });

  it("should throw errors for invalid operations", () => {
    expect(() => literal(1).add("string" as any)).toThrow(
      "Unsupported numeric type: string"
    );
    expect(() => field("a").and(1 as any)).toThrow(
      "Unsupported boolish type: number"
    );
    expect(() => field("a").or(1 as any)).toThrow(
      "Unsupported boolish type: number"
    );
    expect(() => field("a").add(null)).toThrow(
      "Unsupported numeric type: null"
    );
    expect(() => field("a").add("string" as any)).toThrow(
      "Unsupported numeric type: string"
    );
    expect(() => field("a").add([1, 2, 3] as any)).toThrow(
      "Unsupported numeric type: object"
    );
    expect(() => field("a").add({ a: 1 } as any)).toThrow(
      "Unsupported numeric type: object"
    );
  });
});
