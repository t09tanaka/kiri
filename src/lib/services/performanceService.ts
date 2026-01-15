/**
 * Performance measurement service (development only)
 *
 * Provides utilities for measuring and tracking performance metrics.
 * All functionality is no-op in production builds (tree-shaken away).
 */
import { invoke } from '@tauri-apps/api/core';

const isDev = import.meta.env.DEV;

/**
 * Memory metrics from the Rust backend
 */
export interface MemoryMetrics {
  /** Resident Set Size in bytes */
  rss: number;
  /** Virtual memory size in bytes */
  vms: number;
  /** Platform identifier */
  platform: string;
}

/**
 * Command timing entry
 */
export interface CommandTiming {
  /** Command name */
  command: string;
  /** Duration in milliseconds */
  duration_ms: number;
  /** Timestamp since app start (ms) */
  timestamp_ms: number;
}

/**
 * Full performance report from the backend
 */
export interface PerformanceReport {
  /** Memory metrics */
  memory: MemoryMetrics;
  /** Command timings */
  command_timings: CommandTiming[];
  /** App uptime in milliseconds */
  app_uptime_ms: number;
}

/**
 * Startup metrics from frontend
 */
export interface StartupMetrics {
  /** Total startup time in ms */
  totalMs: number;
  /** Phase timings (phase name -> ms since init) */
  phases: Record<string, number>;
}

// Internal state (dev only)
const startupPhases = new Map<string, number>();
const operationTimings = new Map<string, number[]>();
let appStartTime = 0;
let longTaskCount = 0;

/**
 * Performance service for development-time metrics tracking
 */
export const performanceService = {
  /**
   * Initialize performance tracking
   * Call this once at app startup (in main.ts)
   */
  init: (): void => {
    if (!isDev) return;
    appStartTime = performance.now();
    console.log('[Perf] Performance tracking initialized');
  },

  /**
   * Get memory metrics from the Rust backend
   */
  getMemoryMetrics: async (): Promise<MemoryMetrics | null> => {
    if (!isDev) return null;
    try {
      return await invoke<MemoryMetrics>('get_memory_metrics');
    } catch (error) {
      console.error('[Perf] Failed to get memory metrics:', error);
      return null;
    }
  },

  /**
   * Get full performance report from the backend
   */
  getPerformanceReport: async (): Promise<PerformanceReport | null> => {
    if (!isDev) return null;
    try {
      return await invoke<PerformanceReport>('get_performance_report');
    } catch (error) {
      console.error('[Perf] Failed to get performance report:', error);
      return null;
    }
  },

  /**
   * Mark a startup phase
   * @param phase - Name of the phase (e.g., 'main-start', 'app-mount-complete')
   */
  markStartupPhase: (phase: string): void => {
    if (!isDev) return;
    const now = performance.now();
    startupPhases.set(phase, now);
    const elapsed = now - appStartTime;
    console.log(`[Perf] ${phase}: ${elapsed.toFixed(1)}ms`);
  },

  /**
   * Get startup metrics
   */
  getStartupMetrics: (): StartupMetrics => {
    const phases: Record<string, number> = {};
    startupPhases.forEach((time, name) => {
      phases[name] = time - appStartTime;
    });
    return {
      totalMs: performance.now() - appStartTime,
      phases,
    };
  },

  /**
   * Start tracking an operation
   * Returns a function to call when the operation completes
   * @param name - Operation name
   * @returns Stop function that records the duration
   */
  startOperation: (name: string): (() => void) => {
    if (!isDev) return () => {};
    const start = performance.now();
    return () => {
      const duration = performance.now() - start;
      const timings = operationTimings.get(name) || [];
      timings.push(duration);
      operationTimings.set(name, timings);

      // Warn about slow operations
      if (duration > 100) {
        console.warn(`[Perf] Slow operation: ${name} took ${duration.toFixed(1)}ms`);
      }

      // Also record to backend
      invoke('record_command_timing', { command: name, durationMs: duration }).catch(() => {
        // Ignore errors - backend might not be available
      });
    };
  },

  /**
   * Track a long task (UI blocking > 50ms)
   * @param duration - Duration in milliseconds
   */
  trackLongTask: (duration: number): void => {
    if (!isDev) return;
    longTaskCount++;
    console.warn(`[Perf] Long task detected: ${duration.toFixed(1)}ms (total: ${longTaskCount})`);
  },

  /**
   * Get all collected metrics
   */
  exportMetrics: (): {
    startup: StartupMetrics;
    operations: Record<string, number[]>;
    longTaskCount: number;
  } | null => {
    if (!isDev) return null;
    return {
      startup: performanceService.getStartupMetrics(),
      operations: Object.fromEntries(operationTimings),
      longTaskCount,
    };
  },

  /**
   * Log a summary of all metrics to the console
   */
  logSummary: async (): Promise<void> => {
    if (!isDev) return;

    const frontendMetrics = performanceService.exportMetrics();
    const backendReport = await performanceService.getPerformanceReport();

    console.group('[Perf] Performance Summary');

    // Startup metrics
    console.log('Startup phases:', frontendMetrics?.startup.phases);
    console.log(`Total startup: ${frontendMetrics?.startup.totalMs.toFixed(1)}ms`);

    // Memory metrics
    if (backendReport?.memory) {
      const rssMB = (backendReport.memory.rss / 1024 / 1024).toFixed(1);
      const vmsMB = (backendReport.memory.vms / 1024 / 1024).toFixed(1);
      console.log(`Memory: RSS=${rssMB}MB, VMS=${vmsMB}MB`);
    }

    // Operation timings
    if (frontendMetrics?.operations) {
      const ops = Object.entries(frontendMetrics.operations);
      if (ops.length > 0) {
        console.log('Operations:');
        ops.forEach(([name, timings]) => {
          const avg = timings.reduce((a, b) => a + b, 0) / timings.length;
          const max = Math.max(...timings);
          console.log(
            `  ${name}: avg=${avg.toFixed(1)}ms, max=${max.toFixed(1)}ms, count=${timings.length}`
          );
        });
      }
    }

    // Long tasks
    console.log(`Long tasks (>50ms): ${frontendMetrics?.longTaskCount || 0}`);

    console.groupEnd();
  },

  /**
   * Clear all recorded timings (for testing)
   */
  clear: (): void => {
    if (!isDev) return;
    startupPhases.clear();
    operationTimings.clear();
    longTaskCount = 0;
    invoke('clear_performance_timings').catch(() => {});
  },
};
