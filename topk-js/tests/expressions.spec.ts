import { field, literal } from '../query';

describe('Expression Tests', () => {
  describe('Query expressions with flexible expressions', () => {
    test('addition operations', () => {
      expect(field('a').add(1).expr).toEqual(field('a').add(literal(1)).expr);
      // JS doesn't support operator overloading, so we can't test 1 + field('a')
    });

    test('subtraction operations', () => {
      expect(field('a').sub(1).expr).toEqual(field('a').sub(literal(1)).expr);
      // JS doesn't support operator overloading, so we can't test 1 - field('a')
    });

    test('multiplication operations', () => {
      expect(field('a').mul(1).expr).toEqual(field('a').mul(literal(1)).expr);
      // JS doesn't support operator overloading, so we can't test 1 * field('a')
    });

    test('division operations', () => {
      expect(field('a').div(1).expr).toEqual(field('a').div(literal(1)).expr);
      // JS doesn't support operator overloading, so we can't test 1 / field('a')
    });

    test('logical AND operations', () => {
      expect(field('a').and(true).expr).toEqual(field('a').and(literal(true)).expr);
      // JS doesn't support operator overloading, so we can't test true & field('a')
    });

    test('logical OR operations', () => {
      expect(field('a').or(false).expr).toEqual(field('a').or(literal(false)).expr);
      // JS doesn't support operator overloading, so we can't test false | field('a')
    });
  });

  describe('Comparison operators', () => {
    test('equality operations', () => {
      expect(field('a').eq(1).expr).toEqual(field('a').eq(literal(1)).expr);
      // JS doesn't support operator overloading, so we can't test 1 == field('a')
    });

    test('inequality operations', () => {
      expect(field('a').ne(1).expr).toEqual(field('a').ne(literal(1)).expr);
      // JS doesn't support operator overloading, so we can't test 1 != field('a')
    });

    test('less than operations', () => {
      expect(field('a').lt(1).expr).toEqual(field('a').lt(literal(1)).expr);
      // JS doesn't support operator overloading, so we can't test 1 > field('a')
    });

    test('less than or equal operations', () => {
      expect(field('a').lte(1).expr).toEqual(field('a').lte(literal(1)).expr);
      // JS doesn't support operator overloading, so we can't test 1 >= field('a')
    });

    test('greater than operations', () => {
      expect(field('a').gt(1).expr).toEqual(field('a').gt(literal(1)).expr);
      // JS doesn't support operator overloading, so we can't test 1 < field('a')
    });

    test('greater than or equal operations', () => {
      expect(field('a').gte(1).expr).toEqual(field('a').gte(literal(1)).expr);
      // JS doesn't support operator overloading, so we can't test 1 <= field('a')
    });
  });

  describe('Expression equality', () => {
    test('literals equality', () => {
      expect(literal('a').add(literal('b')).expr).toEqual(literal('a').add(literal('b')).expr);
      expect(literal('a').expr).not.toEqual(literal('b').expr);
    });

    test('fields equality', () => {
      expect(field('a').expr).toEqual(field('a').expr);
      expect(field('a').expr).not.toEqual(field('b').expr);
      expect(field('a').expr).not.toEqual(literal('a').expr);
      expect(literal('a').expr).not.toEqual(field('a').expr);
    });
  });

  describe('Query literal', () => {
    test('field operations with literals', () => {
      expect(field('foo').eq(literal(1))).toBeDefined();
      expect(field('foo').eq(1)).toBeDefined();

      expect(field('foo').ne(literal(1))).toBeDefined();
      expect(field('foo').ne(1)).toBeDefined();
    });
  });

  describe('Invalid operations', () => {
    test('invalid operations throw errors', () => {
      expect(() => literal(1).add("string" as any)).toThrow();
      expect(() => field('a').and(1 as any)).toThrow();
      expect(() => field('a').or(1 as any)).toThrow();
      expect(() => field('a').add(null as any)).toThrow();
      expect(() => field('a').add("string" as any)).toThrow();
      expect(() => field('a').add([1, 2, 3] as any)).toThrow();
      expect(() => field('a').add({a: 1} as any)).toThrow();
    });
  });
});
