import { describe, it, expect, vi } from 'vitest';
import {
  portIsolationService,
  targetFilePatternToRegex,
  matchesTargetFilePattern,
  DEFAULT_TARGET_FILES,
} from './portIsolationService';
import type { DetectedPorts, PortSource } from './portIsolationService';
import type { PortConfig } from './persistenceService';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

function makePortSource(overrides: Partial<PortSource> = {}): PortSource {
  return {
    file_path: '.env',
    variable_name: 'PORT',
    port_value: 3000,
    line_number: 1,
    ...overrides,
  };
}

function makeEmptyDetected(): DetectedPorts {
  return { env_ports: [], dockerfile_ports: [], compose_ports: [], script_ports: [] };
}

function makeDefaultConfig(overrides: Partial<PortConfig> = {}): PortConfig {
  return {
    enabled: true,
    worktreeAssignments: {},
    targetFiles: ['**/.env*'],
    ...overrides,
  };
}

describe('portIsolationService', () => {
  describe('getUniqueComposePorts', () => {
    it('should return empty array when no compose ports', () => {
      const detected = makeEmptyDetected();
      expect(portIsolationService.getUniqueComposePorts(detected)).toEqual([]);
    });

    it('should return all compose ports when no duplicates', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        compose_ports: [
          makePortSource({
            variable_name: 'COMPOSE:3000',
            port_value: 3000,
            file_path: 'docker-compose.yml',
          }),
          makePortSource({
            variable_name: 'COMPOSE:5432',
            port_value: 5432,
            file_path: 'docker-compose.yml',
          }),
        ],
      };
      const result = portIsolationService.getUniqueComposePorts(detected);
      expect(result).toHaveLength(2);
      expect(result[0].variable_name).toBe('COMPOSE:3000');
      expect(result[1].variable_name).toBe('COMPOSE:5432');
    });

    it('should deduplicate compose ports by variable_name', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        compose_ports: [
          makePortSource({
            variable_name: 'COMPOSE:3000',
            port_value: 3000,
            file_path: 'docker-compose.yml',
            line_number: 5,
          }),
          makePortSource({
            variable_name: 'COMPOSE:3000',
            port_value: 3000,
            file_path: 'docker-compose.dev.yml',
            line_number: 10,
          }),
          makePortSource({
            variable_name: 'COMPOSE:5432',
            port_value: 5432,
            file_path: 'docker-compose.yml',
            line_number: 8,
          }),
        ],
      };
      const result = portIsolationService.getUniqueComposePorts(detected);
      expect(result).toHaveLength(2);
      expect(result[0].variable_name).toBe('COMPOSE:3000');
      expect(result[0].file_path).toBe('docker-compose.yml');
      expect(result[0].line_number).toBe(5);
      expect(result[1].variable_name).toBe('COMPOSE:5432');
    });
  });

  describe('getAllUniquePorts', () => {
    it('should return empty array when no ports detected', () => {
      const detected = makeEmptyDetected();
      expect(portIsolationService.getAllUniquePorts(detected)).toEqual([]);
    });

    it('should combine env and compose ports', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        env_ports: [
          makePortSource({ variable_name: 'PORT', port_value: 3000 }),
          makePortSource({ variable_name: 'DB_PORT', port_value: 5432 }),
        ],
        compose_ports: [
          makePortSource({
            variable_name: 'COMPOSE:3000',
            port_value: 3000,
            file_path: 'docker-compose.yml',
          }),
          makePortSource({
            variable_name: 'COMPOSE:6379',
            port_value: 6379,
            file_path: 'docker-compose.yml',
          }),
        ],
      };
      const result = portIsolationService.getAllUniquePorts(detected);
      expect(result).toHaveLength(4);
      expect(result[0].variable_name).toBe('PORT');
      expect(result[1].variable_name).toBe('DB_PORT');
      expect(result[2].variable_name).toBe('COMPOSE:3000');
      expect(result[3].variable_name).toBe('COMPOSE:6379');
    });

    it('should deduplicate within each source', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        env_ports: [
          makePortSource({ variable_name: 'PORT', port_value: 3000, file_path: '.env' }),
          makePortSource({ variable_name: 'PORT', port_value: 3000, file_path: '.env.local' }),
        ],
        compose_ports: [
          makePortSource({
            variable_name: 'COMPOSE:3000',
            port_value: 3000,
            file_path: 'docker-compose.yml',
          }),
          makePortSource({
            variable_name: 'COMPOSE:3000',
            port_value: 3000,
            file_path: 'docker-compose.dev.yml',
          }),
        ],
      };
      const result = portIsolationService.getAllUniquePorts(detected);
      expect(result).toHaveLength(2);
      expect(result[0].variable_name).toBe('PORT');
      expect(result[1].variable_name).toBe('COMPOSE:3000');
    });

    it('should not include dockerfile ports', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        env_ports: [makePortSource({ variable_name: 'PORT', port_value: 3000 })],
        dockerfile_ports: [
          makePortSource({ variable_name: 'EXPOSE', port_value: 3000, file_path: 'Dockerfile' }),
        ],
      };
      const result = portIsolationService.getAllUniquePorts(detected);
      expect(result).toHaveLength(1);
      expect(result[0].variable_name).toBe('PORT');
    });
  });

  describe('isComposePort', () => {
    it('should return true for compose port variable names', () => {
      expect(portIsolationService.isComposePort('COMPOSE:3000')).toBe(true);
      expect(portIsolationService.isComposePort('COMPOSE:5432')).toBe(true);
      expect(portIsolationService.isComposePort('COMPOSE:80')).toBe(true);
    });

    it('should return false for non-compose variable names', () => {
      expect(portIsolationService.isComposePort('PORT')).toBe(false);
      expect(portIsolationService.isComposePort('DB_PORT')).toBe(false);
      expect(portIsolationService.isComposePort('REDIS_PORT')).toBe(false);
      expect(portIsolationService.isComposePort('')).toBe(false);
    });

    it('should be case-sensitive', () => {
      expect(portIsolationService.isComposePort('compose:3000')).toBe(false);
      expect(portIsolationService.isComposePort('Compose:3000')).toBe(false);
    });
  });

  describe('isScriptPort', () => {
    it('should return true for script port variable names', () => {
      expect(portIsolationService.isScriptPort('SCRIPT:3000')).toBe(true);
      expect(portIsolationService.isScriptPort('SCRIPT:8080')).toBe(true);
    });

    it('should return false for non-script variable names', () => {
      expect(portIsolationService.isScriptPort('PORT')).toBe(false);
      expect(portIsolationService.isScriptPort('COMPOSE:3000')).toBe(false);
      expect(portIsolationService.isScriptPort('')).toBe(false);
    });

    it('should be case-sensitive', () => {
      expect(portIsolationService.isScriptPort('script:3000')).toBe(false);
    });
  });

  describe('getUniqueScriptPorts', () => {
    it('should return empty array when no script ports', () => {
      const detected = makeEmptyDetected();
      expect(portIsolationService.getUniqueScriptPorts(detected)).toEqual([]);
    });

    it('should return all script ports when no duplicates', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        script_ports: [
          makePortSource({
            variable_name: 'SCRIPT:3000',
            port_value: 3000,
            file_path: 'package.json',
          }),
          makePortSource({
            variable_name: 'SCRIPT:8080',
            port_value: 8080,
            file_path: 'package.json',
          }),
        ],
      };
      const result = portIsolationService.getUniqueScriptPorts(detected);
      expect(result).toHaveLength(2);
    });

    it('should deduplicate script ports by variable_name', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        script_ports: [
          makePortSource({
            variable_name: 'SCRIPT:3000',
            port_value: 3000,
            file_path: 'package.json',
            line_number: 5,
          }),
          makePortSource({
            variable_name: 'SCRIPT:3000',
            port_value: 3000,
            file_path: 'package.json',
            line_number: 10,
          }),
        ],
      };
      const result = portIsolationService.getUniqueScriptPorts(detected);
      expect(result).toHaveLength(1);
      expect(result[0].line_number).toBe(5); // Keeps first occurrence
    });
  });

  describe('createDefaultConfig', () => {
    it('should return default port config', () => {
      const config = portIsolationService.createDefaultConfig();
      expect(config.enabled).toBe(true);
      expect(config.worktreeAssignments).toEqual({});
      expect(config.targetFiles).toContain('**/.env*');
      expect(config.targetFiles).toContain('**/docker-compose.yml');
      expect(config.targetFiles).toContain('**/package.json');
      expect(config.targetFiles.length).toBeGreaterThan(0);
    });

    it('should return a new object each time', () => {
      const config1 = portIsolationService.createDefaultConfig();
      const config2 = portIsolationService.createDefaultConfig();
      expect(config1).toEqual(config2);
      expect(config1).not.toBe(config2);
      expect(config1.targetFiles).not.toBe(config2.targetFiles);
    });
  });

  describe('hasDetectablePorts', () => {
    it('should return false when no ports detected', () => {
      expect(portIsolationService.hasDetectablePorts(makeEmptyDetected())).toBe(false);
    });

    it('should return true when only compose_ports exist', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        compose_ports: [
          makePortSource({
            variable_name: 'COMPOSE:3000',
            port_value: 3000,
            file_path: 'docker-compose.yml',
          }),
        ],
      };
      expect(portIsolationService.hasDetectablePorts(detected)).toBe(true);
    });

    it('should return true when only env_ports exist', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        env_ports: [makePortSource()],
      };
      expect(portIsolationService.hasDetectablePorts(detected)).toBe(true);
    });

    it('should return true when only dockerfile_ports exist', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        dockerfile_ports: [makePortSource({ file_path: 'Dockerfile', variable_name: 'EXPOSE' })],
      };
      expect(portIsolationService.hasDetectablePorts(detected)).toBe(true);
    });
  });

  describe('getUsedWorktreeIndices', () => {
    it('should return empty set when no worktree assignments', () => {
      const config = makeDefaultConfig();
      const result = portIsolationService.getUsedWorktreeIndices(config);
      expect(result.size).toBe(0);
    });

    it('should return empty set when worktreeAssignments is undefined', () => {
      const config = makeDefaultConfig({ worktreeAssignments: undefined as never });
      const result = portIsolationService.getUsedWorktreeIndices(config);
      expect(result.size).toBe(0);
    });

    it('should derive index from assignments (assignedValue - originalValue) / 100', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
        },
      });
      const result = portIsolationService.getUsedWorktreeIndices(config);
      expect(result.size).toBe(1);
      expect(result.has(1)).toBe(true);
    });

    it('should handle multiple worktrees with different indices', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
          'feature-b': {
            worktreeName: 'feature-b',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3300 }],
          },
        },
      });
      const result = portIsolationService.getUsedWorktreeIndices(config);
      expect(result.size).toBe(2);
      expect(result.has(1)).toBe(true);
      expect(result.has(3)).toBe(true);
    });

    it('should skip worktrees with empty assignments', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [],
          },
        },
      });
      const result = portIsolationService.getUsedWorktreeIndices(config);
      expect(result.size).toBe(0);
    });

    it('should skip worktrees with zero or negative index', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            // assignedValue === originalValue → index = 0, skipped
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3000 }],
          },
          'feature-b': {
            worktreeName: 'feature-b',
            // assignedValue < originalValue → negative index, skipped
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 2900 }],
          },
        },
      });
      const result = portIsolationService.getUsedWorktreeIndices(config);
      expect(result.size).toBe(0);
    });

    it('should skip worktrees with non-integer index', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            // (3050 - 3000) / 100 = 0.5 → not integer, skipped
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3050 }],
          },
        },
      });
      const result = portIsolationService.getUsedWorktreeIndices(config);
      expect(result.size).toBe(0);
    });
  });

  describe('getNextWorktreeIndex', () => {
    it('should return 1 when no worktrees exist', () => {
      const config = makeDefaultConfig();
      expect(portIsolationService.getNextWorktreeIndex(config)).toBe(1);
    });

    it('should return 2 when index 1 is used', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
        },
      });
      expect(portIsolationService.getNextWorktreeIndex(config)).toBe(2);
    });

    it('should reuse gaps (return 2 when 1 and 3 are used)', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
          'feature-c': {
            worktreeName: 'feature-c',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3300 }],
          },
        },
      });
      expect(portIsolationService.getNextWorktreeIndex(config)).toBe(2);
    });
  });

  describe('allocatePortsWithOffset', () => {
    it('should allocate ports with offset for first worktree (index 1)', () => {
      const config = makeDefaultConfig();
      const ports: PortSource[] = [
        makePortSource({ variable_name: 'PORT', port_value: 3000 }),
        makePortSource({ variable_name: 'DB_PORT', port_value: 5432 }),
      ];

      const result = portIsolationService.allocatePortsWithOffset(ports, config);

      expect(result.worktreeIndex).toBe(1);
      expect(result.assignments).toHaveLength(2);
      expect(result.overflowWarnings).toHaveLength(0);
      expect(result.assignments[0]).toEqual({
        variable_name: 'PORT',
        original_value: 3000,
        assigned_value: 3100,
      });
      expect(result.assignments[1]).toEqual({
        variable_name: 'DB_PORT',
        original_value: 5432,
        assigned_value: 5532,
      });
    });

    it('should assign same port for same original port_value (env + compose)', () => {
      const config = makeDefaultConfig();
      const ports: PortSource[] = [
        makePortSource({ variable_name: 'PORT', port_value: 3000 }),
        makePortSource({ variable_name: 'DB_PORT', port_value: 5432 }),
        makePortSource({
          variable_name: 'COMPOSE:3000',
          port_value: 3000,
          file_path: 'docker-compose.yml',
        }),
        makePortSource({
          variable_name: 'COMPOSE:5432',
          port_value: 5432,
          file_path: 'docker-compose.yml',
        }),
      ];

      const result = portIsolationService.allocatePortsWithOffset(ports, config);

      expect(result.assignments).toHaveLength(4);
      // PORT=3000 and COMPOSE:3000 should get the same assigned_value
      expect(result.assignments[0].assigned_value).toBe(3100);
      expect(result.assignments[2].assigned_value).toBe(3100);
      // DB_PORT=5432 and COMPOSE:5432 should get the same assigned_value
      expect(result.assignments[1].assigned_value).toBe(5532);
      expect(result.assignments[3].assigned_value).toBe(5532);
    });

    it('should work with only compose ports', () => {
      const config = makeDefaultConfig();
      const ports: PortSource[] = [
        makePortSource({
          variable_name: 'COMPOSE:3000',
          port_value: 3000,
          file_path: 'docker-compose.yml',
        }),
        makePortSource({
          variable_name: 'COMPOSE:5432',
          port_value: 5432,
          file_path: 'docker-compose.yml',
        }),
      ];

      const result = portIsolationService.allocatePortsWithOffset(ports, config);

      expect(result.assignments).toHaveLength(2);
      expect(result.assignments[0]).toEqual({
        variable_name: 'COMPOSE:3000',
        original_value: 3000,
        assigned_value: 3100,
      });
      expect(result.assignments[1]).toEqual({
        variable_name: 'COMPOSE:5432',
        original_value: 5432,
        assigned_value: 5532,
      });
    });

    it('should use next available index when worktrees exist', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [
              { variableName: 'PORT', originalValue: 3000, assignedValue: 3100 },
              { variableName: 'DB_PORT', originalValue: 5432, assignedValue: 5532 },
            ],
          },
        },
      });
      const ports: PortSource[] = [
        makePortSource({ variable_name: 'PORT', port_value: 3000 }),
        makePortSource({
          variable_name: 'COMPOSE:3000',
          port_value: 3000,
          file_path: 'docker-compose.yml',
        }),
      ];

      const result = portIsolationService.allocatePortsWithOffset(ports, config);

      expect(result.worktreeIndex).toBe(2);
      expect(result.assignments).toHaveLength(2);
      // index 2 means offset = 200
      expect(result.assignments[0].assigned_value).toBe(3200);
      expect(result.assignments[1].assigned_value).toBe(3200);
    });

    it('should reuse gap indices when worktrees are deleted', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          // index 1 was deleted (gap)
          'feature-b': {
            worktreeName: 'feature-b',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3200 }],
          },
          'feature-c': {
            worktreeName: 'feature-c',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3300 }],
          },
        },
      });
      const ports: PortSource[] = [makePortSource({ variable_name: 'PORT', port_value: 3000 })];

      const result = portIsolationService.allocatePortsWithOffset(ports, config);

      // Should reuse index 1 (the gap)
      expect(result.worktreeIndex).toBe(1);
      expect(result.assignments[0].assigned_value).toBe(3100);
    });

    it('should produce overflow warnings for ports exceeding 65535', () => {
      const config = makeDefaultConfig();
      const ports: PortSource[] = [
        makePortSource({ variable_name: 'PORT', port_value: 3000 }),
        makePortSource({ variable_name: 'HIGH_PORT', port_value: 65500 }),
      ];

      const result = portIsolationService.allocatePortsWithOffset(ports, config);

      // PORT=3000 should be assigned (3100)
      expect(result.assignments).toHaveLength(1);
      expect(result.assignments[0].assigned_value).toBe(3100);
      // HIGH_PORT=65500 should produce an overflow warning
      expect(result.overflowWarnings).toHaveLength(1);
      expect(result.overflowWarnings[0]).toContain('65500');
      expect(result.overflowWarnings[0]).toContain('65535');
    });

    it('should handle mixed env and compose ports where compose appears first', () => {
      const config = makeDefaultConfig();
      const ports: PortSource[] = [
        makePortSource({
          variable_name: 'COMPOSE:3000',
          port_value: 3000,
          file_path: 'docker-compose.yml',
        }),
        makePortSource({ variable_name: 'PORT', port_value: 3000 }),
      ];

      const result = portIsolationService.allocatePortsWithOffset(ports, config);

      expect(result.assignments).toHaveLength(2);
      // Both should get the same assigned_value regardless of order
      expect(result.assignments[0].assigned_value).toBe(3100);
      expect(result.assignments[1].assigned_value).toBe(3100);
    });

    it('should use higher index for 3rd worktree', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
          'feature-b': {
            worktreeName: 'feature-b',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3200 }],
          },
        },
      });
      const ports: PortSource[] = [
        makePortSource({ variable_name: 'PORT', port_value: 3000 }),
        makePortSource({ variable_name: 'DB_PORT', port_value: 5473 }),
      ];

      const result = portIsolationService.allocatePortsWithOffset(ports, config);

      expect(result.worktreeIndex).toBe(3);
      expect(result.assignments[0].assigned_value).toBe(3300);
      expect(result.assignments[1].assigned_value).toBe(5773);
    });
  });

  describe('targetFilePatternToRegex', () => {
    it('should match .env files with **/.env* pattern', () => {
      const regex = targetFilePatternToRegex('**/.env*');
      expect(regex.test('.env')).toBe(true);
      expect(regex.test('.env.local')).toBe(true);
      expect(regex.test('.env.production')).toBe(true);
      expect(regex.test('packages/api/.env')).toBe(true);
      expect(regex.test('a/b/.env.production')).toBe(true);
    });

    it('should not match non-.env files with **/.env* pattern', () => {
      const regex = targetFilePatternToRegex('**/.env*');
      expect(regex.test('env')).toBe(false);
      expect(regex.test('something.env')).toBe(false);
      expect(regex.test('config.json')).toBe(false);
    });

    it('should match exact docker-compose.yml with **/docker-compose.yml', () => {
      const regex = targetFilePatternToRegex('**/docker-compose.yml');
      expect(regex.test('docker-compose.yml')).toBe(true);
      expect(regex.test('services/docker-compose.yml')).toBe(true);
      expect(regex.test('a/b/docker-compose.yml')).toBe(true);
    });

    it('should not match similar files with **/docker-compose.yml', () => {
      const regex = targetFilePatternToRegex('**/docker-compose.yml');
      expect(regex.test('docker-compose.yaml')).toBe(false);
      expect(regex.test('docker-compose.dev.yml')).toBe(false);
    });

    it('should match docker-compose.*.yml pattern', () => {
      const regex = targetFilePatternToRegex('**/docker-compose.*.yml');
      expect(regex.test('docker-compose.dev.yml')).toBe(true);
      expect(regex.test('docker-compose.test.yml')).toBe(true);
      expect(regex.test('services/docker-compose.prod.yml')).toBe(true);
    });

    it('should match compose.yml pattern', () => {
      const regex = targetFilePatternToRegex('**/compose.yml');
      expect(regex.test('compose.yml')).toBe(true);
      expect(regex.test('infra/compose.yml')).toBe(true);
    });

    it('should handle pattern without **/ prefix', () => {
      const regex = targetFilePatternToRegex('.env*');
      expect(regex.test('.env')).toBe(true);
      expect(regex.test('.env.local')).toBe(true);
      // Without **/, should NOT match subdirectory paths
      expect(regex.test('subdir/.env')).toBe(false);
    });
  });

  describe('matchesTargetFilePattern', () => {
    it('should return true for matching path and pattern', () => {
      expect(matchesTargetFilePattern('.env', '**/.env*')).toBe(true);
      expect(matchesTargetFilePattern('backend/.env.local', '**/.env*')).toBe(true);
      expect(matchesTargetFilePattern('docker-compose.yml', '**/docker-compose.yml')).toBe(true);
      expect(
        matchesTargetFilePattern('backend/docker-compose.test.yml', '**/docker-compose.*.yml')
      ).toBe(true);
    });

    it('should return false for non-matching path and pattern', () => {
      expect(matchesTargetFilePattern('config.json', '**/.env*')).toBe(false);
      expect(matchesTargetFilePattern('docker-compose.dev.yml', '**/docker-compose.yml')).toBe(
        false
      );
    });
  });

  describe('isPortTransformable', () => {
    const projectPath = '/project';

    it('should return true when source file matches an enabled pattern', () => {
      const sources: PortSource[] = [
        makePortSource({ variable_name: 'PORT', file_path: '/project/.env' }),
      ];
      expect(
        portIsolationService.isPortTransformable('PORT', sources, projectPath, ['**/.env*'])
      ).toBe(true);
    });

    it('should return false when no enabled patterns', () => {
      const sources: PortSource[] = [
        makePortSource({ variable_name: 'PORT', file_path: '/project/.env' }),
      ];
      expect(portIsolationService.isPortTransformable('PORT', sources, projectPath, [])).toBe(
        false
      );
    });

    it('should return false when source file does not match any enabled pattern', () => {
      const sources: PortSource[] = [
        makePortSource({
          variable_name: 'COMPOSE:5432',
          file_path: '/project/docker-compose.test.yml',
        }),
      ];
      expect(
        portIsolationService.isPortTransformable('COMPOSE:5432', sources, projectPath, ['**/.env*'])
      ).toBe(false);
    });

    it('should return true when at least one source file matches (mixed sources)', () => {
      const sources: PortSource[] = [
        makePortSource({ variable_name: 'PORT', file_path: '/project/.env' }),
        makePortSource({
          variable_name: 'PORT',
          file_path: '/project/docker-compose.yml',
        }),
      ];
      // Only .env* pattern enabled, docker-compose.yml disabled
      expect(
        portIsolationService.isPortTransformable('PORT', sources, projectPath, ['**/.env*'])
      ).toBe(true);
    });

    it('should return false when all source files match only disabled patterns', () => {
      const sources: PortSource[] = [
        makePortSource({
          variable_name: 'COMPOSE:3000',
          file_path: '/project/docker-compose.yml',
        }),
        makePortSource({
          variable_name: 'COMPOSE:3000',
          file_path: '/project/docker-compose.dev.yml',
        }),
      ];
      // Only .env* enabled, both compose patterns disabled
      expect(
        portIsolationService.isPortTransformable('COMPOSE:3000', sources, projectPath, ['**/.env*'])
      ).toBe(false);
    });

    it('should handle subdirectory source files', () => {
      const sources: PortSource[] = [
        makePortSource({
          variable_name: 'PORT',
          file_path: '/project/backend/.env',
        }),
      ];
      expect(
        portIsolationService.isPortTransformable('PORT', sources, projectPath, ['**/.env*'])
      ).toBe(true);
    });

    it('should return false when variable has no sources', () => {
      expect(
        portIsolationService.isPortTransformable('UNKNOWN', [], projectPath, ['**/.env*'])
      ).toBe(false);
    });

    it('should handle relative file paths (no prefix stripping needed)', () => {
      const sources: PortSource[] = [makePortSource({ variable_name: 'PORT', file_path: '.env' })];
      expect(
        portIsolationService.isPortTransformable('PORT', sources, projectPath, ['**/.env*'])
      ).toBe(true);
    });

    it('should handle project path ending with slash', () => {
      const sources: PortSource[] = [
        makePortSource({ variable_name: 'PORT', file_path: '/project/.env' }),
      ];
      expect(
        portIsolationService.isPortTransformable('PORT', sources, '/project/', ['**/.env*'])
      ).toBe(true);
    });
  });

  describe('registerWorktreeAssignments', () => {
    it('should add assignments for a new worktree', () => {
      const config = makeDefaultConfig();
      const assignments = [
        { variable_name: 'PORT', original_value: 3000, assigned_value: 3100 },
        { variable_name: 'DB_PORT', original_value: 5432, assigned_value: 5532 },
      ];

      const result = portIsolationService.registerWorktreeAssignments(
        config,
        'feature-a',
        assignments
      );

      expect(result.worktreeAssignments['feature-a']).toBeDefined();
      expect(result.worktreeAssignments['feature-a'].worktreeName).toBe('feature-a');
      expect(result.worktreeAssignments['feature-a'].assignments).toHaveLength(2);
      expect(result.worktreeAssignments['feature-a'].assignments[0]).toEqual({
        variableName: 'PORT',
        originalValue: 3000,
        assignedValue: 3100,
      });
      expect(result.worktreeAssignments['feature-a'].assignments[1]).toEqual({
        variableName: 'DB_PORT',
        originalValue: 5432,
        assignedValue: 5532,
      });
    });

    it('should preserve existing worktree assignments', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
        },
      });
      const assignments = [{ variable_name: 'PORT', original_value: 3000, assigned_value: 3200 }];

      const result = portIsolationService.registerWorktreeAssignments(
        config,
        'feature-b',
        assignments
      );

      expect(result.worktreeAssignments['feature-a']).toBeDefined();
      expect(result.worktreeAssignments['feature-b']).toBeDefined();
    });

    it('should handle empty assignments array', () => {
      const config = makeDefaultConfig();

      const result = portIsolationService.registerWorktreeAssignments(config, 'empty-wt', []);

      expect(result.worktreeAssignments['empty-wt']).toBeDefined();
      expect(result.worktreeAssignments['empty-wt'].assignments).toHaveLength(0);
    });

    it('should handle config with undefined worktreeAssignments', () => {
      const config = { enabled: true, targetFiles: ['**/.env*'] } as PortConfig;

      const assignments = [{ variable_name: 'PORT', original_value: 3000, assigned_value: 3100 }];

      const result = portIsolationService.registerWorktreeAssignments(
        config,
        'feature-a',
        assignments
      );

      expect(result.worktreeAssignments['feature-a']).toBeDefined();
    });
  });

  describe('removeWorktreeAssignments', () => {
    it('should remove assignments for a specific worktree', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
          'feature-b': {
            worktreeName: 'feature-b',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3200 }],
          },
        },
      });

      const result = portIsolationService.removeWorktreeAssignments(config, 'feature-a');

      expect(result.worktreeAssignments['feature-a']).toBeUndefined();
      expect(result.worktreeAssignments['feature-b']).toBeDefined();
    });

    it('should handle removing non-existent worktree gracefully', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
        },
      });

      const result = portIsolationService.removeWorktreeAssignments(config, 'nonexistent');

      expect(result.worktreeAssignments['feature-a']).toBeDefined();
    });

    it('should handle config with undefined worktreeAssignments', () => {
      const config = { enabled: true, targetFiles: ['**/.env*'] } as PortConfig;

      const result = portIsolationService.removeWorktreeAssignments(config, 'feature-a');

      expect(result).toBe(config); // Should return same reference
    });

    it('should return new object, not mutate original', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
        },
      });

      const result = portIsolationService.removeWorktreeAssignments(config, 'feature-a');

      expect(result).not.toBe(config);
      // Original should still have feature-a
      expect(config.worktreeAssignments['feature-a']).toBeDefined();
    });
  });

  describe('detectPorts (invoke wrapper)', () => {
    it('should invoke detect_ports command', async () => {
      const mockResult: DetectedPorts = {
        env_ports: [makePortSource()],
        dockerfile_ports: [],
        compose_ports: [],
        script_ports: [],
      };
      vi.mocked(invoke).mockResolvedValue(mockResult);

      const result = await portIsolationService.detectPorts('/path/to/project');

      expect(invoke).toHaveBeenCalledWith('detect_ports', { dirPath: '/path/to/project' });
      expect(result).toEqual(mockResult);
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Tauri error'));

      await expect(portIsolationService.detectPorts('/path')).rejects.toThrow('Tauri error');
    });
  });

  describe('allocatePorts (invoke wrapper)', () => {
    it('should invoke allocate_worktree_ports command', async () => {
      const ports = [makePortSource()];
      const mockResult = { assignments: [], overflow_warnings: [] };
      vi.mocked(invoke).mockResolvedValue(mockResult);

      const result = await portIsolationService.allocatePorts(ports, 1);

      expect(invoke).toHaveBeenCalledWith('allocate_worktree_ports', {
        ports,
        worktreeIndex: 1,
      });
      expect(result).toEqual(mockResult);
    });
  });

  describe('copyFilesWithPorts (invoke wrapper)', () => {
    it('should invoke copy_files_with_ports command', async () => {
      const assignments = [{ variable_name: 'PORT', original_value: 3000, assigned_value: 3100 }];
      const mockResult = { copied: ['file.env'], skipped: [], errors: [] };
      vi.mocked(invoke).mockResolvedValue(mockResult);

      const result = await portIsolationService.copyFilesWithPorts(
        '/source',
        '/target',
        ['**/.env*'],
        assignments
      );

      expect(invoke).toHaveBeenCalledWith('copy_files_with_ports', {
        sourcePath: '/source',
        targetPath: '/target',
        patterns: ['**/.env*'],
        assignments,
      });
      expect(result).toEqual(mockResult);
    });
  });

  describe('mergeDefaultTargetFiles', () => {
    it('should add new defaults to saved config missing them', () => {
      const config = makeDefaultConfig({
        targetFiles: ['**/.env*', '**/docker-compose.yml'],
      });
      const result = portIsolationService.mergeDefaultTargetFiles(config);
      // Should have added missing defaults
      expect(result.targetFiles).toContain('**/package.json');
      expect(result.targetFiles).toContain('**/compose.yml');
      expect(result.targetFiles).toContain('**/compose.yaml');
      // Should preserve existing
      expect(result.targetFiles).toContain('**/.env*');
      expect(result.targetFiles).toContain('**/docker-compose.yml');
    });

    it('should preserve user custom patterns', () => {
      const config = makeDefaultConfig({
        targetFiles: ['**/.env*', '**/custom.conf'],
      });
      const result = portIsolationService.mergeDefaultTargetFiles(config);
      expect(result.targetFiles).toContain('**/custom.conf');
    });

    it('should not re-add explicitly disabled patterns', () => {
      const config = makeDefaultConfig({
        targetFiles: ['**/.env*'],
        disabledTargetFiles: ['**/package.json'],
      });
      const result = portIsolationService.mergeDefaultTargetFiles(config);
      expect(result.targetFiles).not.toContain('**/package.json');
      // Other missing defaults should still be added
      expect(result.targetFiles).toContain('**/docker-compose.yml');
    });

    it('should return same object when all defaults already present', () => {
      const config = makeDefaultConfig({
        targetFiles: [...DEFAULT_TARGET_FILES],
      });
      const result = portIsolationService.mergeDefaultTargetFiles(config);
      expect(result).toBe(config); // Same reference
    });

    it('should handle undefined targetFiles as empty', () => {
      const config = makeDefaultConfig({
        targetFiles: undefined as unknown as string[],
      });
      const result = portIsolationService.mergeDefaultTargetFiles(config);
      expect(result.targetFiles).toEqual(DEFAULT_TARGET_FILES);
    });

    it('should handle undefined disabledTargetFiles', () => {
      const config = makeDefaultConfig({
        targetFiles: ['**/.env*'],
        disabledTargetFiles: undefined,
      });
      const result = portIsolationService.mergeDefaultTargetFiles(config);
      // Should add all missing defaults
      expect(result.targetFiles!.length).toBeGreaterThan(1);
      expect(result.targetFiles).toContain('**/package.json');
    });
  });

  describe('getUniqueEnvPorts - same variable name different port values', () => {
    it('should keep both entries when same variable has different port values', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        env_ports: [
          makePortSource({
            variable_name: 'DATABASE_URL',
            port_value: 5432,
            file_path: '.env',
          }),
          makePortSource({
            variable_name: 'DATABASE_URL',
            port_value: 5434,
            file_path: '.env.test',
          }),
        ],
      };
      const result = portIsolationService.getUniqueEnvPorts(detected);
      expect(result).toHaveLength(2);
      expect(result[0].port_value).toBe(5432);
      expect(result[1].port_value).toBe(5434);
    });

    it('should still deduplicate same (variable_name, port_value) across files', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        env_ports: [
          makePortSource({
            variable_name: 'PORT',
            port_value: 3000,
            file_path: '.env',
          }),
          makePortSource({
            variable_name: 'PORT',
            port_value: 3000,
            file_path: '.env.local',
          }),
        ],
      };
      const result = portIsolationService.getUniqueEnvPorts(detected);
      expect(result).toHaveLength(1);
      expect(result[0].port_value).toBe(3000);
    });

    it('should handle mix of same and different port values', () => {
      const detected: DetectedPorts = {
        ...makeEmptyDetected(),
        env_ports: [
          makePortSource({ variable_name: 'PORT', port_value: 3000, file_path: '.env' }),
          makePortSource({ variable_name: 'PORT', port_value: 3000, file_path: '.env.local' }),
          makePortSource({
            variable_name: 'DATABASE_URL',
            port_value: 5432,
            file_path: '.env',
          }),
          makePortSource({
            variable_name: 'DATABASE_URL',
            port_value: 5434,
            file_path: '.env.test',
          }),
        ],
      };
      const result = portIsolationService.getUniqueEnvPorts(detected);
      expect(result).toHaveLength(3); // PORT:3000, DATABASE_URL:5432, DATABASE_URL:5434
    });
  });

  describe('pruneOrphanedAssignments', () => {
    it('should remove assignments for worktrees that no longer exist', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
          'feature-b': {
            worktreeName: 'feature-b',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3200 }],
          },
          'feature-c': {
            worktreeName: 'feature-c',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3300 }],
          },
        },
      });

      const result = portIsolationService.pruneOrphanedAssignments(config, ['feature-b']);

      expect(Object.keys(result.worktreeAssignments)).toEqual(['feature-b']);
      expect(result.worktreeAssignments['feature-b'].assignments[0].assignedValue).toBe(3200);
    });

    it('should preserve all entries when all worktrees exist', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
          'feature-b': {
            worktreeName: 'feature-b',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3200 }],
          },
        },
      });

      const result = portIsolationService.pruneOrphanedAssignments(config, [
        'feature-a',
        'feature-b',
      ]);

      expect(result).toBe(config); // Same object reference
    });

    it('should return same object when worktreeAssignments is empty', () => {
      const config = makeDefaultConfig({ worktreeAssignments: {} });

      const result = portIsolationService.pruneOrphanedAssignments(config, ['feature-a']);

      expect(result).toBe(config);
    });

    it('should return same object when worktreeAssignments is undefined', () => {
      const config = makeDefaultConfig();
      // Force undefined to test guard clause
      (config as Record<string, unknown>).worktreeAssignments = undefined;

      const result = portIsolationService.pruneOrphanedAssignments(config as PortConfig, [
        'feature-a',
      ]);

      expect(result).toBe(config);
    });

    it('should remove all entries when no worktrees exist', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 3100 }],
          },
        },
      });

      const result = portIsolationService.pruneOrphanedAssignments(config, []);

      expect(Object.keys(result.worktreeAssignments)).toEqual([]);
    });
  });
});
