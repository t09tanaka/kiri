import { beforeEach, describe, expect, it } from 'vitest';
import { focusedPaneStore } from './focusedPaneStore';

describe('focusedPaneStore', () => {
  beforeEach(() => focusedPaneStore.set(null));

  it('starts null', () => {
    expect(focusedPaneStore.current()).toBeNull();
  });

  it('set / current round-trip', () => {
    focusedPaneStore.set('abc');
    expect(focusedPaneStore.current()).toBe('abc');
  });

  it('subscribers see updates', () => {
    const seen: (string | null)[] = [];
    const unsub = focusedPaneStore.subscribe((v) => seen.push(v));
    focusedPaneStore.set('p1');
    focusedPaneStore.set(null);
    unsub();
    expect(seen).toEqual([null, 'p1', null]);
  });
});
