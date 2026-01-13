import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { terminalRegistry, type TerminalInstance } from './terminalRegistry';

// Create mock terminal instance
function createMockInstance(id: number): TerminalInstance {
  return {
    terminal: {} as TerminalInstance['terminal'],
    fitAddon: {} as TerminalInstance['fitAddon'],
    terminalId: id,
    unlisten: vi.fn(),
  };
}

describe('terminalRegistry', () => {
  beforeEach(() => {
    // Clear the registry by removing all known instances
    const map = get(terminalRegistry);
    for (const key of map.keys()) {
      terminalRegistry.remove(key);
    }
  });

  describe('register', () => {
    it('should register a terminal instance', () => {
      const instance = createMockInstance(1);

      terminalRegistry.register('pane-1', instance);

      const registered = terminalRegistry.get('pane-1');
      expect(registered).toBe(instance);
    });

    it('should overwrite existing instance for same pane ID', () => {
      const instance1 = createMockInstance(1);
      const instance2 = createMockInstance(2);

      terminalRegistry.register('pane-1', instance1);
      terminalRegistry.register('pane-1', instance2);

      const registered = terminalRegistry.get('pane-1');
      expect(registered).toBe(instance2);
      expect(registered?.terminalId).toBe(2);
    });
  });

  describe('get', () => {
    it('should return registered instance', () => {
      const instance = createMockInstance(1);
      terminalRegistry.register('pane-1', instance);

      expect(terminalRegistry.get('pane-1')).toBe(instance);
    });

    it('should return undefined for non-existent pane', () => {
      expect(terminalRegistry.get('non-existent')).toBeUndefined();
    });
  });

  describe('has', () => {
    it('should return true for registered pane', () => {
      const instance = createMockInstance(1);
      terminalRegistry.register('pane-1', instance);

      expect(terminalRegistry.has('pane-1')).toBe(true);
    });

    it('should return false for non-existent pane', () => {
      expect(terminalRegistry.has('non-existent')).toBe(false);
    });
  });

  describe('remove', () => {
    it('should remove and return the instance', () => {
      const instance = createMockInstance(1);
      terminalRegistry.register('pane-1', instance);

      const removed = terminalRegistry.remove('pane-1');

      expect(removed).toBe(instance);
      expect(terminalRegistry.has('pane-1')).toBe(false);
    });

    it('should return undefined when removing non-existent pane', () => {
      const removed = terminalRegistry.remove('non-existent');

      expect(removed).toBeUndefined();
    });
  });

  describe('detach', () => {
    it('should return the instance but keep it in registry', () => {
      const instance = createMockInstance(1);
      terminalRegistry.register('pane-1', instance);

      const detached = terminalRegistry.detach('pane-1');

      expect(detached).toBe(instance);
      // Instance should still be in registry (detach preserves it)
      expect(terminalRegistry.has('pane-1')).toBe(true);
    });

    it('should return undefined for non-existent pane', () => {
      const detached = terminalRegistry.detach('non-existent');

      expect(detached).toBeUndefined();
    });
  });

  describe('subscribe', () => {
    it('should allow subscribing to registry changes', () => {
      let lastValue: Map<string, TerminalInstance> | null = null;
      const unsubscribe = terminalRegistry.subscribe((value) => {
        lastValue = value;
      });

      const instance = createMockInstance(1);
      terminalRegistry.register('pane-1', instance);

      expect(lastValue).toBeInstanceOf(Map);
      expect(lastValue?.has('pane-1')).toBe(true);

      unsubscribe();
    });
  });
});
