import { describe, expect, it, vi, beforeEach } from 'vitest';
import { buildCreateTaskList, buildOpenTaskList } from './worktreeFlowService';
import { executeCreateFlow, executeOpenFlow } from './worktreeFlowService';
import type { WorktreeInitCommand } from './persistenceService';
import type { PortAssignment } from './portIsolationService';
import type { ComposeNameReplacement } from './composeIsolationService';
import type { CreateFlowOptions, FlowCallbacks } from './worktreeFlowService';

vi.mock('./worktreeService', () => ({
  worktreeService: {
    create: vi.fn(),
    copyGitignoredFiles: vi.fn(),
    runInitCommand: vi.fn(),
  },
}));
vi.mock('./portIsolationService', () => ({
  portIsolationService: {
    copyFilesWithPorts: vi.fn(),
    registerWorktreeAssignments: vi.fn(),
  },
}));
vi.mock('./composeIsolationService', () => ({
  composeIsolationService: {
    applyComposeIsolation: vi.fn(),
  },
}));
vi.mock('../utils/gitWorktree', () => ({
  branchToWorktreeName: vi.fn((name: string) => name),
}));

// Import mocked modules for assertions
import { worktreeService } from './worktreeService';
import { portIsolationService } from './portIsolationService';
import { composeIsolationService } from './composeIsolationService';

const noOpOptions: CreateFlowOptions = {
  gitignorePatterns: [],
  portAssignments: [],
  portConfig: null,
  composeReplacements: [],
  initCommands: [],
};

const makeCallbacks = (
  cancelAfter = -1
): { callbacks: FlowCallbacks; updates: [string, string, string | undefined][] } => {
  let callCount = 0;
  const updates: [string, string, string | undefined][] = [];
  const callbacks: FlowCallbacks = {
    onTaskUpdate: (id, status, detail) => {
      updates.push([id, status, detail]);
    },
    onCancelCheck: () => {
      callCount++;
      return cancelAfter >= 0 && callCount > cancelAfter;
    },
  };
  return { callbacks, updates };
};

describe('buildCreateTaskList', () => {
  it('returns minimal task list when no ports, compose, or init commands', () => {
    const tasks = buildCreateTaskList('feature-foo', noOpOptions);
    expect(tasks.map((t) => t.id)).toEqual(['worktree', 'copy', 'open-window']);
    expect(tasks.every((t) => t.status === 'pending')).toBe(true);
  });

  it('includes worktree task name with branch name', () => {
    const tasks = buildCreateTaskList('my-branch', noOpOptions);
    const worktreeTask = tasks.find((t) => t.id === 'worktree');
    expect(worktreeTask?.name).toBe("Create worktree 'my-branch'");
  });

  it('includes port-remap task when portAssignments is non-empty', () => {
    const portAssignments: PortAssignment[] = [
      { variable_name: 'PORT', original_value: 3000, assigned_value: 20001 },
    ];
    const tasks = buildCreateTaskList('feature-ports', {
      ...noOpOptions,
      portAssignments,
    });
    expect(tasks.map((t) => t.id)).toContain('port-remap');
  });

  it('does not include port-remap task when portAssignments is empty', () => {
    const tasks = buildCreateTaskList('feature-foo', noOpOptions);
    expect(tasks.map((t) => t.id)).not.toContain('port-remap');
  });

  it('includes compose-name task when composeReplacements is non-empty', () => {
    const composeReplacements: ComposeNameReplacement[] = [
      { file_path: 'docker-compose.yml', original_name: 'myapp', new_name: 'myapp-feat' },
    ];
    const tasks = buildCreateTaskList('feature-compose', {
      ...noOpOptions,
      composeReplacements,
    });
    expect(tasks.map((t) => t.id)).toContain('compose-name');
  });

  it('does not include compose-name task when composeReplacements is empty', () => {
    const tasks = buildCreateTaskList('feature-foo', noOpOptions);
    expect(tasks.map((t) => t.id)).not.toContain('compose-name');
  });

  it('includes init tasks for each init command with correct names', () => {
    const initCommands: WorktreeInitCommand[] = [
      { name: 'Install dependencies', command: 'npm install', enabled: true },
      { name: 'Build project', command: 'npm run build', enabled: true },
    ];
    const tasks = buildCreateTaskList('feature-init', {
      ...noOpOptions,
      initCommands,
    });
    const initTask0 = tasks.find((t) => t.id === 'init-0');
    const initTask1 = tasks.find((t) => t.id === 'init-1');
    expect(initTask0?.name).toBe('Install dependencies');
    expect(initTask1?.name).toBe('Build project');
  });

  it('produces correct order with all options present', () => {
    const portAssignments: PortAssignment[] = [
      { variable_name: 'PORT', original_value: 3000, assigned_value: 20001 },
    ];
    const composeReplacements: ComposeNameReplacement[] = [
      { file_path: 'docker-compose.yml', original_name: 'myapp', new_name: 'myapp-feat' },
    ];
    const initCommands: WorktreeInitCommand[] = [
      { name: 'Install dependencies', command: 'npm install', enabled: true },
    ];
    const tasks = buildCreateTaskList('feature-all', {
      ...noOpOptions,
      portAssignments,
      composeReplacements,
      initCommands,
    });
    expect(tasks.map((t) => t.id)).toEqual([
      'worktree',
      'copy',
      'port-remap',
      'compose-name',
      'init-0',
      'open-window',
    ]);
  });

  it('all tasks start with status pending', () => {
    const initCommands: WorktreeInitCommand[] = [
      { name: 'Install', command: 'npm install', enabled: true },
    ];
    const portAssignments: PortAssignment[] = [
      { variable_name: 'PORT', original_value: 3000, assigned_value: 20001 },
    ];
    const tasks = buildCreateTaskList('branch', {
      ...noOpOptions,
      portAssignments,
      initCommands,
    });
    expect(tasks.every((t) => t.status === 'pending')).toBe(true);
  });
});

describe('buildOpenTaskList', () => {
  it('returns only open-window when no init commands', () => {
    const tasks = buildOpenTaskList([]);
    expect(tasks.map((t) => t.id)).toEqual(['open-window']);
    expect(tasks[0].status).toBe('pending');
  });

  it('includes init tasks before open-window with correct ids and names', () => {
    const initCommands: WorktreeInitCommand[] = [
      { name: 'Install dependencies', command: 'npm install', enabled: true },
      { name: 'Migrate DB', command: 'npm run db:migrate', enabled: true },
    ];
    const tasks = buildOpenTaskList(initCommands);
    expect(tasks.map((t) => t.id)).toEqual(['init-0', 'init-1', 'open-window']);
    expect(tasks[0].name).toBe('Install dependencies');
    expect(tasks[1].name).toBe('Migrate DB');
  });

  it('all tasks start with status pending', () => {
    const initCommands: WorktreeInitCommand[] = [
      { name: 'Install', command: 'npm install', enabled: true },
    ];
    const tasks = buildOpenTaskList(initCommands);
    expect(tasks.every((t) => t.status === 'pending')).toBe(true);
  });
});

describe('executeCreateFlow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('full flow with gitignore copy + init commands — verifies service calls and task updates', async () => {
    const mockWorktreeInfo = {
      name: 'feature-auth',
      path: '/repo-feature-auth',
      branch: 'feature/auth',
      is_locked: false,
      is_main: false,
      is_valid: true,
    };
    vi.mocked(worktreeService.create).mockResolvedValue(mockWorktreeInfo);
    vi.mocked(worktreeService.copyGitignoredFiles).mockResolvedValue({
      copied_files: ['a.env', 'b.env'],
      skipped_files: [],
      transformed_files: [],
      errors: [],
    });
    vi.mocked(worktreeService.runInitCommand).mockResolvedValue({
      success: true,
      stdout: '',
      stderr: '',
      exit_code: 0,
    });

    const options: CreateFlowOptions = {
      ...noOpOptions,
      gitignorePatterns: ['.env'],
      initCommands: [{ name: 'Install', command: 'npm install', enabled: true }],
    };
    const { callbacks, updates } = makeCallbacks();

    const result = await executeCreateFlow('/repo', 'feature/auth', true, options, callbacks);

    expect(result.worktreeInfo).toBe(mockWorktreeInfo);
    expect(worktreeService.create).toHaveBeenCalledWith(
      '/repo',
      'feature/auth',
      'feature/auth',
      false
    );
    expect(worktreeService.copyGitignoredFiles).toHaveBeenCalledWith(
      '/repo',
      '/repo-feature-auth',
      ['.env']
    );
    expect(worktreeService.runInitCommand).toHaveBeenCalledWith(
      '/repo-feature-auth',
      'npm install'
    );

    // Task updates: worktree running/completed, copy running/completed, init-0 running/completed
    expect(updates).toContainEqual(['worktree', 'running', undefined]);
    expect(updates).toContainEqual(['worktree', 'completed', undefined]);
    expect(updates).toContainEqual(['copy', 'running', undefined]);
    expect(updates.find(([id, status]) => id === 'copy' && status === 'completed')).toBeTruthy();
    expect(updates).toContainEqual(['init-0', 'running', undefined]);
    expect(updates).toContainEqual(['init-0', 'completed', undefined]);
  });

  it('uses copyFilesWithPorts when portAssignments present', async () => {
    const mockWorktreeInfo = {
      name: 'feature-port',
      path: '/repo-feature-port',
      branch: 'feature/port',
      is_locked: false,
      is_main: false,
      is_valid: true,
    };
    vi.mocked(worktreeService.create).mockResolvedValue(mockWorktreeInfo);
    vi.mocked(portIsolationService.copyFilesWithPorts).mockResolvedValue({
      copied_files: ['a.env'],
      skipped_files: [],
      transformed_files: ['a.env'],
      errors: [],
    });
    vi.mocked(portIsolationService.registerWorktreeAssignments).mockReturnValue({
      enabled: true,
      portRangeStart: 20000,
      portRangeEnd: 39999,
      nextPort: 20002,
      customRules: [],
      worktreeAssignments: {},
    });

    const portAssignments: PortAssignment[] = [
      { variable_name: 'PORT', original_value: 3000, assigned_value: 20001 },
    ];
    const portConfig = {
      enabled: true,
      portRangeStart: 20000,
      portRangeEnd: 39999,
      nextPort: 20001,
      customRules: [],
      worktreeAssignments: {},
    };
    const options: CreateFlowOptions = {
      ...noOpOptions,
      gitignorePatterns: ['.env'],
      portAssignments,
      portConfig,
    };
    const { callbacks } = makeCallbacks();

    const result = await executeCreateFlow('/repo', 'feature/port', false, options, callbacks);

    expect(portIsolationService.copyFilesWithPorts).toHaveBeenCalledWith(
      '/repo',
      '/repo-feature-port',
      ['.env'],
      portAssignments
    );
    expect(worktreeService.copyGitignoredFiles).not.toHaveBeenCalled();
    expect(portIsolationService.registerWorktreeAssignments).toHaveBeenCalledWith(
      portConfig,
      'feature/port',
      portAssignments
    );
    expect(result.portConfigUpdated).toBeDefined();
  });

  it('applies compose isolation when composeReplacements present', async () => {
    const mockWorktreeInfo = {
      name: 'feature-compose',
      path: '/repo-feature-compose',
      branch: 'feature/compose',
      is_locked: false,
      is_main: false,
      is_valid: true,
    };
    vi.mocked(worktreeService.create).mockResolvedValue(mockWorktreeInfo);
    vi.mocked(composeIsolationService.applyComposeIsolation).mockResolvedValue({
      files_modified: ['docker-compose.yml'],
      replacements_made: 1,
    });

    const composeReplacements: ComposeNameReplacement[] = [
      {
        file_path: 'docker-compose.yml',
        original_name: 'myapp',
        new_name: 'myapp-feature-compose',
      },
    ];
    const options: CreateFlowOptions = {
      ...noOpOptions,
      composeReplacements,
    };
    const { callbacks, updates } = makeCallbacks();

    await executeCreateFlow('/repo', 'feature/compose', true, options, callbacks);

    expect(composeIsolationService.applyComposeIsolation).toHaveBeenCalledWith(
      '/repo-feature-compose',
      composeReplacements
    );
    expect(
      updates.find(([id, status]) => id === 'compose-name' && status === 'completed')
    ).toBeTruthy();
  });

  it('stops on cancel — verify init command not called after cancel', async () => {
    const mockWorktreeInfo = {
      name: 'feature-cancel',
      path: '/repo-feature-cancel',
      branch: 'feature/cancel',
      is_locked: false,
      is_main: false,
      is_valid: true,
    };
    vi.mocked(worktreeService.create).mockResolvedValue(mockWorktreeInfo);

    const options: CreateFlowOptions = {
      ...noOpOptions,
      initCommands: [{ name: 'Install', command: 'npm install', enabled: true }],
    };
    // Cancel immediately after the first onCancelCheck call (after worktree creation)
    const { callbacks } = makeCallbacks(0);

    const result = await executeCreateFlow('/repo', 'feature/cancel', true, options, callbacks);

    expect(result.worktreeInfo).toBe(mockWorktreeInfo);
    expect(worktreeService.runInitCommand).not.toHaveBeenCalled();
  });
});

describe('executeOpenFlow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('runs all init commands in order', async () => {
    vi.mocked(worktreeService.runInitCommand).mockResolvedValue({
      success: true,
      stdout: '',
      stderr: '',
      exit_code: 0,
    });

    const initCommands: WorktreeInitCommand[] = [
      { name: 'Install', command: 'npm install', enabled: true },
      { name: 'Migrate', command: 'npm run migrate', enabled: true },
    ];
    const { callbacks, updates } = makeCallbacks();

    await executeOpenFlow('/wt-path', initCommands, callbacks);

    expect(worktreeService.runInitCommand).toHaveBeenCalledTimes(2);
    expect(worktreeService.runInitCommand).toHaveBeenNthCalledWith(1, '/wt-path', 'npm install');
    expect(worktreeService.runInitCommand).toHaveBeenNthCalledWith(
      2,
      '/wt-path',
      'npm run migrate'
    );
    expect(updates).toContainEqual(['init-0', 'running', undefined]);
    expect(updates).toContainEqual(['init-0', 'completed', undefined]);
    expect(updates).toContainEqual(['init-1', 'running', undefined]);
    expect(updates).toContainEqual(['init-1', 'completed', undefined]);
  });

  it('stops on cancel', async () => {
    vi.mocked(worktreeService.runInitCommand).mockResolvedValue({
      success: true,
      stdout: '',
      stderr: '',
      exit_code: 0,
    });

    const initCommands: WorktreeInitCommand[] = [
      { name: 'First', command: 'cmd1', enabled: true },
      { name: 'Second', command: 'cmd2', enabled: true },
    ];
    // Cancel immediately (before any command runs)
    const { callbacks } = makeCallbacks(0);

    await executeOpenFlow('/wt-path', initCommands, callbacks);

    expect(worktreeService.runInitCommand).not.toHaveBeenCalled();
  });

  it('reports failed init commands with last line of stderr', async () => {
    vi.mocked(worktreeService.runInitCommand).mockResolvedValue({
      success: false,
      stdout: '',
      stderr: 'line1\nline2\nactual error message',
      exit_code: 1,
    });

    const initCommands: WorktreeInitCommand[] = [
      { name: 'Fail cmd', command: 'bad-cmd', enabled: true },
    ];
    const { callbacks, updates } = makeCallbacks();

    await executeOpenFlow('/wt-path', initCommands, callbacks);

    const failUpdate = updates.find(([id, status]) => id === 'init-0' && status === 'failed');
    expect(failUpdate).toBeDefined();
    expect(failUpdate![2]).toBe('actual error message');
  });
});
