/**
 * Notification Service
 *
 * Handles desktop notifications from terminal output.
 * Detects OSC 9 (iTerm2) and OSC 777 (rxvt-unicode) escape sequences.
 *
 * OSC 9 format: \x1b]9;message\x07
 * OSC 777 format: \x1b]777;notify;title;message\x07
 */

import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';

// OSC escape sequence patterns
// OSC 9: iTerm2 notification - \x1b]9;message\x07 or \x1b]9;message\x1b\\
// eslint-disable-next-line no-control-regex
const OSC9_PATTERN = /\x1b\]9;([^\x07\x1b]*?)(?:\x07|\x1b\\)/g;

// OSC 777: rxvt-unicode notification - \x1b]777;notify;title;message\x07
// eslint-disable-next-line no-control-regex
const OSC777_PATTERN = /\x1b\]777;notify;([^;]*?);([^\x07\x1b]*?)(?:\x07|\x1b\\)/g;

// BEL character for simple beep notifications
const BEL_CHAR = '\x07';

interface NotificationOptions {
  title: string;
  body: string;
  sound?: boolean;
}

class NotificationService {
  private permissionGranted = false;
  private initialized = false;

  /**
   * Initialize notification permissions
   */
  async init(): Promise<void> {
    if (this.initialized) return;

    try {
      this.permissionGranted = await isPermissionGranted();

      if (!this.permissionGranted) {
        const permission = await requestPermission();
        this.permissionGranted = permission === 'granted';
      }

      this.initialized = true;
    } catch (error) {
      console.warn('Failed to initialize notifications:', error);
      this.initialized = true;
    }
  }

  /**
   * Send a desktop notification
   */
  async notify(options: NotificationOptions): Promise<void> {
    if (!this.initialized) {
      await this.init();
    }

    if (!this.permissionGranted) {
      console.debug('Notification permission not granted');
      return;
    }

    try {
      await sendNotification({
        title: options.title,
        body: options.body,
        // Add sound on macOS if requested
        ...(options.sound && { sound: 'Ping' }),
      });
    } catch (error) {
      console.warn('Failed to send notification:', error);
    }
  }

  /**
   * Parse terminal output for notification escape sequences
   * Returns stripped output and any notifications found
   */
  parseNotifications(data: string): {
    output: string;
    notifications: NotificationOptions[];
  } {
    const notifications: NotificationOptions[] = [];
    let output = data;

    // Parse OSC 9 (iTerm2 format)
    let match: RegExpExecArray | null;
    while ((match = OSC9_PATTERN.exec(data)) !== null) {
      const message = match[1];
      if (message) {
        notifications.push({
          title: 'Terminal',
          body: message,
        });
      }
    }
    // Remove OSC 9 sequences from output
    output = output.replace(OSC9_PATTERN, '');

    // Reset regex lastIndex
    OSC777_PATTERN.lastIndex = 0;

    // Parse OSC 777 (rxvt-unicode format)
    while ((match = OSC777_PATTERN.exec(data)) !== null) {
      const title = match[1] || 'Terminal';
      const message = match[2];
      if (message) {
        notifications.push({
          title,
          body: message,
        });
      }
    }
    // Remove OSC 777 sequences from output
    output = output.replace(OSC777_PATTERN, '');

    // Reset regex lastIndex
    OSC9_PATTERN.lastIndex = 0;

    return { output, notifications };
  }

  /**
   * Process terminal output and send any detected notifications
   * Returns the cleaned output with notification sequences removed
   */
  async processTerminalOutput(data: string): Promise<string> {
    const { output, notifications } = this.parseNotifications(data);

    // Send all detected notifications
    for (const notification of notifications) {
      await this.notify(notification);
    }

    return output;
  }

  /**
   * Check if a BEL character is present (for simple beep)
   */
  hasBel(data: string): boolean {
    return data.includes(BEL_CHAR);
  }
}

export const notificationService = new NotificationService();
