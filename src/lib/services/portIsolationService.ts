import { invoke } from '@tauri-apps/api/core';
import type { CopyResult } from './worktreeService';
import type {
  PortConfig as PersistencePortConfig,
  PortAssignment as PersistencePortAssignment,
  WorktreePortAssignment as PersistenceWorktreePortAssignment,
} from './persistenceService';

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

// Re-export persistence types for convenience
export type { PersistencePortConfig as PortConfig };
export type { PersistencePortAssignment };
export type { PersistenceWorktreePortAssignment as WorktreePortAssignment };

// Default port range (100 ports per project)
export const DEFAULT_PORT_RANGE_START = 20000;
export const DEFAULT_PORT_BLOCK_SIZE = 100;
export const DEFAULT_PORT_RANGE_END = DEFAULT_PORT_RANGE_START + DEFAULT_PORT_BLOCK_SIZE - 1;

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
   * Allocate unique ports for the given port sources (Tauri command wrapper)
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
  createDefaultConfig: (): PersistencePortConfig => ({
    enabled: true,
    portRangeStart: DEFAULT_PORT_RANGE_START,
    portRangeEnd: DEFAULT_PORT_RANGE_END,
    worktreeAssignments: {},
    customRules: [],
  }),

  /**
   * Get all ports currently in use by existing worktrees
   */
  getUsedPorts: (config: PersistencePortConfig): Set<number> => {
    const usedPorts = new Set<number>();
    // Handle case where worktreeAssignments is undefined (old config format)
    if (!config.worktreeAssignments) {
      return usedPorts;
    }
    for (const assignment of Object.values(config.worktreeAssignments)) {
      for (const port of assignment.assignments) {
        usedPorts.add(port.assignedValue);
      }
    }
    return usedPorts;
  },

  /**
   * Allocate ports avoiding those already used by other worktrees
   */
  allocatePortsAvoidingUsed: (
    ports: PortSource[],
    config: PersistencePortConfig
  ): PortAssignment[] => {
    const usedPorts = portIsolationService.getUsedPorts(config);
    const assignments: PortAssignment[] = [];

    let nextAvailable = config.portRangeStart;

    for (const port of ports) {
      // Find next available port that's not in use
      while (usedPorts.has(nextAvailable) && nextAvailable <= config.portRangeEnd) {
        nextAvailable++;
      }

      if (nextAvailable > config.portRangeEnd) {
        console.error('Port range exhausted');
        break;
      }

      assignments.push({
        variable_name: port.variable_name,
        original_value: port.port_value,
        assigned_value: nextAvailable,
      });

      // Mark this port as used and move to next
      usedPorts.add(nextAvailable);
      nextAvailable++;
    }

    return assignments;
  },

  /**
   * Register port assignments for a worktree
   */
  registerWorktreeAssignments: (
    config: PersistencePortConfig,
    worktreeName: string,
    assignments: PortAssignment[]
  ): PersistencePortConfig => {
    return {
      ...config,
      worktreeAssignments: {
        ...(config.worktreeAssignments ?? {}),
        [worktreeName]: {
          worktreeName,
          assignments: assignments.map((a) => ({
            variableName: a.variable_name,
            originalValue: a.original_value,
            assignedValue: a.assigned_value,
          })),
        },
      },
    };
  },

  /**
   * Remove port assignments for a worktree (called when worktree is deleted)
   */
  removeWorktreeAssignments: (
    config: PersistencePortConfig,
    worktreeName: string
  ): PersistencePortConfig => {
    // Handle case where worktreeAssignments is undefined
    if (!config.worktreeAssignments) {
      return config;
    }
    // eslint-disable-next-line @typescript-eslint/no-unused-vars -- intentionally discarding removed entry
    const { [worktreeName]: _removed, ...remainingAssignments } = config.worktreeAssignments;
    return {
      ...config,
      worktreeAssignments: remainingAssignments,
    };
  },
};
