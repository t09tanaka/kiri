/**
 * Non-reactive cache helper for parsed git diff payloads.
 *
 * `DiffView.svelte` (and any future component that needs to memoise an
 * expensive transform of `path -> derivedValue`) used to allocate a raw
 * `Map` and silence the Svelte lint rule at every callsite with
 * `// eslint-disable-next-line svelte/prefer-svelte-reactivity`. That worked
 * but spread the rationale across the codebase.
 *
 * This helper centralises the eslint-disable: the cache is intentionally
 * non-reactive because mutating the Map inside a `$derived` would trigger
 * `state_unsafe_mutation`. Callers receive a typed, narrow API instead of a
 * raw Map.
 */

type CacheStore<V> = Map<string, V>;

export interface DiffCache<V> {
  /** Return the cached value for `path`, computing & storing it on first access. */
  getOrCompute: (path: string, compute: () => V) => V;
  /** Drop every entry. Call this when the underlying diff list changes. */
  clear: () => void;
  /** Internal size, mainly for tests. */
  size: () => number;
}

export function createDiffCache<V>(): DiffCache<V> {
  const store: CacheStore<V> = new Map<string, V>();

  return {
    getOrCompute(path, compute) {
      const existing = store.get(path);
      if (existing !== undefined) return existing;
      const value = compute();
      store.set(path, value);
      return value;
    },
    clear() {
      store.clear();
    },
    size() {
      return store.size;
    },
  };
}
