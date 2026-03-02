/**
 * Shared dependency file
 * Referenced by multiple test files for compilation deduplication testing
 */

export const SHARED_VALUE = "shared-dependency-value";

export function getSharedValue(): string {
  return SHARED_VALUE;
}
