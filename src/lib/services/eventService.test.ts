import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock @tauri-apps/api/event
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
  emit: vi.fn(),
}));

// Mock @tauri-apps/api/window
const mockWindowListen = vi.fn();
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    listen: mockWindowListen,
  })),
}));

import { listen, emit } from '@tauri-apps/api/event';
import { eventService } from './eventService';

describe('eventService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('listen', () => {
    it('should call Tauri listen with event name and handler', async () => {
      const mockUnlisten = vi.fn();
      vi.mocked(listen).mockResolvedValue(mockUnlisten);
      const handler = vi.fn();

      const unlisten = await eventService.listen('test-event', handler);

      expect(listen).toHaveBeenCalledWith('test-event', handler);
      expect(listen).toHaveBeenCalledTimes(1);
      expect(unlisten).toBe(mockUnlisten);
    });

    it('should pass typed event data through to handler', async () => {
      const mockUnlisten = vi.fn();
      vi.mocked(listen).mockResolvedValue(mockUnlisten);
      const handler = vi.fn();

      await eventService.listen<{ value: number }>('data-event', handler);

      expect(listen).toHaveBeenCalledWith('data-event', handler);
    });
  });

  describe('listenCurrentWindow', () => {
    it('should call getCurrentWindow().listen with event name and handler', async () => {
      const mockUnlisten = vi.fn();
      mockWindowListen.mockResolvedValue(mockUnlisten);
      const handler = vi.fn();

      const unlisten = await eventService.listenCurrentWindow('window-event', handler);

      expect(mockWindowListen).toHaveBeenCalledWith('window-event', handler);
      expect(mockWindowListen).toHaveBeenCalledTimes(1);
      expect(unlisten).toBe(mockUnlisten);
    });

    it('should scope events to the current window only', async () => {
      const mockUnlisten = vi.fn();
      mockWindowListen.mockResolvedValue(mockUnlisten);
      const handler = vi.fn();

      await eventService.listenCurrentWindow('scoped-event', handler);

      // Should use window.listen, not global listen
      expect(mockWindowListen).toHaveBeenCalledWith('scoped-event', handler);
      expect(listen).not.toHaveBeenCalled();
    });
  });

  describe('emit', () => {
    it('should call Tauri emit with event name and payload', async () => {
      vi.mocked(emit).mockResolvedValue(undefined);

      await eventService.emit('test-event', { message: 'hello' });

      expect(emit).toHaveBeenCalledWith('test-event', { message: 'hello' });
      expect(emit).toHaveBeenCalledTimes(1);
    });

    it('should handle string payload', async () => {
      vi.mocked(emit).mockResolvedValue(undefined);

      await eventService.emit('string-event', 'simple-payload');

      expect(emit).toHaveBeenCalledWith('string-event', 'simple-payload');
    });

    it('should handle null payload', async () => {
      vi.mocked(emit).mockResolvedValue(undefined);

      await eventService.emit('null-event', null);

      expect(emit).toHaveBeenCalledWith('null-event', null);
    });
  });
});
