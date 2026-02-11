import { describe, it, expect } from 'vitest';
import { composeIsolationService } from './composeIsolationService';
import type { DetectedComposeFiles } from './composeIsolationService';

describe('composeIsolationService', () => {
  describe('generateWorktreeName', () => {
    it('should append worktree name to original name', () => {
      const result = composeIsolationService.generateWorktreeName('myapp', 'feature-auth');
      expect(result).toBe('myapp-feature-auth');
    });
  });

  describe('hasDetectableNames', () => {
    it('should return false for empty files list', () => {
      const detected: DetectedComposeFiles = { files: [] };
      expect(composeIsolationService.hasDetectableNames(detected)).toBe(false);
    });

    it('should return false when no files have project names', () => {
      const detected: DetectedComposeFiles = {
        files: [
          {
            file_path: 'docker-compose.yml',
            project_name: null,
            name_line_number: 0,
            warnings: [],
          },
        ],
      };
      expect(composeIsolationService.hasDetectableNames(detected)).toBe(false);
    });

    it('should return true when at least one file has a project name', () => {
      const detected: DetectedComposeFiles = {
        files: [
          {
            file_path: 'docker-compose.yml',
            project_name: 'myapp',
            name_line_number: 1,
            warnings: [],
          },
        ],
      };
      expect(composeIsolationService.hasDetectableNames(detected)).toBe(true);
    });
  });

  describe('getAllWarnings', () => {
    it('should aggregate warnings from multiple files', () => {
      const detected: DetectedComposeFiles = {
        files: [
          {
            file_path: 'docker-compose.yml',
            project_name: 'myapp',
            name_line_number: 1,
            warnings: [
              {
                warning_type: 'ContainerName',
                value: 'myapp-web',
                line_number: 5,
                message: 'Hardcoded container_name detected',
              },
            ],
          },
          {
            file_path: 'docker-compose.prod.yml',
            project_name: 'myapp',
            name_line_number: 1,
            warnings: [
              {
                warning_type: 'VolumeName',
                value: 'myapp-data',
                line_number: 10,
                message: 'Named volume detected',
              },
              {
                warning_type: 'ContainerName',
                value: 'myapp-db',
                line_number: 15,
                message: 'Hardcoded container_name detected',
              },
            ],
          },
        ],
      };

      const warnings = composeIsolationService.getAllWarnings(detected);
      expect(warnings).toHaveLength(3);
      expect(warnings[0].warning_type).toBe('ContainerName');
      expect(warnings[1].warning_type).toBe('VolumeName');
      expect(warnings[2].warning_type).toBe('ContainerName');
    });

    it('should return empty array when no warnings exist', () => {
      const detected: DetectedComposeFiles = {
        files: [
          {
            file_path: 'docker-compose.yml',
            project_name: 'myapp',
            name_line_number: 1,
            warnings: [],
          },
        ],
      };

      expect(composeIsolationService.getAllWarnings(detected)).toEqual([]);
    });
  });

  describe('buildReplacements', () => {
    it('should build replacements for files with project names', () => {
      const detected: DetectedComposeFiles = {
        files: [
          {
            file_path: 'docker-compose.yml',
            project_name: 'myapp',
            name_line_number: 1,
            warnings: [],
          },
          {
            file_path: 'docker-compose.prod.yml',
            project_name: 'myapp-prod',
            name_line_number: 1,
            warnings: [],
          },
        ],
      };

      const replacements = composeIsolationService.buildReplacements(detected, 'feature-auth', []);

      expect(replacements).toHaveLength(2);
      expect(replacements[0]).toEqual({
        file_path: 'docker-compose.yml',
        original_name: 'myapp',
        new_name: 'myapp-feature-auth',
      });
      expect(replacements[1]).toEqual({
        file_path: 'docker-compose.prod.yml',
        original_name: 'myapp-prod',
        new_name: 'myapp-prod-feature-auth',
      });
    });

    it('should exclude files in the disabled list', () => {
      const detected: DetectedComposeFiles = {
        files: [
          {
            file_path: 'docker-compose.yml',
            project_name: 'myapp',
            name_line_number: 1,
            warnings: [],
          },
          {
            file_path: 'docker-compose.prod.yml',
            project_name: 'myapp-prod',
            name_line_number: 1,
            warnings: [],
          },
        ],
      };

      const replacements = composeIsolationService.buildReplacements(detected, 'feature-auth', [
        'docker-compose.prod.yml',
      ]);

      expect(replacements).toHaveLength(1);
      expect(replacements[0].file_path).toBe('docker-compose.yml');
    });

    it('should exclude files without a project name', () => {
      const detected: DetectedComposeFiles = {
        files: [
          {
            file_path: 'docker-compose.yml',
            project_name: 'myapp',
            name_line_number: 1,
            warnings: [],
          },
          {
            file_path: 'docker-compose.dev.yml',
            project_name: null,
            name_line_number: 0,
            warnings: [],
          },
        ],
      };

      const replacements = composeIsolationService.buildReplacements(detected, 'fix-bug', []);

      expect(replacements).toHaveLength(1);
      expect(replacements[0]).toEqual({
        file_path: 'docker-compose.yml',
        original_name: 'myapp',
        new_name: 'myapp-fix-bug',
      });
    });
  });
});
