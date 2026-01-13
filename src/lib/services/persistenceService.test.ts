import { describe, it, expect } from 'vitest';
import { createUIState, getDefaultUI } from './persistenceService';

describe('persistenceService helper functions', () => {
  describe('createUIState', () => {
    it('should create UI state with provided values', () => {
      const ui = createUIState(300, false, 'explorer');

      expect(ui).toEqual({
        sidebarWidth: 300,
        showSidebar: false,
        sidebarMode: 'explorer',
      });
    });

    it('should create UI state with default-like values', () => {
      const ui = createUIState(220, true, 'explorer');

      expect(ui).toEqual({
        sidebarWidth: 220,
        showSidebar: true,
        sidebarMode: 'explorer',
      });
    });

    it('should accept changes sidebar mode', () => {
      const ui = createUIState(250, true, 'changes');

      expect(ui.sidebarMode).toBe('changes');
    });
  });

  describe('getDefaultUI', () => {
    it('should return default UI state', () => {
      const ui = getDefaultUI();

      expect(ui).toEqual({
        sidebarWidth: 220,
        showSidebar: true,
        sidebarMode: 'explorer',
      });
    });

    it('should return a new object each time', () => {
      const ui1 = getDefaultUI();
      const ui2 = getDefaultUI();

      expect(ui1).not.toBe(ui2);
      expect(ui1).toEqual(ui2);
    });

    it('should not be affected by modifications', () => {
      const ui1 = getDefaultUI();
      ui1.sidebarWidth = 500;

      const ui2 = getDefaultUI();
      expect(ui2.sidebarWidth).toBe(220);
    });
  });
});
