import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { settingsStore, fontSize, FONT_SIZE_CONSTRAINTS } from './settingsStore';

describe('settingsStore', () => {
  beforeEach(() => {
    settingsStore.reset();
  });

  describe('initial state', () => {
    it('should have default font size', () => {
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT);
    });
  });

  describe('zoomIn', () => {
    it('should increase font size by step', () => {
      settingsStore.zoomIn();
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT + FONT_SIZE_CONSTRAINTS.STEP);
    });

    it('should not exceed max font size', () => {
      // Set to max
      settingsStore.setFontSize(FONT_SIZE_CONSTRAINTS.MAX);
      settingsStore.zoomIn();
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.MAX);
    });

    it('should increase multiple times', () => {
      settingsStore.zoomIn();
      settingsStore.zoomIn();
      settingsStore.zoomIn();
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT + 3 * FONT_SIZE_CONSTRAINTS.STEP);
    });
  });

  describe('zoomOut', () => {
    it('should decrease font size by step', () => {
      settingsStore.zoomOut();
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT - FONT_SIZE_CONSTRAINTS.STEP);
    });

    it('should not go below min font size', () => {
      // Set to min
      settingsStore.setFontSize(FONT_SIZE_CONSTRAINTS.MIN);
      settingsStore.zoomOut();
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.MIN);
    });

    it('should decrease multiple times', () => {
      settingsStore.zoomOut();
      settingsStore.zoomOut();
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT - 2 * FONT_SIZE_CONSTRAINTS.STEP);
    });
  });

  describe('resetZoom', () => {
    it('should reset font size to default', () => {
      settingsStore.zoomIn();
      settingsStore.zoomIn();
      settingsStore.resetZoom();
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT);
    });

    it('should reset from below default', () => {
      settingsStore.zoomOut();
      settingsStore.zoomOut();
      settingsStore.resetZoom();
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT);
    });
  });

  describe('setFontSize', () => {
    it('should set font size to specific value', () => {
      settingsStore.setFontSize(16);
      const state = get(settingsStore);
      expect(state.fontSize).toBe(16);
    });

    it('should clamp to min when below minimum', () => {
      settingsStore.setFontSize(1);
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.MIN);
    });

    it('should clamp to max when above maximum', () => {
      settingsStore.setFontSize(100);
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.MAX);
    });
  });

  describe('getFontSize', () => {
    it('should return current font size', () => {
      expect(settingsStore.getFontSize()).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT);
      settingsStore.zoomIn();
      expect(settingsStore.getFontSize()).toBe(
        FONT_SIZE_CONSTRAINTS.DEFAULT + FONT_SIZE_CONSTRAINTS.STEP
      );
    });
  });

  describe('getStateForPersistence', () => {
    it('should return current state', () => {
      settingsStore.setFontSize(16);
      const state = settingsStore.getStateForPersistence();
      expect(state).toEqual({ fontSize: 16 });
    });
  });

  describe('restoreState', () => {
    it('should restore font size from persisted state', () => {
      settingsStore.restoreState({ fontSize: 20 });
      const state = get(settingsStore);
      expect(state.fontSize).toBe(20);
    });

    it('should use default when font size is not provided', () => {
      settingsStore.setFontSize(20);
      settingsStore.restoreState({});
      const state = get(settingsStore);
      expect(state.fontSize).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT);
    });
  });

  describe('derived store: fontSize', () => {
    it('should reflect current font size', () => {
      expect(get(fontSize)).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT);
      settingsStore.zoomIn();
      expect(get(fontSize)).toBe(FONT_SIZE_CONSTRAINTS.DEFAULT + FONT_SIZE_CONSTRAINTS.STEP);
    });
  });

  describe('FONT_SIZE_CONSTRAINTS', () => {
    it('should have correct values', () => {
      expect(FONT_SIZE_CONSTRAINTS.MIN).toBe(8);
      expect(FONT_SIZE_CONSTRAINTS.MAX).toBe(32);
      expect(FONT_SIZE_CONSTRAINTS.DEFAULT).toBe(13);
      expect(FONT_SIZE_CONSTRAINTS.STEP).toBe(1);
    });
  });
});
