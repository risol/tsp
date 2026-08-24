import { deepStrictEqual, ok, strictEqual } from "node:assert/strict";

export function assertEquals<T>(actual: T, expected: T, message?: string): void {
  deepStrictEqual(actual, expected, message);
}

export function assertExists<T>(value: T, message?: string): asserts value is NonNullable<T> {
  ok(value !== null && value !== undefined, message);
}

export function assertStringIncludes(actual: string, expected: string, message?: string): void {
  strictEqual(actual.includes(expected), true, message);
}
