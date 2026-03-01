import { describe, it, expect, vi, beforeEach } from 'vitest';

// Hoisted mock functions so they are available when vi.mock factory runs
const { mockIsPermissionGranted, mockRequestPermission, mockSendNotification } = vi.hoisted(() => ({
  mockIsPermissionGranted: vi.fn().mockResolvedValue(true),
  mockRequestPermission: vi.fn().mockResolvedValue('granted'),
  mockSendNotification: vi.fn().mockResolvedValue(undefined),
}));

// Mock the Tauri notification plugin
vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: mockIsPermissionGranted,
  requestPermission: mockRequestPermission,
  sendNotification: mockSendNotification,
}));

// Helper to get a fresh notificationService instance (resets module-level singleton)
async function createFreshService() {
  vi.resetModules();
  const mod = await import('./notificationService');
  return mod.notificationService;
}

// Import for parseNotifications/hasBel tests (these are stateless and safe to reuse)
import { notificationService } from './notificationService';

describe('notificationService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsPermissionGranted.mockResolvedValue(true);
    mockRequestPermission.mockResolvedValue('granted');
    mockSendNotification.mockResolvedValue(undefined);
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

    it('should use default title "Terminal" when OSC 777 title is empty', () => {
      const input = '\x1b]777;notify;;Some message\x07';
      const result = notificationService.parseNotifications(input);

      expect(result.notifications).toHaveLength(1);
      expect(result.notifications[0]).toEqual({
        title: 'Terminal',
        body: 'Some message',
      });
    });

    it('should ignore OSC 777 with empty message', () => {
      const input = '\x1b]777;notify;Title;\x07';
      const result = notificationService.parseNotifications(input);

      expect(result.notifications).toHaveLength(0);
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

  describe('init', () => {
    it('should check permission and set initialized on success', async () => {
      const service = await createFreshService();
      await service.init();

      expect(mockIsPermissionGranted).toHaveBeenCalledOnce();
      // Permission was already granted, so requestPermission should not be called
      expect(mockRequestPermission).not.toHaveBeenCalled();
    });

    it('should request permission when not already granted', async () => {
      mockIsPermissionGranted.mockResolvedValue(false);
      mockRequestPermission.mockResolvedValue('granted');

      const service = await createFreshService();
      await service.init();

      expect(mockIsPermissionGranted).toHaveBeenCalledOnce();
      expect(mockRequestPermission).toHaveBeenCalledOnce();
    });

    it('should handle permission denied after request', async () => {
      mockIsPermissionGranted.mockResolvedValue(false);
      mockRequestPermission.mockResolvedValue('denied');

      const service = await createFreshService();
      await service.init();

      expect(mockRequestPermission).toHaveBeenCalledOnce();
      // After init, notify should not send because permission is denied
      await service.notify({ title: 'Test', body: 'Message' });
      expect(mockSendNotification).not.toHaveBeenCalled();
    });

    it('should only initialize once (early return on second call)', async () => {
      const service = await createFreshService();
      await service.init();
      await service.init();

      // isPermissionGranted should only be called once
      expect(mockIsPermissionGranted).toHaveBeenCalledOnce();
    });

    it('should handle init error gracefully and still mark as initialized', async () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      mockIsPermissionGranted.mockRejectedValue(new Error('Plugin not available'));

      const service = await createFreshService();
      await service.init();

      expect(warnSpy).toHaveBeenCalledWith(
        'Failed to initialize notifications:',
        expect.any(Error)
      );

      // Should still be marked as initialized (won't try again)
      mockIsPermissionGranted.mockResolvedValue(true);
      await service.init();
      // isPermissionGranted should have been called only once (the failed one)
      expect(mockIsPermissionGranted).toHaveBeenCalledOnce();

      warnSpy.mockRestore();
    });
  });

  describe('notify', () => {
    it('should auto-initialize and send notification', async () => {
      const service = await createFreshService();

      await service.notify({ title: 'Test', body: 'Hello' });

      expect(mockIsPermissionGranted).toHaveBeenCalledOnce();
      expect(mockSendNotification).toHaveBeenCalledWith({
        title: 'Test',
        body: 'Hello',
      });
    });

    it('should include sound option when sound is true', async () => {
      const service = await createFreshService();

      await service.notify({ title: 'Alert', body: 'Ding', sound: true });

      expect(mockSendNotification).toHaveBeenCalledWith({
        title: 'Alert',
        body: 'Ding',
        sound: 'Ping',
      });
    });

    it('should not include sound option when sound is false', async () => {
      const service = await createFreshService();

      await service.notify({ title: 'Alert', body: 'Silent', sound: false });

      expect(mockSendNotification).toHaveBeenCalledWith({
        title: 'Alert',
        body: 'Silent',
      });
    });

    it('should not send notification when permission is not granted', async () => {
      const debugSpy = vi.spyOn(console, 'debug').mockImplementation(() => {});
      mockIsPermissionGranted.mockResolvedValue(false);
      mockRequestPermission.mockResolvedValue('denied');

      const service = await createFreshService();
      await service.notify({ title: 'Test', body: 'Blocked' });

      expect(mockSendNotification).not.toHaveBeenCalled();
      expect(debugSpy).toHaveBeenCalledWith('Notification permission not granted');

      debugSpy.mockRestore();
    });

    it('should handle sendNotification error gracefully', async () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      mockSendNotification.mockRejectedValue(new Error('Send failed'));

      const service = await createFreshService();
      await service.notify({ title: 'Test', body: 'Fail' });

      expect(warnSpy).toHaveBeenCalledWith('Failed to send notification:', expect.any(Error));

      warnSpy.mockRestore();
    });
  });

  describe('processTerminalOutput', () => {
    it('should return cleaned output and send notifications for OSC 9', async () => {
      const service = await createFreshService();

      const result = await service.processTerminalOutput('before\x1b]9;Task done\x07after');

      expect(result).toBe('beforeafter');
      expect(mockSendNotification).toHaveBeenCalledWith({
        title: 'Terminal',
        body: 'Task done',
      });
    });

    it('should send multiple notifications from terminal output', async () => {
      const service = await createFreshService();

      const result = await service.processTerminalOutput(
        '\x1b]9;First\x07text\x1b]777;notify;Title;Second\x07end'
      );

      expect(result).toBe('textend');
      expect(mockSendNotification).toHaveBeenCalledTimes(2);
      expect(mockSendNotification).toHaveBeenNthCalledWith(1, {
        title: 'Terminal',
        body: 'First',
      });
      expect(mockSendNotification).toHaveBeenNthCalledWith(2, {
        title: 'Title',
        body: 'Second',
      });
    });

    it('should return output unchanged when no notifications present', async () => {
      const service = await createFreshService();

      const result = await service.processTerminalOutput('plain output');

      expect(result).toBe('plain output');
      expect(mockSendNotification).not.toHaveBeenCalled();
    });
  });
});
