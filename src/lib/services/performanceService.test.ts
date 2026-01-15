import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock Tauri invoke - must return a Promise
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

// Import after mocking
import { invoke } from '@tauri-apps/api/core';
import { performanceService } from './performanceService';

describe('performanceService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset mock to return Promise
    vi.mocked(invoke).mockResolvedValue(undefined);
    performanceService.clear();
  });

  afterEach(() => {
    performanceService.clear();
  });

  describe('init', () => {
    it('should initialize without error', () => {
      expect(() => performanceService.init()).not.toThrow();
    });

    it('should log initialization message', () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      performanceService.init();
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining('Performance tracking initialized')
      );
      consoleSpy.mockRestore();
    });
  });

  describe('markStartupPhase', () => {
    it('should record startup phases', () => {
      performanceService.init();
      performanceService.markStartupPhase('test-phase-1');
      performanceService.markStartupPhase('test-phase-2');

      const metrics = performanceService.getStartupMetrics();
      expect(metrics.phases).toHaveProperty('test-phase-1');
      expect(metrics.phases).toHaveProperty('test-phase-2');
    });

    it('should log phase timing', () => {
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      performanceService.init();
      performanceService.markStartupPhase('my-phase');

      expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('my-phase'));
      consoleSpy.mockRestore();
    });
  });

  describe('getStartupMetrics', () => {
    it('should return total startup time', () => {
      performanceService.init();
      const metrics = performanceService.getStartupMetrics();

      expect(metrics.totalMs).toBeGreaterThanOrEqual(0);
      expect(typeof metrics.totalMs).toBe('number');
    });

    it('should return empty phases initially', () => {
      performanceService.init();
      const metrics = performanceService.getStartupMetrics();

      expect(metrics.phases).toEqual({});
    });
  });

  describe('startOperation', () => {
    it('should track operation timing', async () => {
      const stop = performanceService.startOperation('test-operation');

      // Simulate some work
      await new Promise((resolve) => setTimeout(resolve, 10));
      stop();

      const metrics = performanceService.exportMetrics();
      expect(metrics?.operations['test-operation']).toBeDefined();
      expect(metrics?.operations['test-operation'].length).toBe(1);
      expect(metrics?.operations['test-operation'][0]).toBeGreaterThanOrEqual(10);
    });

    it('should call invoke to record timing', async () => {
      vi.mocked(invoke).mockResolvedValue(undefined);

      const stop = performanceService.startOperation('backend-op');
      stop();

      // Wait for async invoke
      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(invoke).toHaveBeenCalledWith(
        'record_command_timing',
        expect.objectContaining({ command: 'backend-op' })
      );
    });

    it('should warn on slow operations', async () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      const stop = performanceService.startOperation('slow-op');
      // Simulate a slow operation (> 100ms)
      await new Promise((resolve) => setTimeout(resolve, 110));
      stop();

      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('Slow operation'));
      warnSpy.mockRestore();
    });
  });

  describe('trackLongTask', () => {
    it('should increment long task count', () => {
      performanceService.trackLongTask(60);
      performanceService.trackLongTask(80);

      const metrics = performanceService.exportMetrics();
      expect(metrics?.longTaskCount).toBe(2);
    });

    it('should log warning', () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      performanceService.trackLongTask(75);

      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('Long task detected'));
      warnSpy.mockRestore();
    });
  });

  describe('getMemoryMetrics', () => {
    it('should call invoke with correct command', async () => {
      const mockMetrics = { rss: 1000000, vms: 2000000, platform: 'darwin' };
      vi.mocked(invoke).mockResolvedValue(mockMetrics);

      const result = await performanceService.getMemoryMetrics();

      expect(invoke).toHaveBeenCalledWith('get_memory_metrics');
      expect(result).toEqual(mockMetrics);
    });

    it('should handle errors gracefully', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Backend unavailable'));
      const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      const result = await performanceService.getMemoryMetrics();

      expect(result).toBeNull();
      expect(errorSpy).toHaveBeenCalled();
      errorSpy.mockRestore();
    });
  });

  describe('getPerformanceReport', () => {
    it('should call invoke with correct command', async () => {
      const mockReport = {
        memory: { rss: 1000000, vms: 2000000, platform: 'darwin' },
        command_timings: [],
        app_uptime_ms: 5000,
      };
      vi.mocked(invoke).mockResolvedValue(mockReport);

      const result = await performanceService.getPerformanceReport();

      expect(invoke).toHaveBeenCalledWith('get_performance_report');
      expect(result).toEqual(mockReport);
    });
  });

  describe('exportMetrics', () => {
    it('should return all collected metrics', () => {
      performanceService.init();
      performanceService.markStartupPhase('phase-1');
      performanceService.trackLongTask(50);

      const stop = performanceService.startOperation('op-1');
      stop();

      const metrics = performanceService.exportMetrics();

      expect(metrics).toHaveProperty('startup');
      expect(metrics).toHaveProperty('operations');
      expect(metrics).toHaveProperty('longTaskCount');
      expect(metrics?.startup.phases).toHaveProperty('phase-1');
      expect(metrics?.operations['op-1']).toBeDefined();
      expect(metrics?.longTaskCount).toBe(1);
    });
  });

  describe('clear', () => {
    it('should reset all metrics', () => {
      performanceService.init();
      performanceService.markStartupPhase('test');
      performanceService.trackLongTask(50);
      const stop = performanceService.startOperation('test-op');
      stop();

      performanceService.clear();

      const metrics = performanceService.exportMetrics();
      expect(metrics?.startup.phases).toEqual({});
      expect(metrics?.operations).toEqual({});
      expect(metrics?.longTaskCount).toBe(0);
    });

    it('should call backend clear', () => {
      vi.mocked(invoke).mockResolvedValue(undefined);
      performanceService.clear();

      expect(invoke).toHaveBeenCalledWith('clear_performance_timings');
    });
  });

  describe('logSummary', () => {
    it('should log without error', async () => {
      vi.mocked(invoke).mockResolvedValue({
        memory: { rss: 50000000, vms: 100000000, platform: 'darwin' },
        command_timings: [],
        app_uptime_ms: 5000,
      });

      const groupSpy = vi.spyOn(console, 'group').mockImplementation(() => {});
      const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      const groupEndSpy = vi.spyOn(console, 'groupEnd').mockImplementation(() => {});

      performanceService.init();
      await performanceService.logSummary();

      expect(groupSpy).toHaveBeenCalledWith(expect.stringContaining('Performance Summary'));
      expect(groupEndSpy).toHaveBeenCalled();

      groupSpy.mockRestore();
      logSpy.mockRestore();
      groupEndSpy.mockRestore();
    });
  });
});
