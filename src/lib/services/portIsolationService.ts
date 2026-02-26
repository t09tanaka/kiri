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
  script_ports: PortSource[];
}

export interface PortAssignment {
  variable_name: string;
  original_value: number;
  assigned_value: number;
}

export interface PortAllocationResult {
  assignments: PortAssignment[];
  worktree_index: number;
  overflow_warnings: string[];
}

// Re-export persistence types for convenience
export type { PersistencePortConfig as PortConfig };
export type { PersistencePortAssignment };
export type { PersistenceWorktreePortAssignment as WorktreePortAssignment };

export const PORT_OFFSET_STEP = 100;

// Default target files for port isolation
export const DEFAULT_TARGET_FILES = [
  '**/.env*',
  '**/docker-compose.yml',
  '**/docker-compose.yaml',
  '**/docker-compose.*.yml',
  '**/docker-compose.*.yaml',
  '**/compose.yml',
  '**/compose.yaml',
  '**/package.json',
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
  allocatePorts: (ports: PortSource[], worktreeIndex: number): Promise<PortAllocationResult> =>
    invoke('allocate_worktree_ports', { ports, worktreeIndex }),

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
   * Get all unique script ports (deduplicated by variable name)
   */
  getUniqueScriptPorts: (detected: DetectedPorts): PortSource[] => {
    const seen = new Map<string, PortSource>();
    for (const port of detected.script_ports) {
      if (!seen.has(port.variable_name)) {
        seen.set(port.variable_name, port);
      }
    }
    return Array.from(seen.values());
  },

  /**
   * Get all unique ports from env, compose, and script sources
   */
  getAllUniquePorts: (detected: DetectedPorts): PortSource[] => {
    return [
      ...portIsolationService.getUniqueEnvPorts(detected),
      ...portIsolationService.getUniqueComposePorts(detected),
      ...portIsolationService.getUniqueScriptPorts(detected),
    ];
  },

  /**
   * Check if a variable name represents a compose port (has "COMPOSE:" prefix)
   */
  isComposePort: (variableName: string): boolean => {
    return variableName.startsWith('COMPOSE:');
  },

  /**
   * Check if a variable name represents a script port (has "SCRIPT:" prefix)
   */
  isScriptPort: (variableName: string): boolean => {
    return variableName.startsWith('SCRIPT:');
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
      detected.compose_ports.length > 0 ||
      detected.script_ports.length > 0
    );
  },

  /**
   * Create default port config
   */
  createDefaultConfig: (): PersistencePortConfig => ({
    enabled: true,
    worktreeAssignments: {},
    targetFiles: [...DEFAULT_TARGET_FILES],
  }),

  /**
   * Get all worktree indices currently in use by existing worktrees.
   * The index is derived from: (assignedValue - originalValue) / 100
   */
  getUsedWorktreeIndices: (config: PersistencePortConfig): Set<number> => {
    const indices = new Set<number>();
    if (!config.worktreeAssignments) {
      return indices;
    }
    for (const assignment of Object.values(config.worktreeAssignments)) {
      if (assignment.assignments.length > 0) {
        const first = assignment.assignments[0];
        const index = (first.assignedValue - first.originalValue) / PORT_OFFSET_STEP;
        if (Number.isInteger(index) && index > 0) {
          indices.add(index);
        }
      }
    }
    return indices;
  },

  /**
   * Get the next available worktree index (smallest positive integer not in use)
   */
  getNextWorktreeIndex: (config: PersistencePortConfig): number => {
    const used = portIsolationService.getUsedWorktreeIndices(config);
    let index = 1;
    while (used.has(index)) {
      index++;
    }
    return index;
  },

  /**
   * Allocate ports using offset strategy: original_port + (worktree_index * 100).
   * The next available worktree index is automatically determined.
   * Ports with the same original port_value get the same assigned value.
   * Returns assignments, the worktree index used, and any overflow warnings.
   */
  allocatePortsWithOffset: (
    ports: PortSource[],
    config: PersistencePortConfig
  ): { assignments: PortAssignment[]; worktreeIndex: number; overflowWarnings: string[] } => {
    const worktreeIndex = portIsolationService.getNextWorktreeIndex(config);
    const offset = worktreeIndex * PORT_OFFSET_STEP;
    const assignments: PortAssignment[] = [];
    const overflowWarnings: string[] = [];
    const assignedByOriginalValue = new Map<number, number>();

    for (const port of ports) {
      const existing = assignedByOriginalValue.get(port.port_value);
      if (existing !== undefined) {
        assignments.push({
          variable_name: port.variable_name,
          original_value: port.port_value,
          assigned_value: existing,
        });
        continue;
      }

      const assigned = port.port_value + offset;

      if (assigned > 65535) {
        overflowWarnings.push(
          `${port.variable_name}=${port.port_value}: ${assigned} exceeds max port 65535`
        );
        continue;
      }

      assignments.push({
        variable_name: port.variable_name,
        original_value: port.port_value,
        assigned_value: assigned,
      });
      assignedByOriginalValue.set(port.port_value, assigned);
    }

    return { assignments, worktreeIndex, overflowWarnings };
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
