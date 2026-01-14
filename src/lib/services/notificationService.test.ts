import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the Tauri notification plugin
vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true),
  requestPermission: vi.fn().mockResolvedValue('granted'),
  sendNotification: vi.fn().mockResolvedValue(undefined),
}));

// Import after mocking
import { notificationService } from './notificationService';

describe('notificationService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('parseNotifications', () => {
    it('should parse OSC 9 notifications (iTerm2 format)', () => {
      const input = 'Hello\x1b]9;Task completed!\x07World';
      const result = notificationService.parseNotifications(input);

      expect(result.notifications).toHaveLength(1);
      expect(result.notifications[0]).toEqual({
        title: 'Terminal',
        body: 'Task completed!',
      });
      expect(result.output).toBe('HelloWorld');
    });

    it('should parse OSC 9 with ST terminator', () => {
      const input = 'Before\x1b]9;Message here\x1b\\After';
      const result = notificationService.parseNotifications(input);

      expect(result.notifications).toHaveLength(1);
      expect(result.notifications[0].body).toBe('Message here');
      expect(result.output).toBe('BeforeAfter');
    });

    it('should parse OSC 777 notifications (rxvt format)', () => {
      const input = 'Start\x1b]777;notify;Build Status;Build succeeded!\x07End';
      const result = notificationService.parseNotifications(input);

      expect(result.notifications).toHaveLength(1);
      expect(result.notifications[0]).toEqual({
        title: 'Build Status',
        body: 'Build succeeded!',
      });
      expect(result.output).toBe('StartEnd');
    });

    it('should parse multiple notifications', () => {
      const input = '\x1b]9;First\x07middle\x1b]777;notify;Title;Second\x07end';
      const result = notificationService.parseNotifications(input);

      expect(result.notifications).toHaveLength(2);
      expect(result.notifications[0].body).toBe('First');
      expect(result.notifications[1].body).toBe('Second');
      expect(result.output).toBe('middleend');
    });

    it('should return empty notifications for regular output', () => {
      const input = 'Regular terminal output with no notifications';
      const result = notificationService.parseNotifications(input);

      expect(result.notifications).toHaveLength(0);
      expect(result.output).toBe(input);
    });

    it('should handle empty message in OSC 9', () => {
      const input = '\x1b]9;\x07';
      const result = notificationService.parseNotifications(input);

      // Empty message should not create a notification
      expect(result.notifications).toHaveLength(0);
    });

    it('should preserve other escape sequences', () => {
      const input = '\x1b[32mGreen\x1b[0m\x1b]9;Notify\x07Normal';
      const result = notificationService.parseNotifications(input);

      expect(result.notifications).toHaveLength(1);
      expect(result.output).toBe('\x1b[32mGreen\x1b[0mNormal');
    });
  });

  describe('hasBel', () => {
    it('should detect BEL character', () => {
      expect(notificationService.hasBel('Hello\x07World')).toBe(true);
    });

    it('should return false when no BEL', () => {
      expect(notificationService.hasBel('Hello World')).toBe(false);
    });
  });
});
