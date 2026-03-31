import { describe, expect, it } from 'vitest';
import { buildCreateTaskList, buildOpenTaskList } from './worktreeFlowService';
import type { WorktreeInitCommand } from './persistenceService';
import type { PortAssignment } from './portIsolationService';
import type { ComposeNameReplacement } from './composeIsolationService';
import type { CreateFlowOptions } from './worktreeFlowService';

const noOpOptions: CreateFlowOptions = {
  gitignorePatterns: [],
  portAssignments: [],
  portConfig: null,
  composeReplacements: [],
  initCommands: [],
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
