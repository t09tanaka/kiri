import { Store } from '@tauri-apps/plugin-store';

const STORE_PATH = 'kiri-settings.json';

// ============================================================================
// Startup Command
// ============================================================================

export type StartupCommand = 'none' | 'claude' | 'codex';

export const DEFAULT_STARTUP_COMMAND: StartupCommand = 'none';

export interface StartupCommandOption {
  id: StartupCommand;
  label: string;
  command: string;
}

export const STARTUP_COMMANDS: StartupCommandOption[] = [
  { id: 'none', label: 'None', command: '' },
  { id: 'claude', label: 'Claude', command: 'claude' },
  { id: 'codex', label: 'Codex', command: 'codex' },
];

/**
 * Get the shell command string for a startup command setting
 */
export function getStartupCommandString(id: StartupCommand): string {
  const cmd = STARTUP_COMMANDS.find((c) => c.id === id);
  return cmd?.command ?? '';
}

// ============================================================================
// Global Settings
// ============================================================================

// Global settings (shared across all windows)
export interface PersistedSettings {
  fontSize: number;
  startupCommand: StartupCommand;
}

const DEFAULT_SETTINGS: PersistedSettings = {
  fontSize: 13,
  startupCommand: DEFAULT_STARTUP_COMMAND,
};

let store: Store | null = null;

async function getStore(): Promise<Store> {
  if (!store) {
    store = await Store.load(STORE_PATH);
  }
  return store;
}

/**
 * Load global settings (font size, etc.)
 */
export async function loadSettings(): Promise<PersistedSettings> {
  try {
    const s = await getStore();
    await s.reload();

    const settings = await s.get<PersistedSettings>('globalSettings');
    if (!settings) {
      return { ...DEFAULT_SETTINGS };
    }

    return {
      fontSize: settings.fontSize ?? DEFAULT_SETTINGS.fontSize,
      startupCommand: settings.startupCommand ?? DEFAULT_SETTINGS.startupCommand,
    };
  } catch (error) {
    console.error('Failed to load settings:', error);
    return { ...DEFAULT_SETTINGS };
  }
}

/**
 * Save global settings
 */
export async function saveSettings(settings: PersistedSettings): Promise<void> {
  try {
    const s = await getStore();
    await s.set('globalSettings', settings);
    await s.save();
  } catch (error) {
    console.error('Failed to save settings:', error);
  }
}

/**
 * Get default settings
 */
export function getDefaultSettings(): PersistedSettings {
  return { ...DEFAULT_SETTINGS };
}

// ============================================================================
// Project-specific settings
// ============================================================================

/**
 * Worktree initialization command configuration
 */
export interface WorktreeInitCommand {
  name: string; // Display name (e.g., "Install dependencies")
  command: string; // Shell command (e.g., "npm install")
  enabled: boolean; // Whether to run this command
  auto: boolean; // True if auto-detected, false if user-added
}

/**
 * Port assignment for a single variable
 */
export interface PortAssignment {
  variableName: string;
  originalValue: number;
  assignedValue: number;
}

/**
 * Port assignments for a worktree
 */
export interface WorktreePortAssignment {
  worktreeName: string;
  assignments: PortAssignment[];
}

/**
 * Port isolation configuration
 */
export interface PortConfig {
  enabled: boolean;
  worktreeAssignments: Record<string, WorktreePortAssignment>;
  targetFiles: string[]; // File patterns to process (default: ['.env*', 'docker-compose.yml'])
  disabledTargetFiles?: string[]; // Target files that are disabled (not processed)
}

/**
 * Compose isolation configuration
 */
export interface ComposeIsolationConfig {
  enabled: boolean;
  disabledFiles: string[];
}

/**
 * Project-specific settings (stored per project path)
 */
export interface ProjectSettings {
  searchExcludePatterns: string[];
  worktreeCopyPatterns: string[];
  worktreeInitCommands: WorktreeInitCommand[];
  portConfig?: PortConfig;
  composeIsolationConfig?: ComposeIsolationConfig;
}

/**
 * Default exclude patterns for content search
 */
export const DEFAULT_EXCLUDE_PATTERNS: string[] = [
  '*.min.js',
  '*.min.css',
  '*.map',
  '*.lock',
  'package-lock.json',
  'yarn.lock',
  'pnpm-lock.yaml',
  '.DS_Store',
  'Thumbs.db',
  '*.log',
];

/**
 * Default copy patterns for worktree creation (cannot be removed)
 */
export const DEFAULT_WORKTREE_COPY_PATTERNS: string[] = ['**/.env*'];

const DEFAULT_PROJECT_SETTINGS: ProjectSettings = {
  searchExcludePatterns: [...DEFAULT_EXCLUDE_PATTERNS],
  worktreeCopyPatterns: [],
  worktreeInitCommands: [],
};

/**
 * Normalize project path for use as a store key
 */
function normalizeProjectPath(projectPath: string): string {
  // Replace special characters that might cause issues in keys
  return projectPath.replace(/[/\\:]/g, '_');
}

/**
 * Load project-specific settings
 */
export async function loadProjectSettings(projectPath: string): Promise<ProjectSettings> {
  try {
    const s = await getStore();
    await s.reload();

    const key = `project_${normalizeProjectPath(projectPath)}`;
    const settings = await s.get<ProjectSettings>(key);

    if (!settings) {
      return { ...DEFAULT_PROJECT_SETTINGS };
    }

    return {
      searchExcludePatterns:
        settings.searchExcludePatterns ?? DEFAULT_PROJECT_SETTINGS.searchExcludePatterns,
      worktreeCopyPatterns:
        settings.worktreeCopyPatterns ?? DEFAULT_PROJECT_SETTINGS.worktreeCopyPatterns,
      worktreeInitCommands:
        settings.worktreeInitCommands ?? DEFAULT_PROJECT_SETTINGS.worktreeInitCommands,
      portConfig: settings.portConfig,
      composeIsolationConfig: settings.composeIsolationConfig,
    };
  } catch (error) {
    console.error('Failed to load project settings:', error);
    return { ...DEFAULT_PROJECT_SETTINGS };
  }
}

/**
 * Save project-specific settings
 */
export async function saveProjectSettings(
  projectPath: string,
  settings: ProjectSettings
): Promise<void> {
  try {
    const s = await getStore();
    const key = `project_${normalizeProjectPath(projectPath)}`;
    await s.set(key, settings);
    await s.save();
  } catch (error) {
    console.error('Failed to save project settings:', error);
  }
}

/**
 * Get default project settings
 */
export function getDefaultProjectSettings(): ProjectSettings {
  return { ...DEFAULT_PROJECT_SETTINGS };
}
