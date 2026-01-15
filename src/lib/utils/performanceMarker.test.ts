import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock performanceService
vi.mock('@/lib/services/performanceService', () => ({
  performanceService: {
    startOperation: vi.fn().mockReturnValue(vi.fn()),
    trackLongTask: vi.fn(),
  },
}));

// Import after mocking
import { performanceService } from '@/lib/services/performanceService';
import { measure, measureAsync, mark, measureBetween, withTiming } from './performanceMarker';

describe('performanceMarker', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('measure', () => {
    it('should execute the function and return result', () => {
      const result = measure('test', () => 42);
      expect(result).toBe(42);
    });

    it('should not throw for fast functions', () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      measure('fast-fn', () => 1 + 1);
      // Fast function should not trigger warning
      expect(warnSpy).not.toHaveBeenCalled();
      warnSpy.mockRestore();
    });

    it('should warn for slow functions (>16ms)', () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      // Create a slow synchronous function
      measure('slow-fn', () => {
        const start = Date.now();
        // eslint-disable-next-line no-empty
        while (Date.now() - start < 20) {}
        return 'done';
      });

      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('slow-fn'));
      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('> 1 frame'));
      warnSpy.mockRestore();
    });

    it('should preserve return type', () => {
      const objResult = measure('obj', () => ({ key: 'value' }));
      expect(objResult).toEqual({ key: 'value' });

      const arrResult = measure('arr', () => [1, 2, 3]);
      expect(arrResult).toEqual([1, 2, 3]);
    });
  });

  describe('measureAsync', () => {
    it('should execute async function and return result', async () => {
      const result = await measureAsync('async-test', async () => {
        return 'async result';
      });
      expect(result).toBe('async result');
    });

    it('should handle rejections', async () => {
      await expect(
        measureAsync('failing', async () => {
          throw new Error('test error');
        })
      ).rejects.toThrow('test error');
    });

    it('should warn for slow async functions (>100ms)', async () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      await measureAsync('slow-async', async () => {
        await new Promise((resolve) => setTimeout(resolve, 110));
        return 'done';
      });

      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('slow-async'));
      warnSpy.mockRestore();
    });
  });

  describe('mark', () => {
    it('should call performance.mark', () => {
      const markSpy = vi
        .spyOn(performance, 'mark')
        .mockImplementation(() => ({}) as PerformanceMark);
      mark('test-mark');
      expect(markSpy).toHaveBeenCalledWith('test-mark');
      markSpy.mockRestore();
    });

    it('should not throw if performance.mark fails', () => {
      vi.spyOn(performance, 'mark').mockImplementation(() => {
        throw new Error('Not supported');
      });
      expect(() => mark('failing-mark')).not.toThrow();
    });
  });

  describe('measureBetween', () => {
    it('should call performance.measure with two marks', () => {
      const measureSpy = vi
        .spyOn(performance, 'measure')
        .mockImplementation(() => ({}) as PerformanceMeasure);
      vi.spyOn(performance, 'getEntriesByName').mockReturnValue([
        { duration: 100 } as PerformanceEntry,
      ]);
      const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      measureBetween('duration', 'start-mark', 'end-mark');

      expect(measureSpy).toHaveBeenCalledWith('duration', 'start-mark', 'end-mark');
      expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('duration'));

      measureSpy.mockRestore();
      logSpy.mockRestore();
    });

    it('should handle missing end mark', () => {
      const measureSpy = vi
        .spyOn(performance, 'measure')
        .mockImplementation(() => ({}) as PerformanceMeasure);
      vi.spyOn(performance, 'getEntriesByName').mockReturnValue([]);

      measureBetween('single-mark', 'start-mark');

      expect(measureSpy).toHaveBeenCalledWith('single-mark', 'start-mark');
      measureSpy.mockRestore();
    });
  });

  describe('withTiming', () => {
    it('should wrap sync function and track timing', () => {
      const mockStop = vi.fn();
      vi.mocked(performanceService.startOperation).mockReturnValue(mockStop);

      const fn = (x: number) => x * 2;
      const wrapped = withTiming('multiply', fn);

      const result = wrapped(5);

      expect(result).toBe(10);
      expect(performanceService.startOperation).toHaveBeenCalledWith('multiply');
      expect(mockStop).toHaveBeenCalled();
    });

    it('should wrap async function and track timing', async () => {
      const mockStop = vi.fn();
      vi.mocked(performanceService.startOperation).mockReturnValue(mockStop);

      const fn = async (x: number) => x * 2;
      const wrapped = withTiming('async-multiply', fn);

      const result = await wrapped(5);

      expect(result).toBe(10);
      expect(performanceService.startOperation).toHaveBeenCalledWith('async-multiply');
      expect(mockStop).toHaveBeenCalled();
    });

    it('should call stop even if async function rejects', async () => {
      const mockStop = vi.fn();
      vi.mocked(performanceService.startOperation).mockReturnValue(mockStop);

      const fn = async () => {
        throw new Error('fail');
      };
      const wrapped = withTiming('failing-async', fn);

      await expect(wrapped()).rejects.toThrow('fail');
      expect(mockStop).toHaveBeenCalled();
    });
  });
});
