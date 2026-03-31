import type { WorktreeInitCommand, PortConfig } from './persistenceService';
import type { PortAssignment } from './portIsolationService';
import type { ComposeNameReplacement } from './composeIsolationService';
export type { WorktreeInfo } from './worktreeService';
import type { WorktreeInfo } from './worktreeService';
import { worktreeService } from './worktreeService';
import { portIsolationService } from './portIsolationService';
import { composeIsolationService } from './composeIsolationService';
import { branchToWorktreeName } from '../utils/gitWorktree';

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
  onTaskUpdate: (taskId: string, status: TaskStatus, detail?: string) => void | Promise<void>;
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

// ============================================================================
// Flow executors
// ============================================================================

/**
 * Execute the full create worktree flow.
 * Returns when all steps complete or a cancel is requested.
 */
export async function executeCreateFlow(
  repoPath: string,
  branchName: string,
  isExistingBranch: boolean,
  options: CreateFlowOptions,
  callbacks: FlowCallbacks
): Promise<CreateFlowResult> {
  const { gitignorePatterns, portAssignments, portConfig, composeReplacements, initCommands } =
    options;
  const { onTaskUpdate, onCancelCheck } = callbacks;

  // Step 1: Create worktree
  await onTaskUpdate('worktree', 'running');
  const wtName = branchToWorktreeName(branchName);
  const worktreeInfo = await worktreeService.create(
    repoPath,
    wtName,
    branchName,
    !isExistingBranch
  );
  await onTaskUpdate('worktree', 'completed');

  // Check cancel
  if (onCancelCheck()) {
    return { worktreeInfo };
  }

  // Step 2: Copy files
  let copyFailed = false;
  await onTaskUpdate('copy', 'running');
  if (gitignorePatterns.length > 0) {
    try {
      let copyResult;
      if (portAssignments.length > 0) {
        copyResult = await portIsolationService.copyFilesWithPorts(
          repoPath,
          worktreeInfo.path,
          gitignorePatterns,
          portAssignments
        );
      } else {
        copyResult = await worktreeService.copyGitignoredFiles(
          repoPath,
          worktreeInfo.path,
          gitignorePatterns
        );
      }
      const parts: string[] = [`${copyResult.copied_files.length} files copied`];
      if (copyResult.transformed_files.length > 0) {
        parts.push(`${copyResult.transformed_files.length} transformed`);
      }
      await onTaskUpdate('copy', 'completed', parts.join(', '));
    } catch (err) {
      copyFailed = true;
      await onTaskUpdate('copy', 'failed', String(err));
    }
  } else {
    await onTaskUpdate('copy', 'completed', 'No copy rules enabled');
  }

  // Check cancel
  if (onCancelCheck()) {
    return { worktreeInfo };
  }

  // Step 3: Remap ports
  let portConfigUpdated: PortConfig | undefined;
  if (portAssignments.length > 0) {
    await onTaskUpdate('port-remap', 'running');
    if (!copyFailed) {
      const updated = portIsolationService.registerWorktreeAssignments(
        portConfig!,
        wtName,
        portAssignments
      );
      portConfigUpdated = updated;
      await onTaskUpdate('port-remap', 'completed');
    } else {
      await onTaskUpdate('port-remap', 'failed', 'Skipped due to copy failure');
    }
  }

  // Check cancel
  if (onCancelCheck()) {
    return { worktreeInfo, portConfigUpdated };
  }

  // Step 4: Compose isolation
  if (composeReplacements.length > 0) {
    await onTaskUpdate('compose-name', 'running');
    await composeIsolationService.applyComposeIsolation(worktreeInfo.path, composeReplacements);
    await onTaskUpdate('compose-name', 'completed');
  }

  // Check cancel
  if (onCancelCheck()) {
    return { worktreeInfo, portConfigUpdated };
  }

  // Step 5: Init commands
  for (let i = 0; i < initCommands.length; i++) {
    if (onCancelCheck()) {
      return { worktreeInfo, portConfigUpdated };
    }
    const cmd = initCommands[i];
    const taskId = `init-${i}`;
    await onTaskUpdate(taskId, 'running');
    const result = await worktreeService.runInitCommand(worktreeInfo.path, cmd.command);
    if (result.success) {
      await onTaskUpdate(taskId, 'completed');
    } else {
      const lines = result.stderr.trim().split('\n');
      const detail = lines[lines.length - 1] || result.stderr;
      await onTaskUpdate(taskId, 'failed', detail);
    }
  }

  return { worktreeInfo, portConfigUpdated };
}

/**
 * Execute the open (existing) worktree flow.
 * Runs init commands in order, respecting cancel.
 */
export async function executeOpenFlow(
  worktreePath: string,
  initCommands: WorktreeInitCommand[],
  callbacks: FlowCallbacks
): Promise<void> {
  const { onTaskUpdate, onCancelCheck } = callbacks;

  for (let i = 0; i < initCommands.length; i++) {
    if (onCancelCheck()) {
      return;
    }
    const cmd = initCommands[i];
    const taskId = `init-${i}`;
    await onTaskUpdate(taskId, 'running');
    const result = await worktreeService.runInitCommand(worktreePath, cmd.command);
    if (result.success) {
      await onTaskUpdate(taskId, 'completed');
    } else {
      const lines = result.stderr.trim().split('\n');
      const detail = lines[lines.length - 1] || result.stderr;
      await onTaskUpdate(taskId, 'failed', detail);
    }
  }
}
