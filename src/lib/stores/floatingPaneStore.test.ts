import { beforeEach, describe, expect, it } from 'vitest';
import { floatingPaneStore } from './floatingPaneStore';

describe('floatingPaneStore', () => {
  beforeEach(() => floatingPaneStore.close());

  it('starts with no floating pane', () => {
    expect(floatingPaneStore.current()).toBeNull();
  });

  it('open / current round-trip', () => {
    floatingPaneStore.open('pane-1');
    expect(floatingPaneStore.current()).toBe('pane-1');
  });

  it('open replaces the currently floating pane', () => {
    floatingPaneStore.open('pane-1');
    floatingPaneStore.open('pane-2');
    expect(floatingPaneStore.current()).toBe('pane-2');
  });

  it('close clears the floating pane', () => {
    floatingPaneStore.open('pane-1');
    floatingPaneStore.close();
    expect(floatingPaneStore.current()).toBeNull();
  });

  it('toggle floats a pane when none/another is floating', () => {
    floatingPaneStore.toggle('pane-1');
    expect(floatingPaneStore.current()).toBe('pane-1');
    floatingPaneStore.toggle('pane-2');
    expect(floatingPaneStore.current()).toBe('pane-2');
  });

  it('toggle dismisses the pane when it is already floating', () => {
    floatingPaneStore.open('pane-1');
    floatingPaneStore.toggle('pane-1');
    expect(floatingPaneStore.current()).toBeNull();
  });

  it('subscribers see updates', () => {
    const seen: (string | null)[] = [];
    const unsub = floatingPaneStore.subscribe((v) => seen.push(v));
    floatingPaneStore.open('p1');
    floatingPaneStore.close();
    unsub();
    expect(seen).toEqual([null, 'p1', null]);
  });
});
