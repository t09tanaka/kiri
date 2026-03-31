import type { WorktreeInitCommand, PortConfig } from './persistenceService';
import type { PortAssignment } from './portIsolationService';
import type { ComposeNameReplacement } from './composeIsolationService';
export type { WorktreeInfo } from './worktreeService';
import type { WorktreeInfo } from './worktreeService';

// ============================================================================
// Types
// ============================================================================

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed';

export interface ProgressTask {
  id: string;
  name: string;
  status: TaskStatus;
  detail?: string;
}

export interface FlowCallbacks {
  onTaskUpdate: (taskId: string, status: TaskStatus, detail?: string) => void;
  onCancelCheck: () => boolean;
}

export interface CreateFlowOptions {
  gitignorePatterns: string[];
  portAssignments: PortAssignment[];
  portConfig: PortConfig | null;
  composeReplacements: ComposeNameReplacement[];
  initCommands: WorktreeInitCommand[];
}

export interface CreateFlowResult {
  worktreeInfo: WorktreeInfo;
  portConfigUpdated?: PortConfig;
}

// ============================================================================
// Task list builders
// ============================================================================

/**
 * Build the task list for the create worktree flow.
 * Order: worktree → copy → [port-remap] → [compose-name] → [init-N...] → open-window
 */
export function buildCreateTaskList(
  branchName: string,
  options: CreateFlowOptions
): ProgressTask[] {
  const tasks: ProgressTask[] = [
    { id: 'worktree', name: `Create worktree '${branchName}'`, status: 'pending' },
    { id: 'copy', name: 'Copy files', status: 'pending' },
  ];

  if (options.portAssignments.length > 0) {
    tasks.push({ id: 'port-remap', name: 'Remap ports', status: 'pending' });
  }

  if (options.composeReplacements.length > 0) {
    tasks.push({ id: 'compose-name', name: 'Isolate compose names', status: 'pending' });
  }

  for (let i = 0; i < options.initCommands.length; i++) {
    tasks.push({ id: `init-${i}`, name: options.initCommands[i].name, status: 'pending' });
  }

  tasks.push({ id: 'open-window', name: 'Open worktree window', status: 'pending' });

  return tasks;
}

/**
 * Build the task list for the open (existing) worktree flow.
 * Order: [init-N...] → open-window
 */
export function buildOpenTaskList(initCommands: WorktreeInitCommand[]): ProgressTask[] {
  const tasks: ProgressTask[] = [];

  for (let i = 0; i < initCommands.length; i++) {
    tasks.push({ id: `init-${i}`, name: initCommands[i].name, status: 'pending' });
  }

  tasks.push({ id: 'open-window', name: 'Open worktree window', status: 'pending' });

  return tasks;
}
