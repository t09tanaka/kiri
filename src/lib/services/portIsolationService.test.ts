import { describe, it, expect } from 'vitest';
import { portIsolationService } from './portIsolationService';
import type { DetectedPorts, PortSource } from './portIsolationService';
import type { PortConfig } from './persistenceService';

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
  return { env_ports: [], dockerfile_ports: [], compose_ports: [] };
}

function makeDefaultConfig(overrides: Partial<PortConfig> = {}): PortConfig {
  return {
    enabled: true,
    portRangeStart: 20000,
    portRangeEnd: 20099,
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
        env_ports: [makePortSource({ variable_name: 'PORT', port_value: 3000 })],
        dockerfile_ports: [
          makePortSource({ variable_name: 'EXPOSE', port_value: 3000, file_path: 'Dockerfile' }),
        ],
        compose_ports: [],
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

  describe('allocatePortsAvoidingUsed', () => {
    it('should allocate sequential ports for different port values', () => {
      const config = makeDefaultConfig();
      const ports: PortSource[] = [
        makePortSource({ variable_name: 'PORT', port_value: 3000 }),
        makePortSource({ variable_name: 'DB_PORT', port_value: 5432 }),
      ];

      const result = portIsolationService.allocatePortsAvoidingUsed(ports, config);

      expect(result).toHaveLength(2);
      expect(result[0]).toEqual({
        variable_name: 'PORT',
        original_value: 3000,
        assigned_value: 20000,
      });
      expect(result[1]).toEqual({
        variable_name: 'DB_PORT',
        original_value: 5432,
        assigned_value: 20001,
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

      const result = portIsolationService.allocatePortsAvoidingUsed(ports, config);

      expect(result).toHaveLength(4);
      // PORT=3000 and COMPOSE:3000 should get the same assigned_value
      expect(result[0].assigned_value).toBe(20000);
      expect(result[2].assigned_value).toBe(20000);
      // DB_PORT=5432 and COMPOSE:5432 should get the same assigned_value
      expect(result[1].assigned_value).toBe(20001);
      expect(result[3].assigned_value).toBe(20001);
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

      const result = portIsolationService.allocatePortsAvoidingUsed(ports, config);

      expect(result).toHaveLength(2);
      expect(result[0]).toEqual({
        variable_name: 'COMPOSE:3000',
        original_value: 3000,
        assigned_value: 20000,
      });
      expect(result[1]).toEqual({
        variable_name: 'COMPOSE:5432',
        original_value: 5432,
        assigned_value: 20001,
      });
    });

    it('should skip used ports from existing worktrees', () => {
      const config = makeDefaultConfig({
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [
              { variableName: 'PORT', originalValue: 3000, assignedValue: 20000 },
              { variableName: 'DB_PORT', originalValue: 5432, assignedValue: 20001 },
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

      const result = portIsolationService.allocatePortsAvoidingUsed(ports, config);

      expect(result).toHaveLength(2);
      // Should skip 20000 and 20001 (used by feature-a)
      expect(result[0].assigned_value).toBe(20002);
      // COMPOSE:3000 should reuse the same assignment as PORT since same port_value
      expect(result[1].assigned_value).toBe(20002);
    });

    it('should handle port range exhaustion', () => {
      const config = makeDefaultConfig({
        portRangeStart: 20000,
        portRangeEnd: 20000,
        worktreeAssignments: {
          'feature-a': {
            worktreeName: 'feature-a',
            assignments: [{ variableName: 'PORT', originalValue: 3000, assignedValue: 20000 }],
          },
        },
      });
      const ports: PortSource[] = [makePortSource({ variable_name: 'NEW_PORT', port_value: 8080 })];

      const result = portIsolationService.allocatePortsAvoidingUsed(ports, config);
      expect(result).toHaveLength(0);
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

      const result = portIsolationService.allocatePortsAvoidingUsed(ports, config);

      expect(result).toHaveLength(2);
      // Both should get the same assigned_value regardless of order
      expect(result[0].assigned_value).toBe(20000);
      expect(result[1].assigned_value).toBe(20000);
    });
  });
});
