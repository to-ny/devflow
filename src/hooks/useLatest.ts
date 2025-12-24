import { useRef } from "react";

/**
 * Returns a ref that always contains the latest value.
 * Useful for accessing current values in callbacks without stale closures.
 */
export function useLatest<T>(value: T): { readonly current: T } {
  const ref = useRef(value);
  ref.current = value;
  return ref;
}
