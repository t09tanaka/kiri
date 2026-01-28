import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { diffViewStore, isDiffViewOpen } from './diffViewStore';

describe('diffViewStore', () => {
  beforeEach(() => {
    // Reset the store to initial state before each test
    diffViewStore.close();
  });

  describe('initial state', () => {
    it('should start with isOpen as false', () => {
      const state = get(diffViewStore);
      expect(state.isOpen).toBe(false);
    });

    it('should start with projectPath as null', () => {
      const state = get(diffViewStore);
      expect(state.projectPath).toBe(null);
    });
  });

  describe('open', () => {
    it('should set isOpen to true', () => {
      diffViewStore.open('/path/to/project');
      const state = get(diffViewStore);
      expect(state.isOpen).toBe(true);
    });

    it('should set projectPath to the provided path', () => {
      diffViewStore.open('/path/to/project');
      const state = get(diffViewStore);
      expect(state.projectPath).toBe('/path/to/project');
    });

    it('should handle paths with special characters', () => {
      diffViewStore.open('/path/to/my project/with spaces');
      const state = get(diffViewStore);
      expect(state.projectPath).toBe('/path/to/my project/with spaces');
    });

    it('should update isDiffViewOpen derived store', () => {
      expect(get(isDiffViewOpen)).toBe(false);
      diffViewStore.open('/path/to/project');
      expect(get(isDiffViewOpen)).toBe(true);
    });
  });

  describe('close', () => {
    it('should reset to initial state', () => {
      diffViewStore.open('/path/to/project');
      diffViewStore.close();

      const state = get(diffViewStore);
      expect(state.isOpen).toBe(false);
      expect(state.projectPath).toBe(null);
    });

    it('should update isDiffViewOpen derived store', () => {
      diffViewStore.open('/path/to/project');
      expect(get(isDiffViewOpen)).toBe(true);

      diffViewStore.close();
      expect(get(isDiffViewOpen)).toBe(false);
    });
  });

  describe('subscribe', () => {
    it('should notify subscribers when state changes', () => {
      const states: Array<{ isOpen: boolean; projectPath: string | null }> = [];
      const unsubscribe = diffViewStore.subscribe((state) => {
        states.push({ ...state });
      });

      diffViewStore.open('/project1');
      diffViewStore.close();

      expect(states.length).toBe(3); // initial + open + close
      expect(states[0]).toEqual({ isOpen: false, projectPath: null });
      expect(states[1]).toEqual({ isOpen: true, projectPath: '/project1' });
      expect(states[2]).toEqual({ isOpen: false, projectPath: null });

      unsubscribe();
    });
  });

  describe('isDiffViewOpen derived store', () => {
    it('should derive correctly from diffViewStore state', () => {
      const values: boolean[] = [];
      const unsubscribe = isDiffViewOpen.subscribe((value) => {
        values.push(value);
      });

      diffViewStore.open('/project');
      diffViewStore.close();
      diffViewStore.open('/another');

      expect(values).toEqual([false, true, false, true]);

      unsubscribe();
    });
  });
});
