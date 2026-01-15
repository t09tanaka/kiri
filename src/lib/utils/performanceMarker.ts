/**
 * Performance measurement utilities (development only)
 *
 * Provides helper functions for measuring execution time and detecting long tasks.
 * All functionality is no-op in production builds.
 */

import { performanceService } from '@/lib/services/performanceService';

const isDev = import.meta.env.DEV;

/**
 * Measure the execution time of a synchronous function
 * Logs a warning if execution exceeds 16ms (one frame at 60fps)
 *
 * @param name - Name for logging
 * @param fn - Function to measure
 * @returns Result of the function
 */
export function measure<T>(name: string, fn: () => T): T {
  if (!isDev) return fn();

  const start = performance.now();
  const result = fn();
  const duration = performance.now() - start;

  if (duration > 16) {
    console.warn(`[Perf] ${name}: ${duration.toFixed(1)}ms (> 1 frame)`);
  }

  return result;
}

/**
 * Measure the execution time of an asynchronous function
 * Logs a warning if execution exceeds 100ms
 *
 * @param name - Name for logging
 * @param fn - Async function to measure
 * @returns Promise resolving to the function's result
 */
export async function measureAsync<T>(name: string, fn: () => Promise<T>): Promise<T> {
  if (!isDev) return fn();

  const start = performance.now();
  const result = await fn();
  const duration = performance.now() - start;

  if (duration > 100) {
    console.warn(`[Perf] ${name}: ${duration.toFixed(1)}ms`);
  }

  return result;
}

/**
 * Setup the Long Task observer to detect UI-blocking tasks
 * Reports tasks that take longer than 50ms
 *
 * @returns Cleanup function to disconnect the observer
 */
export function setupLongTaskObserver(): () => void {
  if (!isDev) {
    return () => {};
  }

  if (!('PerformanceObserver' in window)) {
    console.warn('[Perf] PerformanceObserver not available');
    return () => {};
  }

  try {
    const observer = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        if (entry.duration > 50) {
          performanceService.trackLongTask(entry.duration);
        }
      }
    });

    observer.observe({ entryTypes: ['longtask'] });

    return () => observer.disconnect();
  } catch (error) {
    // longtask entryType might not be supported in all browsers
    console.warn('[Perf] Long task observer not supported:', error);
    return () => {};
  }
}

/**
 * Create a performance marker for profiling
 * Uses the Performance API's mark and measure features
 *
 * @param name - Marker name
 */
export function mark(name: string): void {
  if (!isDev) return;

  try {
    performance.mark(name);
  } catch {
    // Ignore if not supported
  }
}

/**
 * Measure time between two marks
 *
 * @param name - Measurement name
 * @param startMark - Start mark name
 * @param endMark - End mark name (defaults to current time)
 */
export function measureBetween(name: string, startMark: string, endMark?: string): void {
  if (!isDev) return;

  try {
    if (endMark) {
      performance.measure(name, startMark, endMark);
    } else {
      performance.measure(name, startMark);
    }

    const entries = performance.getEntriesByName(name, 'measure');
    const entry = entries[entries.length - 1];
    if (entry) {
      console.log(`[Perf] ${name}: ${entry.duration.toFixed(1)}ms`);
    }
  } catch {
    // Ignore if marks don't exist or not supported
  }
}

/**
 * Decorator-style function wrapper for measuring execution time
 * Useful for wrapping service methods
 *
 * @param name - Operation name
 * @param fn - Function to wrap
 * @returns Wrapped function that tracks timing
 */
export function withTiming<T extends (...args: unknown[]) => unknown>(name: string, fn: T): T {
  if (!isDev) return fn;

  return ((...args: Parameters<T>) => {
    const stop = performanceService.startOperation(name);
    const result = fn(...args);

    if (result instanceof Promise) {
      return result.finally(stop);
    }

    stop();
    return result;
  }) as T;
}
