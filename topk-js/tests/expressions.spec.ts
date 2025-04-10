import { field, literal } from '../index';

describe('Expression Tests', () => {
  describe('Query expressions with flexible expressions', () => {
    test('addition operations', () => {
      expect(field('a').add(1)).toStrictEqual(field('a').add(literal(1)));
      // JS doesn't support operator overloading, so we can't test 1 + field('a')
    });

    test('subtraction operations', () => {
      expect(field('a').sub(1)).toStrictEqual(field('a').sub(literal(1)));
      // JS doesn't support operator overloading, so we can't test 1 - field('a')
    });

    test('multiplication operations', () => {
      expect(field('a').mul(1)).toStrictEqual(field('a').mul(literal(1)));
      // JS doesn't support operator overloading, so we can't test 1 * field('a')
    });

    test('division operations', () => {
      expect(field('a').div(1)).toStrictEqual(field('a').div(literal(1)));
      // JS doesn't support operator overloading, so we can't test 1 / field('a')
    });

    test('logical AND operations', () => {
      expect(field('a').and(true)).toStrictEqual(field('a').and(literal(true)));
      // JS doesn't support operator overloading, so we can't test true & field('a')
    });

    test('logical OR operations', () => {
      expect(field('a').or(false)).toStrictEqual(field('a').or(literal(false)));
      // JS doesn't support operator overloading, so we can't test false | field('a')
    });
  });

  describe('Comparison operators', () => {
    test('equality operations', () => {
      expect(field('a').eq(1)).toStrictEqual(field('a').eq(literal(1)));
      // JS doesn't support operator overloading, so we can't test 1 == field('a')
    });

    test('inequality operations', () => {
      expect(field('a').ne(1)).toStrictEqual(field('a').ne(literal(1)));
      // JS doesn't support operator overloading, so we can't test 1 != field('a')
    });

    test('less than operations', () => {
      expect(field('a').lt(1)).toStrictEqual(field('a').lt(literal(1)));
      // JS doesn't support operator overloading, so we can't test 1 > field('a')
    });

    test('less than or equal operations', () => {
      expect(field('a').lte(1)).toStrictEqual(field('a').lte(literal(1)));
      // JS doesn't support operator overloading, so we can't test 1 >= field('a')
    });

    test('greater than operations', () => {
      expect(field('a').gt(1)).toStrictEqual(field('a').gt(literal(1)));
      // JS doesn't support operator overloading, so we can't test 1 < field('a')
    });

    test('greater than or equal operations', () => {
      expect(field('a').gte(1)).toStrictEqual(field('a').gte(literal(1)));
      // JS doesn't support operator overloading, so we can't test 1 <= field('a')
    });
  });

  describe('Expression equality', () => {
    test('literals equality', () => {
      expect(literal('a').add(literal('b'))).toStrictEqual(literal('a').add(literal('b')));
      expect(literal('a')).not.toStrictEqual(literal('b'));
    });

    test('fields equality', () => {
      expect(field('a')).toStrictEqual(field('a'));
      expect(field('a')).not.toStrictEqual(field('b'));
      expect(field('a')).not.toStrictEqual(literal('a'));
      expect(literal('a')).not.toStrictEqual(field('a'));
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
