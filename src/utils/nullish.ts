/**
 * Checks if a value is null or undefined.
 */
export function isNullish(value: unknown): value is null | undefined {
  return value == null;
}

/**
 * Checks if a value is neither null nor undefined.
 */
export function isPresent<T>(value: T | null | undefined): value is T {
  return value != null;
}
