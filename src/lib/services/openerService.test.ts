import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';
import { openerService } from './openerService';

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}));

describe('openerService', () => {
  let mockOpenUrl: Mock;

  beforeEach(async () => {
    vi.clearAllMocks();
    const opener = await import('@tauri-apps/plugin-opener');
    mockOpenUrl = opener.openUrl as Mock;
  });

  describe('openUrl', () => {
    it('should call openUrl with the provided URL', async () => {
      const testUrl = 'https://example.com';
      await openerService.openUrl(testUrl);

      expect(mockOpenUrl).toHaveBeenCalledWith(testUrl);
      expect(mockOpenUrl).toHaveBeenCalledTimes(1);
    });

    it('should handle URLs with query parameters', async () => {
      const testUrl = 'https://example.com/search?q=test&page=1';
      await openerService.openUrl(testUrl);

      expect(mockOpenUrl).toHaveBeenCalledWith(testUrl);
    });

    it('should handle localhost URLs', async () => {
      const testUrl = 'http://localhost:3000/api/health';
      await openerService.openUrl(testUrl);

      expect(mockOpenUrl).toHaveBeenCalledWith(testUrl);
    });
  });
});
