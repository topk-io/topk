import {describe, expect, test} from '@jest/globals';
import { match } from '../index.js'

describe('sum module', () => {
  test('adds 1 + 2 to equal 3', () => {
    expect(match('hello')).toBeTruthy();
  });
});