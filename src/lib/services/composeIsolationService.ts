import { invoke } from '@tauri-apps/api/core';

export interface ComposeFileInfo {
  file_path: string;
  project_name: string | null;
  name_line_number: number;
  warnings: ComposeWarning[];
}

export interface ComposeWarning {
  warning_type: 'ContainerName' | 'VolumeName';
  value: string;
  line_number: number;
  message: string;
}

export interface DetectedComposeFiles {
  files: ComposeFileInfo[];
}

export interface ComposeNameReplacement {
  file_path: string;
  original_name: string;
  new_name: string;
}

export interface ComposeTransformResult {
  transformed_files: string[];
  skipped_files: string[];
  errors: string[];
}

/**
 * Compose isolation service for worktrees
 * Wraps Tauri compose isolation commands and provides pure helper functions
 */
export const composeIsolationService = {
  /**
   * Detect compose files in a directory
   */
  detectComposeFiles: (dirPath: string): Promise<DetectedComposeFiles> =>
    invoke('detect_compose_files', { dirPath }),

  /**
   * Apply compose isolation by replacing project names in worktree compose files
   */
  applyComposeIsolation: (
    worktreePath: string,
    replacements: ComposeNameReplacement[]
  ): Promise<ComposeTransformResult> =>
    invoke('apply_compose_isolation', { worktreePath, replacements }),

  /**
   * Generate a worktree-specific project name
   */
  generateWorktreeName: (originalName: string, worktreeName: string): string =>
    `${originalName}-${worktreeName}`,

  /**
   * Check if any compose files have detectable project names
   */
  hasDetectableNames: (detected: DetectedComposeFiles): boolean =>
    detected.files.some((f) => f.project_name !== null),

  /**
   * Get all warnings from all detected compose files
   */
  getAllWarnings: (detected: DetectedComposeFiles): ComposeWarning[] =>
    detected.files.flatMap((f) => f.warnings),

  /**
   * Build replacement list for compose isolation
   * Filters out files without a project name and files in the disabled list
   */
  buildReplacements: (
    detected: DetectedComposeFiles,
    worktreeName: string,
    disabledFiles: string[]
  ): ComposeNameReplacement[] => {
    const disabledSet = new Set(disabledFiles);

    return detected.files
      .filter((f) => f.project_name !== null && !disabledSet.has(f.file_path))
      .map((f) => ({
        file_path: f.file_path,
        original_name: f.project_name!,
        new_name: composeIsolationService.generateWorktreeName(f.project_name!, worktreeName),
      }));
  },
};
