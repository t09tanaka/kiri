import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { prViewStore, isPrViewOpen } from './prViewStore';

describe('prViewStore', () => {
  beforeEach(() => {
    prViewStore.close();
  });

  it('should start closed', () => {
    const state = get(prViewStore);
    expect(state.isOpen).toBe(false);
    expect(state.projectPath).toBeNull();
  });

  it('should open with project path', () => {
    prViewStore.open('/repo');
    const state = get(prViewStore);
    expect(state.isOpen).toBe(true);
    expect(state.projectPath).toBe('/repo');
  });

  it('should close and reset state', () => {
    prViewStore.open('/repo');
    prViewStore.close();
    const state = get(prViewStore);
    expect(state.isOpen).toBe(false);
    expect(state.projectPath).toBeNull();
  });

  it('should derive isPrViewOpen', () => {
    expect(get(isPrViewOpen)).toBe(false);
    prViewStore.open('/repo');
    expect(get(isPrViewOpen)).toBe(true);
  });
});
