import { describe, it, expect } from 'vitest';
import { searchExtension } from './searchExtension';

describe('searchExtension', () => {
  it('should return an array of extensions', () => {
    const extensions = searchExtension();
    expect(Array.isArray(extensions)).toBe(true);
    expect(extensions.length).toBeGreaterThan(0);
  });

  it('should return consistent results on multiple calls', () => {
    const first = searchExtension();
    const second = searchExtension();
    expect(first.length).toBe(second.length);
  });
});
