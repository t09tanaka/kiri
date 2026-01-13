import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { appStore } from './appStore';

describe('appStore', () => {
  beforeEach(() => {
    appStore.reset();
  });

  describe('initial state', () => {
    it('should have correct initial values', () => {
      const state = get(appStore);

      expect(state).toEqual({
        sidebarWidth: 220,
        showSidebar: true,
        currentMode: 'terminal',
        currentFile: null,
        sidebarMode: 'explorer',
      });
    });
  });

  describe('setSidebarWidth', () => {
    it('should set sidebar width', () => {
      appStore.setSidebarWidth(300);

      const state = get(appStore);
      expect(state.sidebarWidth).toBe(300);
    });

    it('should clamp width to minimum of 160', () => {
      appStore.setSidebarWidth(100);

      const state = get(appStore);
      expect(state.sidebarWidth).toBe(160);
    });

    it('should clamp width to maximum of 400', () => {
      appStore.setSidebarWidth(500);

      const state = get(appStore);
      expect(state.sidebarWidth).toBe(400);
    });
  });

  describe('toggleSidebar', () => {
    it('should toggle sidebar visibility', () => {
      expect(get(appStore).showSidebar).toBe(true);

      appStore.toggleSidebar();
      expect(get(appStore).showSidebar).toBe(false);

      appStore.toggleSidebar();
      expect(get(appStore).showSidebar).toBe(true);
    });
  });

  describe('showSidebar', () => {
    it('should set showSidebar to true', () => {
      appStore.hideSidebar();
      expect(get(appStore).showSidebar).toBe(false);

      appStore.showSidebar();
      expect(get(appStore).showSidebar).toBe(true);
    });
  });

  describe('hideSidebar', () => {
    it('should set showSidebar to false', () => {
      expect(get(appStore).showSidebar).toBe(true);

      appStore.hideSidebar();
      expect(get(appStore).showSidebar).toBe(false);
    });
  });

  describe('setMode', () => {
    it('should set current mode to terminal', () => {
      appStore.setMode('editor');
      appStore.setMode('terminal');

      expect(get(appStore).currentMode).toBe('terminal');
    });

    it('should set current mode to editor', () => {
      appStore.setMode('editor');

      expect(get(appStore).currentMode).toBe('editor');
    });
  });

  describe('setCurrentFile', () => {
    it('should set current file', () => {
      appStore.setCurrentFile('/path/to/file.ts');

      expect(get(appStore).currentFile).toBe('/path/to/file.ts');
    });

    it('should set current file to null', () => {
      appStore.setCurrentFile('/path/to/file.ts');
      appStore.setCurrentFile(null);

      expect(get(appStore).currentFile).toBe(null);
    });
  });

  describe('getUIForPersistence', () => {
    it('should return UI state for persistence', () => {
      appStore.setSidebarWidth(250);
      appStore.hideSidebar();

      const ui = appStore.getUIForPersistence();

      expect(ui).toEqual({
        sidebarWidth: 250,
        showSidebar: false,
        sidebarMode: 'explorer',
      });
    });
  });

  describe('restoreUI', () => {
    it('should restore UI state from persistence', () => {
      appStore.restoreUI({
        sidebarWidth: 300,
        showSidebar: false,
        sidebarMode: 'changes',
      });

      const state = get(appStore);
      expect(state.sidebarWidth).toBe(300);
      expect(state.showSidebar).toBe(false);
      // sidebarMode should always be 'explorer' regardless of persisted value
      expect(state.sidebarMode).toBe('explorer');
    });

    it('should clamp sidebar width on restore', () => {
      appStore.restoreUI({
        sidebarWidth: 500,
        showSidebar: true,
        sidebarMode: 'explorer',
      });

      expect(get(appStore).sidebarWidth).toBe(400);

      appStore.restoreUI({
        sidebarWidth: 100,
        showSidebar: true,
        sidebarMode: 'explorer',
      });

      expect(get(appStore).sidebarWidth).toBe(160);
    });
  });

  describe('reset', () => {
    it('should reset to initial state', () => {
      appStore.setSidebarWidth(300);
      appStore.hideSidebar();
      appStore.setMode('editor');
      appStore.setCurrentFile('/path/to/file.ts');

      appStore.reset();

      const state = get(appStore);
      expect(state).toEqual({
        sidebarWidth: 220,
        showSidebar: true,
        currentMode: 'terminal',
        currentFile: null,
        sidebarMode: 'explorer',
      });
    });
  });
});
