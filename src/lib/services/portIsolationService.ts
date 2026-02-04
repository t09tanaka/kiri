import { invoke } from '@tauri-apps/api/core';
import type { CopyResult } from './worktreeService';

export interface PortSource {
  file_path: string;
  variable_name: string;
  port_value: number;
  line_number: number;
}

export interface DetectedPorts {
  env_ports: PortSource[];
  dockerfile_ports: PortSource[];
  compose_ports: PortSource[];
}

export interface PortAssignment {
  variable_name: string;
  original_value: number;
  assigned_value: number;
}

export interface PortAllocationResult {
  assignments: PortAssignment[];
  next_port: number;
}

export interface CustomPortRule {
  id: string;
  file_pattern: string;
  search_pattern: string;
  enabled: boolean;
}

export interface CustomRuleReplacement {
  file_path: string;
  original_value: number;
  new_value: number;
  line_number: number;
}

export interface PortConfig {
  enabled: boolean;
  port_range_start: number;
  port_range_end: number;
  next_port: number;
  worktree_assignments: Record<string, WorktreePortAssignment>;
  custom_rules: CustomPortRule[];
}

export interface WorktreePortAssignment {
  worktree_name: string;
  assignments: PortAssignment[];
  created_at: string;
}

// Default port range
export const DEFAULT_PORT_RANGE_START = 20000;
export const DEFAULT_PORT_RANGE_END = 39999;

/**
 * Port isolation service for worktrees
 * Wraps Tauri port isolation commands
 */
export const portIsolationService = {
  /**
   * Detect ports in a directory (scans .env files, Dockerfile, docker-compose.yml)
   */
  detectPorts: (dirPath: string): Promise<DetectedPorts> => invoke('detect_ports', { dirPath }),

  /**
   * Allocate unique ports for the given port sources
   */
  allocatePorts: (ports: PortSource[], startPort: number): Promise<PortAllocationResult> =>
    invoke('allocate_worktree_ports', { ports, startPort }),

  /**
   * Copy files with port transformation
   * .env files will have their port values replaced according to assignments
   */
  copyFilesWithPorts: (
    sourcePath: string,
    targetPath: string,
    patterns: string[],
    assignments: PortAssignment[]
  ): Promise<CopyResult> =>
    invoke('copy_files_with_ports', { sourcePath, targetPath, patterns, assignments }),

  /**
   * Apply custom port rules to files
   */
  applyCustomRules: (
    sourcePath: string,
    targetPath: string,
    rules: CustomPortRule[],
    portOffset: number
  ): Promise<CustomRuleReplacement[]> =>
    invoke('apply_port_custom_rules', { sourcePath, targetPath, rules, portOffset }),

  /**
   * Get all unique env ports (deduplicated by variable name)
   */
  getUniqueEnvPorts: (detected: DetectedPorts): PortSource[] => {
    const seen = new Map<string, PortSource>();
    for (const port of detected.env_ports) {
      if (!seen.has(port.variable_name)) {
        seen.set(port.variable_name, port);
      }
    }
    return Array.from(seen.values());
  },

  /**
   * Check if a directory has any detectable ports
   */
  hasDetectablePorts: (detected: DetectedPorts): boolean => {
    return (
      detected.env_ports.length > 0 ||
      detected.dockerfile_ports.length > 0 ||
      detected.compose_ports.length > 0
    );
  },

  /**
   * Create default port config
   */
  createDefaultConfig: (): PortConfig => ({
    enabled: true,
    port_range_start: DEFAULT_PORT_RANGE_START,
    port_range_end: DEFAULT_PORT_RANGE_END,
    next_port: DEFAULT_PORT_RANGE_START,
    worktree_assignments: {},
    custom_rules: [],
  }),

  /**
   * Generate port assignments for a worktree
   */
  generateAssignments: async (
    dirPath: string,
    config: PortConfig
  ): Promise<{ assignments: PortAssignment[]; updatedConfig: PortConfig } | null> => {
    const detected = await portIsolationService.detectPorts(dirPath);

    if (!portIsolationService.hasDetectablePorts(detected)) {
      return null;
    }

    const uniquePorts = portIsolationService.getUniqueEnvPorts(detected);
    const result = await portIsolationService.allocatePorts(uniquePorts, config.next_port);

    const updatedConfig: PortConfig = {
      ...config,
      next_port: result.next_port,
    };

    return {
      assignments: result.assignments,
      updatedConfig,
    };
  },
};
