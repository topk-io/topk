import { field, literal } from "../lib/query";

describe("Expression Tests", () => {
  it("should handle flexible expressions", () => {
    expect(field("a").add(1).toString()).toEqual(
      field("a").add(literal(1)).toString()
    );

    expect(field("a").sub(1).toString()).toEqual(
      field("a").sub(literal(1)).toString()
    );

    expect(field("a").mul(1).toString()).toEqual(
      field("a").mul(literal(1)).toString()
    );

    expect(field("a").div(1).toString()).toEqual(
      field("a").div(literal(1)).toString()
    );

    expect(field("a").and(true).toString()).toEqual(
      field("a").and(literal(true)).toString()
    );

    expect(field("a").or(false).toString()).toEqual(
      field("a").or(literal(false)).toString()
    );
  });

  it("should handle comparison operators", () => {
    expect(field("a").eq(1).toString()).toEqual(
      field("a").eq(literal(1)).toString()
    );

    expect(field("a").ne(1).toString()).toEqual(
      field("a").ne(literal(1)).toString()
    );

    expect(field("a").lt(1).toString()).toEqual(
      field("a").lt(literal(1)).toString()
    );

    expect(field("a").lte(1).toString()).toEqual(
      field("a").lte(literal(1)).toString()
    );

    expect(field("a").gt(1).toString()).toEqual(
      field("a").gt(literal(1)).toString()
    );

    expect(field("a").gte(1).toString()).toEqual(
      field("a").gte(literal(1)).toString()
    );
  });

  it("should handle expression equality", () => {
    expect(literal("a").add(literal("b")).toString()).toEqual(
      literal("a").add(literal("b")).toString()
    );
    expect(literal("a").toString()).not.toEqual(literal("b").toString());
    expect(field("a").toString()).toEqual(field("a").toString());
    expect(field("a").toString()).not.toEqual(field("b").toString());
    expect(field("a").toString()).not.toEqual(literal("a").toString());
    expect(literal("a").toString()).not.toEqual(field("a").toString());
  });

  it("should handle query literals", () => {
    field("foo").eq(literal(1));
    field("foo").eq(1);

    field("foo").ne(literal(1));
    field("foo").ne(1);
  });

  it("should throw errors for invalid operations", () => {
    expect(() => literal(1).add("string" as any)).toThrow(
      "Unsupported numeric type: String"
    );
    expect(() => field("a").and(1 as any)).toThrow("Unsupported bool type");
    expect(() => field("a").or(1 as any)).toThrow("Unsupported bool type");
    expect(() => field("a").add(null)).toThrow("Unsupported numeric type");
    expect(() => field("a").add("string" as any)).toThrow(
      "Unsupported numeric type"
    );
    expect(() => field("a").add([1, 2, 3] as any)).toThrow(
      "Unsupported numeric type"
    );
    expect(() => field("a").add({ a: 1 } as any)).toThrow(
      "Unsupported object type"
    );
  });
});
