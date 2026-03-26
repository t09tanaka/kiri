import { describe, it, expect, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { composeIsolationService } from './composeIsolationService';
import type { DetectedComposeFiles } from './composeIsolationService';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

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

  describe('detectComposeFiles', () => {
    it('should invoke detect_compose_files command', async () => {
      const mockResult: DetectedComposeFiles = {
        files: [
          {
            file_path: 'docker-compose.yml',
            project_name: 'myapp',
            name_line_number: 1,
            warnings: [],
          },
        ],
      };
      vi.mocked(invoke).mockResolvedValue(mockResult);

      const result = await composeIsolationService.detectComposeFiles('/path/to/project');

      expect(invoke).toHaveBeenCalledWith('detect_compose_files', { dirPath: '/path/to/project' });
      expect(result).toEqual(mockResult);
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Tauri error'));

      await expect(composeIsolationService.detectComposeFiles('/path/to/project')).rejects.toThrow(
        'Tauri error'
      );
    });
  });

  describe('applyComposeIsolation', () => {
    it('should invoke apply_compose_isolation command', async () => {
      const mockResult = {
        transformed_files: ['docker-compose.yml'],
        skipped_files: [],
        errors: [],
      };
      vi.mocked(invoke).mockResolvedValue(mockResult);

      const replacements = [
        {
          file_path: 'docker-compose.yml',
          original_name: 'myapp',
          new_name: 'myapp-feature',
        },
      ];

      const result = await composeIsolationService.applyComposeIsolation(
        '/path/to/worktree',
        replacements
      );

      expect(invoke).toHaveBeenCalledWith('apply_compose_isolation', {
        worktreePath: '/path/to/worktree',
        replacements,
      });
      expect(result).toEqual(mockResult);
    });

    it('should propagate errors from invoke', async () => {
      vi.mocked(invoke).mockRejectedValue(new Error('Apply error'));

      await expect(composeIsolationService.applyComposeIsolation('/path', [])).rejects.toThrow(
        'Apply error'
      );
    });
  });
});
