import { describe, expect, it, vi } from 'vitest';
import { createDiffCache } from './diffCache';

describe('createDiffCache', () => {
  it('returns the value computed on first access', () => {
    const cache = createDiffCache<number>();
    const compute = vi.fn(() => 42);

    expect(cache.getOrCompute('a', compute)).toBe(42);
    expect(compute).toHaveBeenCalledTimes(1);
  });

  it('does not recompute on subsequent reads of the same key', () => {
    const cache = createDiffCache<number>();
    const compute = vi.fn(() => 7);

    cache.getOrCompute('a', compute);
    cache.getOrCompute('a', compute);
    cache.getOrCompute('a', compute);

    expect(compute).toHaveBeenCalledTimes(1);
  });

  it('caches per-key independently', () => {
    const cache = createDiffCache<string>();
    expect(cache.getOrCompute('a', () => 'A')).toBe('A');
    expect(cache.getOrCompute('b', () => 'B')).toBe('B');
    expect(cache.size()).toBe(2);
  });

  it('clear() drops every entry', () => {
    const cache = createDiffCache<number>();
    cache.getOrCompute('a', () => 1);
    cache.getOrCompute('b', () => 2);
    expect(cache.size()).toBe(2);

    cache.clear();
    expect(cache.size()).toBe(0);

    const compute = vi.fn(() => 99);
    cache.getOrCompute('a', compute);
    expect(compute).toHaveBeenCalledTimes(1);
  });

  it('caches falsy values without recomputing', () => {
    const cache = createDiffCache<number>();
    const compute = vi.fn(() => 0);

    cache.getOrCompute('a', compute);
    cache.getOrCompute('a', compute);

    expect(compute).toHaveBeenCalledTimes(1);
  });

  it('caches empty array values without recomputing', () => {
    const cache = createDiffCache<readonly string[]>();
    const compute = vi.fn(() => [] as const);

    cache.getOrCompute('a', compute);
    cache.getOrCompute('a', compute);

    expect(compute).toHaveBeenCalledTimes(1);
  });
});
