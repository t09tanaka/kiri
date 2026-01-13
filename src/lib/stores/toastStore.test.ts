import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { toastStore } from './toastStore';

describe('toastStore', () => {
  beforeEach(() => {
    toastStore.clear();
  });

  describe('add', () => {
    it('should add a toast with default type and duration', () => {
      const id = toastStore.add('Test message');

      const toasts = get(toastStore);
      expect(toasts).toHaveLength(1);
      expect(toasts[0]).toMatchObject({
        id,
        message: 'Test message',
        type: 'info',
        duration: 3000,
      });
    });

    it('should add a toast with custom type and duration', () => {
      const id = toastStore.add('Error message', 'error', 5000);

      const toasts = get(toastStore);
      expect(toasts).toHaveLength(1);
      expect(toasts[0]).toMatchObject({
        id,
        message: 'Error message',
        type: 'error',
        duration: 5000,
      });
    });

    it('should add multiple toasts', () => {
      toastStore.add('First');
      toastStore.add('Second');
      toastStore.add('Third');

      const toasts = get(toastStore);
      expect(toasts).toHaveLength(3);
    });

    it('should generate unique IDs', () => {
      const id1 = toastStore.add('First');
      const id2 = toastStore.add('Second');

      expect(id1).not.toBe(id2);
    });
  });

  describe('remove', () => {
    it('should remove a toast by ID', () => {
      const id1 = toastStore.add('First');
      toastStore.add('Second');

      toastStore.remove(id1);

      const toasts = get(toastStore);
      expect(toasts).toHaveLength(1);
      expect(toasts[0].message).toBe('Second');
    });

    it('should do nothing when removing non-existent ID', () => {
      toastStore.add('First');

      toastStore.remove('non-existent');

      const toasts = get(toastStore);
      expect(toasts).toHaveLength(1);
    });
  });

  describe('clear', () => {
    it('should remove all toasts', () => {
      toastStore.add('First');
      toastStore.add('Second');
      toastStore.add('Third');

      toastStore.clear();

      const toasts = get(toastStore);
      expect(toasts).toHaveLength(0);
    });
  });

  describe('convenience methods', () => {
    it('info should add toast with info type', () => {
      toastStore.info('Info message');

      const toasts = get(toastStore);
      expect(toasts[0].type).toBe('info');
    });

    it('success should add toast with success type', () => {
      toastStore.success('Success message');

      const toasts = get(toastStore);
      expect(toasts[0].type).toBe('success');
    });

    it('warning should add toast with warning type', () => {
      toastStore.warning('Warning message');

      const toasts = get(toastStore);
      expect(toasts[0].type).toBe('warning');
    });

    it('error should add toast with error type', () => {
      toastStore.error('Error message');

      const toasts = get(toastStore);
      expect(toasts[0].type).toBe('error');
    });

    it('convenience methods should accept custom duration', () => {
      toastStore.info('Info', 1000);
      toastStore.success('Success', 2000);
      toastStore.warning('Warning', 3000);
      toastStore.error('Error', 4000);

      const toasts = get(toastStore);
      expect(toasts[0].duration).toBe(1000);
      expect(toasts[1].duration).toBe(2000);
      expect(toasts[2].duration).toBe(3000);
      expect(toasts[3].duration).toBe(4000);
    });
  });

  describe('subscribe', () => {
    it('should allow subscribing to store updates', () => {
      let lastValue: unknown = null;
      const unsubscribe = toastStore.subscribe((value) => {
        lastValue = value;
      });

      toastStore.add('Test');

      expect(Array.isArray(lastValue)).toBe(true);
      expect((lastValue as unknown[]).length).toBe(1);

      unsubscribe();
    });
  });
});
