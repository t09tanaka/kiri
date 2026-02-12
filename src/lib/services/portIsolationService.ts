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

// Re-export persistence types for convenience
export type { PersistencePortConfig as PortConfig };
export type { PersistencePortAssignment };
export type { PersistenceWorktreePortAssignment as WorktreePortAssignment };

// Default port range (100 ports per project)
export const DEFAULT_PORT_RANGE_START = 20000;
export const DEFAULT_PORT_BLOCK_SIZE = 100;
export const DEFAULT_PORT_RANGE_END = DEFAULT_PORT_RANGE_START + DEFAULT_PORT_BLOCK_SIZE - 1;

// Default target files for port isolation
export const DEFAULT_TARGET_FILES = [
  '**/.env*',
  '**/docker-compose.yml',
  '**/docker-compose.yaml',
  '**/docker-compose.*.yml',
  '**/docker-compose.*.yaml',
  '**/compose.yml',
  '**/compose.yaml',
];

/**
 * Convert a target file glob pattern to a RegExp.
 * Supports:
 *   ** / -> matches any path prefix (including empty)
 *   *    -> matches any characters except /
 *
 * Examples:
 *   ** /.env*                  -> /^(?:.*\/)?.env[^/]*$/
 *   ** /docker-compose.*.yml  -> /^(?:.*\/)?docker-compose\.[^/]*\.yml$/
 */
export function targetFilePatternToRegex(pattern: string): RegExp {
  const parts = pattern.split('**/');
  let regexStr = '^';

  for (let i = 0; i < parts.length; i++) {
    if (i > 0) {
      // **/ was here -- match any path prefix (including empty)
      regexStr += '(?:.*/)?';
    }
    // Escape regex special chars except *, then replace * with [^/]*
    const segment = parts[i].replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '[^/]*');
    regexStr += segment;
  }

  regexStr += '$';
  return new RegExp(regexStr);
}

/**
 * Check if a relative file path matches a target file pattern.
 */
export function matchesTargetFilePattern(relativePath: string, pattern: string): boolean {
  return targetFilePatternToRegex(pattern).test(relativePath);
}

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
   * Files will have their port values replaced according to assignments
   */
  copyFilesWithPorts: (
    sourcePath: string,
    targetPath: string,
    patterns: string[],
    assignments: PortAssignment[]
  ): Promise<CopyResult> =>
    invoke('copy_files_with_ports', { sourcePath, targetPath, patterns, assignments }),

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
   * Get all unique compose ports (deduplicated by variable name)
   */
  getUniqueComposePorts: (detected: DetectedPorts): PortSource[] => {
    const seen = new Map<string, PortSource>();
    for (const port of detected.compose_ports) {
      if (!seen.has(port.variable_name)) {
        seen.set(port.variable_name, port);
      }
    }
    return Array.from(seen.values());
  },

  /**
   * Get all unique ports from both env and compose sources
   */
  getAllUniquePorts: (detected: DetectedPorts): PortSource[] => {
    return [
      ...portIsolationService.getUniqueEnvPorts(detected),
      ...portIsolationService.getUniqueComposePorts(detected),
    ];
  },

  /**
   * Check if a variable name represents a compose port (has "COMPOSE:" prefix)
   */
  isComposePort: (variableName: string): boolean => {
    return variableName.startsWith('COMPOSE:');
  },

  /**
   * Determine if a port variable is "transformable" given the current target file configuration.
   * A port is transformable if at least one of its source files matches an ENABLED target pattern.
   */
  isPortTransformable: (
    variableName: string,
    allPortSources: PortSource[],
    projectPath: string,
    enabledPatterns: string[]
  ): boolean => {
    if (enabledPatterns.length === 0) return false;

    const prefix = projectPath.endsWith('/') ? projectPath : `${projectPath}/`;
    const sources = allPortSources.filter((p) => p.variable_name === variableName);

    if (sources.length === 0) return false;

    return sources.some((source) => {
      const rel = source.file_path.startsWith(prefix)
        ? source.file_path.slice(prefix.length)
        : source.file_path;
      return enabledPatterns.some((pattern) => matchesTargetFilePattern(rel, pattern));
    });
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
    targetFiles: [...DEFAULT_TARGET_FILES],
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
   * Allocate ports avoiding those already used by other worktrees.
   * Ports with the same original port_value (e.g., .env PORT=3000 and compose COMPOSE:3000)
   * will be assigned the same new port value.
   */
  allocatePortsAvoidingUsed: (
    ports: PortSource[],
    config: PersistencePortConfig
  ): PortAssignment[] => {
    const usedPorts = portIsolationService.getUsedPorts(config);
    const assignments: PortAssignment[] = [];
    const assignedByOriginalValue = new Map<number, number>();

    let nextAvailable = config.portRangeStart;

    for (const port of ports) {
      // If the same original port value was already assigned, reuse it
      const existingAssignment = assignedByOriginalValue.get(port.port_value);
      if (existingAssignment !== undefined) {
        assignments.push({
          variable_name: port.variable_name,
          original_value: port.port_value,
          assigned_value: existingAssignment,
        });
        continue;
      }

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

      // Track assignment by original value for reuse
      assignedByOriginalValue.set(port.port_value, nextAvailable);

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
