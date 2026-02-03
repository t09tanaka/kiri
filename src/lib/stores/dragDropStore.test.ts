import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import { dragDropStore, isDragging, draggedPaths, dropTargetPath } from './dragDropStore';

describe('dragDropStore', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    dragDropStore.endDrag();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('startDrag', () => {
    it('should set isDragging to true', () => {
      dragDropStore.startDrag(['/path/to/file.txt']);
      expect(get(isDragging)).toBe(true);
    });

    it('should set draggedPaths', () => {
      dragDropStore.startDrag(['/path/to/file1.txt', '/path/to/file2.txt']);
      expect(get(draggedPaths)).toEqual(['/path/to/file1.txt', '/path/to/file2.txt']);
    });

    it('should reset dropTargetPath', () => {
      dragDropStore.setDropTarget('/some/path');
      dragDropStore.startDrag(['/path/to/file.txt']);
      expect(get(dropTargetPath)).toBe(null);
    });
  });

  describe('endDrag', () => {
    it('should reset all state', () => {
      dragDropStore.startDrag(['/path/to/file.txt']);
      dragDropStore.setDropTarget('/target/path');
      dragDropStore.endDrag();

      expect(get(isDragging)).toBe(false);
      expect(get(draggedPaths)).toEqual([]);
      expect(get(dropTargetPath)).toBe(null);
    });

    it('should clear all hover timers', () => {
      dragDropStore.startDrag(['/path/to/file.txt']);
      const callback = vi.fn();
      dragDropStore.startHoverTimer('/path1', callback);
      dragDropStore.startHoverTimer('/path2', callback);

      dragDropStore.endDrag();

      vi.advanceTimersByTime(3000);
      expect(callback).not.toHaveBeenCalled();
    });
  });

  describe('setDropTarget', () => {
    it('should set dropTargetPath', () => {
      dragDropStore.setDropTarget('/target/directory');
      expect(get(dropTargetPath)).toBe('/target/directory');
    });

    it('should allow setting to null', () => {
      dragDropStore.setDropTarget('/target/directory');
      dragDropStore.setDropTarget(null);
      expect(get(dropTargetPath)).toBe(null);
    });
  });

  describe('startHoverTimer', () => {
    it('should call callback after 2 seconds', () => {
      const callback = vi.fn();
      dragDropStore.startHoverTimer('/path', callback);

      expect(callback).not.toHaveBeenCalled();

      vi.advanceTimersByTime(2000);
      expect(callback).toHaveBeenCalledTimes(1);
    });

    it('should not call callback before 2 seconds', () => {
      const callback = vi.fn();
      dragDropStore.startHoverTimer('/path', callback);

      vi.advanceTimersByTime(1999);
      expect(callback).not.toHaveBeenCalled();
    });

    it('should replace existing timer for same path', () => {
      const callback1 = vi.fn();
      const callback2 = vi.fn();

      dragDropStore.startHoverTimer('/path', callback1);
      vi.advanceTimersByTime(1000);

      dragDropStore.startHoverTimer('/path', callback2);
      vi.advanceTimersByTime(2000);

      expect(callback1).not.toHaveBeenCalled();
      expect(callback2).toHaveBeenCalledTimes(1);
    });
  });

  describe('clearHoverTimer', () => {
    it('should prevent callback from being called', () => {
      const callback = vi.fn();
      dragDropStore.startHoverTimer('/path', callback);

      vi.advanceTimersByTime(1000);
      dragDropStore.clearHoverTimer('/path');
      vi.advanceTimersByTime(2000);

      expect(callback).not.toHaveBeenCalled();
    });

    it('should handle non-existent timer gracefully', () => {
      expect(() => dragDropStore.clearHoverTimer('/nonexistent')).not.toThrow();
    });
  });

  describe('clearAllHoverTimers', () => {
    it('should clear all timers', () => {
      const callback1 = vi.fn();
      const callback2 = vi.fn();

      dragDropStore.startHoverTimer('/path1', callback1);
      dragDropStore.startHoverTimer('/path2', callback2);

      dragDropStore.clearAllHoverTimers();
      vi.advanceTimersByTime(3000);

      expect(callback1).not.toHaveBeenCalled();
      expect(callback2).not.toHaveBeenCalled();
    });
  });

  describe('getState', () => {
    it('should return current state', () => {
      dragDropStore.startDrag(['/file.txt']);
      dragDropStore.setDropTarget('/target');

      const state = dragDropStore.getState();

      expect(state.isDragging).toBe(true);
      expect(state.draggedPaths).toEqual(['/file.txt']);
      expect(state.dropTargetPath).toBe('/target');
    });
  });

  describe('hasHoverTimer', () => {
    it('should return true if timer exists', () => {
      dragDropStore.startHoverTimer('/path', vi.fn());
      expect(dragDropStore.hasHoverTimer('/path')).toBe(true);
    });

    it('should return false if timer does not exist', () => {
      expect(dragDropStore.hasHoverTimer('/path')).toBe(false);
    });

    it('should return false after timer fires', () => {
      dragDropStore.startHoverTimer('/path', vi.fn());
      vi.advanceTimersByTime(2000);
      expect(dragDropStore.hasHoverTimer('/path')).toBe(false);
    });
  });

  describe('derived stores', () => {
    it('isDragging should reflect store state', () => {
      expect(get(isDragging)).toBe(false);
      dragDropStore.startDrag(['/file.txt']);
      expect(get(isDragging)).toBe(true);
      dragDropStore.endDrag();
      expect(get(isDragging)).toBe(false);
    });

    it('draggedPaths should reflect store state', () => {
      expect(get(draggedPaths)).toEqual([]);
      dragDropStore.startDrag(['/file1.txt', '/file2.txt']);
      expect(get(draggedPaths)).toEqual(['/file1.txt', '/file2.txt']);
    });

    it('dropTargetPath should reflect store state', () => {
      expect(get(dropTargetPath)).toBe(null);
      dragDropStore.setDropTarget('/target');
      expect(get(dropTargetPath)).toBe('/target');
    });
  });
});
