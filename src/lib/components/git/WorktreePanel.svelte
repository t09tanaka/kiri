<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { Spinner } from '@/lib/components/ui';
  import { worktreeStore } from '@/lib/stores/worktreeStore';
  import { worktreeService } from '@/lib/services/worktreeService';
  import { windowService } from '@/lib/services/windowService';
  import { eventService } from '@/lib/services/eventService';
  import {
    loadProjectSettings,
    saveProjectSettings,
    DEFAULT_WORKTREE_COPY_PATTERNS,
    type WorktreeInitCommand,
    type PortConfig,
  } from '@/lib/services/persistenceService';
  import type {
    WorktreeInfo,
    BranchInfo,
    WorktreeContext,
    PackageManager,
  } from '@/lib/services/worktreeService';
  import {
    portIsolationService,
    DEFAULT_PORT_RANGE_START,
    DEFAULT_PORT_RANGE_END,
    DEFAULT_PORT_BLOCK_SIZE,
    DEFAULT_TARGET_FILES,
    type DetectedPorts,
    type PortAssignment,
    type PortSource,
  } from '@/lib/services/portIsolationService';
  import {
    composeIsolationService,
    type DetectedComposeFiles,
    type ComposeNameReplacement,
  } from '@/lib/services/composeIsolationService';
  import type { ComposeIsolationConfig } from '@/lib/services/persistenceService';
  import { branchToWorktreeName } from '@/lib/utils/gitWorktree';
  import { formatRelativeTime } from '@/lib/utils/dateFormat';

  interface Props {
    projectPath: string;
    onClose: () => void;
  }

  let { projectPath, onClose }: Props = $props();

  let mounted = $state(false);

  // Initialization loading state
  let isInitializing = $state(true);

  // Create form state
  let createName = $state('');
  let isCreating = $state(false);
  let branches = $state<BranchInfo[]>([]);
  let createError = $state<string | null>(null);
  let showBranchDropdown = $state(false);
  let isExistingBranch = $state(false);

  // Worktree context for current window
  let currentContext = $state<WorktreeContext | null>(null);

  // Copy settings state
  let showCopySettingsModal = $state(false);
  let userCopyPatterns = $state<string[]>([]);
  let newCopyPattern = $state('');

  // Init commands state
  let initCommands = $state<WorktreeInitCommand[]>([]);
  let detectedPackageManagers = $state<PackageManager[]>([]);
  let newInitCommandName = $state('');
  let newInitCommandValue = $state('');

  // Progress task list state
  type TaskStatus = 'pending' | 'running' | 'completed' | 'failed';

  interface ProgressTask {
    id: string;
    name: string;
    status: TaskStatus;
    detail?: string;
  }

  let progressTasks = $state<ProgressTask[]>([]);
  let isProgressActive = $state(false);
  let creationCancelled = $state(false);
  let openCancelled = $state(false);

  const showProgress = $derived(isProgressActive && progressTasks.length > 0);

  // Port isolation state
  let portConfig = $state<PortConfig | null>(null);
  let detectedPorts = $state<DetectedPorts | null>(null);
  let isDetectingPorts = $state(false);
  let selectedPorts = $state<Map<string, boolean>>(new Map());
  let portAssignments = $state<PortAssignment[]>([]);
  let newTargetFile = $state('');

  // Compose isolation state
  let composeConfig = $state<ComposeIsolationConfig | null>(null);
  let detectedComposeFiles = $state<DetectedComposeFiles | null>(null);
  let isDetectingCompose = $state(false);
  let composeReplacements = $state<ComposeNameReplacement[]>([]);

  // Helper to force UI update - uses setTimeout to ensure render cycle completes
  async function forceUIUpdate(delayMs = 50): Promise<void> {
    await tick();
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }

  // Pause between task completions so the user can see each checkmark
  const TASK_STEP_PAUSE_MS = 300;
  async function pauseBetweenTasks(): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, TASK_STEP_PAUSE_MS));
  }

  // Event listener cleanup
  let unlistenWorktreeRemoved: (() => void) | null = null;

  const worktrees = $derived($worktreeStore.worktrees);

  // Check if current window is a worktree
  const isCurrentWindowWorktree = $derived(() => currentContext?.is_worktree ?? false);

  // Get the main worktree
  const mainWorktree = $derived(() => worktrees.find((w) => w.is_main));

  // Get linked worktrees
  const linkedWorktrees = $derived(() => worktrees.filter((w) => !w.is_main && w.is_valid));

  // Get current branch name (HEAD)
  const currentBranch = $derived(() => {
    const headBranch = branches.find((b) => b.is_head);
    return headBranch?.name ?? null;
  });

  // Get branches that are already used by worktrees
  const usedBranches = $derived(() => {
    return new Set(worktrees.filter((w) => !w.is_main && w.branch).map((w) => w.branch!));
  });

  // Validate branch selection
  const branchValidationError = $derived(() => {
    const branchName = createName.trim();
    if (!branchName) return null;

    const current = currentBranch();
    if (current && branchName === current) {
      return `Branch '${branchName}' is currently checked out. Cannot create a worktree for the current branch.`;
    }

    const used = usedBranches();
    if (used.has(branchName)) {
      const wt = worktrees.find((w) => w.branch === branchName && !w.is_main);
      return `Branch '${branchName}' is already checked out in worktree '${wt?.name ?? 'unknown'}'.`;
    }

    return null;
  });

  // Filter branches for dropdown (exclude current and in-use)
  const availableBranches = $derived(() => {
    const current = currentBranch();
    const used = usedBranches();
    return branches.filter((b) => !b.is_head && b.name !== current && !used.has(b.name));
  });

  // Compute worktree name (with '/' replaced by '-')
  const worktreeName = $derived(() => {
    const name = createName.trim();
    if (!name) return '';
    return branchToWorktreeName(name);
  });

  // Rebuild compose replacements when worktree name changes
  $effect(() => {
    void worktreeName();
    if (detectedComposeFiles && composeConfig?.enabled) {
      rebuildComposeReplacements();
    }
  });

  // Compute worktree path preview
  const pathPreview = $derived(() => {
    const wtName = worktreeName();
    if (!wtName || !projectPath) return '';
    const parts = projectPath.split('/');
    const repoName = parts[parts.length - 1] || parts[parts.length - 2] || 'repo';
    const parentPath = parts.slice(0, -1).join('/');
    return `${parentPath}/${repoName}-${wtName}`;
  });

  onMount(async () => {
    mounted = true;
    document.addEventListener('keydown', handleKeyDown, true);
    document.addEventListener('click', handleDocumentClick, true);

    // Listen for worktree-removed event to refresh the list
    unlistenWorktreeRemoved = await eventService.listen<{ path: string }>(
      'worktree-removed',
      () => {
        loadWorktrees().catch(console.error);
        loadBranches().catch(console.error);
      }
    );

    await loadWorktrees();
    await loadBranches();
    await loadContext();
    await loadCopySettings();
    await loadInitCommands();
    await loadPortConfig();
    await loadComposeConfig();
    await detectPackageManagers();
    await detectPortsForWorktree();
    await detectComposeFilesForWorktree();

    isInitializing = false;
  });

  async function loadContext() {
    try {
      currentContext = await worktreeService.getContext(projectPath);
    } catch {
      currentContext = null;
    }
  }

  async function loadCopySettings() {
    try {
      const settings = await loadProjectSettings(projectPath);
      userCopyPatterns = settings.worktreeCopyPatterns;
    } catch {
      userCopyPatterns = [];
    }
  }

  async function saveCopySettings() {
    try {
      const settings = await loadProjectSettings(projectPath);
      settings.worktreeCopyPatterns = userCopyPatterns;
      await saveProjectSettings(projectPath, settings);
    } catch (e) {
      console.error('Failed to save copy settings:', e);
    }
  }

  function addCopyPattern() {
    const pattern = newCopyPattern.trim();
    if (!pattern) return;
    if (userCopyPatterns.includes(pattern)) {
      newCopyPattern = '';
      return;
    }
    if (DEFAULT_WORKTREE_COPY_PATTERNS.includes(pattern)) {
      newCopyPattern = '';
      return;
    }
    userCopyPatterns = [...userCopyPatterns, pattern];
    newCopyPattern = '';
    saveCopySettings();
  }

  function removeCopyPattern(pattern: string) {
    userCopyPatterns = userCopyPatterns.filter((p) => p !== pattern);
    saveCopySettings();
  }

  async function loadInitCommands() {
    try {
      const settings = await loadProjectSettings(projectPath);
      initCommands = settings.worktreeInitCommands;
    } catch {
      initCommands = [];
    }
  }

  async function saveInitCommands() {
    try {
      const settings = await loadProjectSettings(projectPath);
      settings.worktreeInitCommands = initCommands;
      await saveProjectSettings(projectPath, settings);
    } catch (e) {
      console.error('Failed to save init commands:', e);
    }
  }

  async function detectPackageManagers() {
    try {
      detectedPackageManagers = await worktreeService.detectPackageManagers(projectPath);
    } catch {
      detectedPackageManagers = [];
    }
  }

  async function loadPortConfig() {
    try {
      const settings = await loadProjectSettings(projectPath);
      if (settings.portConfig) {
        // Enforce 100-port block rule: recalculate portRangeEnd from portRangeStart
        const start = settings.portConfig.portRangeStart ?? DEFAULT_PORT_RANGE_START;
        portConfig = {
          ...settings.portConfig,
          portRangeStart: start,
          portRangeEnd: start + DEFAULT_PORT_BLOCK_SIZE - 1,
        };
      } else {
        portConfig = {
          enabled: true,
          portRangeStart: DEFAULT_PORT_RANGE_START,
          portRangeEnd: DEFAULT_PORT_RANGE_END,
          worktreeAssignments: {},
          targetFiles: [...DEFAULT_TARGET_FILES],
        };
      }
    } catch {
      portConfig = null;
    }
  }

  async function savePortConfig() {
    if (!portConfig) return;
    try {
      const settings = await loadProjectSettings(projectPath);
      settings.portConfig = portConfig;
      await saveProjectSettings(projectPath, settings);
    } catch (e) {
      console.error('Failed to save port config:', e);
    }
  }

  async function detectPortsForWorktree(): Promise<void> {
    if (!portConfig?.enabled) {
      detectedPorts = null;
      return;
    }
    isDetectingPorts = true;
    try {
      const detected = await portIsolationService.detectPorts(projectPath);
      if (portIsolationService.hasDetectablePorts(detected)) {
        detectedPorts = detected;
        // Initialize selected ports (all checked by default)
        const uniquePorts = portIsolationService.getAllUniquePorts(detected);
        // eslint-disable-next-line svelte/prefer-svelte-reactivity -- intentionally using Map
        const newSelected = new Map<string, boolean>();
        for (const port of uniquePorts) {
          newSelected.set(port.variable_name, true);
        }
        selectedPorts = newSelected;
        // Allocate ports
        allocatePortsForWorktree();
      } else {
        detectedPorts = null;
      }
    } catch (e) {
      console.error('Failed to detect ports:', e);
      detectedPorts = null;
    } finally {
      isDetectingPorts = false;
    }
  }

  /**
   * Get enabled target file patterns (excluding disabled ones)
   */
  function getEnabledTargetPatterns(): string[] {
    if (!portConfig) return [];
    const disabled = portConfig.disabledTargetFiles ?? [];
    return (portConfig.targetFiles ?? DEFAULT_TARGET_FILES).filter((f) => !disabled.includes(f));
  }

  /**
   * Check if a port variable is transformable (has source files matching enabled target patterns)
   */
  function isPortTransformable(variableName: string): boolean {
    if (!detectedPorts || !portConfig) return true;
    const allSources = [...detectedPorts.env_ports, ...detectedPorts.compose_ports];
    return portIsolationService.isPortTransformable(
      variableName,
      allSources,
      projectPath,
      getEnabledTargetPatterns()
    );
  }

  function allocatePortsForWorktree(): void {
    if (!detectedPorts || !portConfig) return;

    const selectedVars = new Set(
      Array.from(selectedPorts.entries())
        .filter(([, selected]) => selected)
        .map(([name]) => name)
    );

    const uniquePorts = portIsolationService.getAllUniquePorts(detectedPorts);
    const portsToAllocate = uniquePorts.filter(
      (p) => selectedVars.has(p.variable_name) && isPortTransformable(p.variable_name)
    );

    if (portsToAllocate.length === 0) {
      portAssignments = [];
      return;
    }

    // Allocate ports avoiding those already used by other worktrees
    portAssignments = portIsolationService.allocatePortsAvoidingUsed(portsToAllocate, portConfig);
  }

  function togglePortSelection(variableName: string) {
    const current = selectedPorts.get(variableName) ?? true;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- intentionally using Map
    selectedPorts = new Map(selectedPorts).set(variableName, !current);
    allocatePortsForWorktree();
  }

  function getAssignedPortValue(variableName: string): number | null {
    const assignment = portAssignments.find((a) => a.variable_name === variableName);
    return assignment?.assigned_value ?? null;
  }

  function getPortSourceFiles(variableName: string): string[] {
    if (!detectedPorts) return [];
    const allPorts = [...detectedPorts.env_ports, ...detectedPorts.compose_ports];
    const prefix = projectPath.endsWith('/') ? projectPath : `${projectPath}/`;
    return allPorts
      .filter((p) => p.variable_name === variableName)
      .map((p) => {
        const rel = p.file_path.startsWith(prefix) ? p.file_path.slice(prefix.length) : p.file_path;
        return `${rel}:${p.line_number}`;
      });
  }

  // Get unique ports for display (env + compose)
  function getUniquePorts(): PortSource[] {
    if (!detectedPorts) return [];
    return portIsolationService.getAllUniquePorts(detectedPorts);
  }

  // Count of selected AND transformable ports
  function getSelectedPortCount(): number {
    return Array.from(selectedPorts.entries()).filter(
      ([name, selected]) => selected && isPortTransformable(name)
    ).length;
  }

  // Counts for execution summary
  function getCopyFileCount(): number {
    // Default patterns + user patterns + enabled target files (for port isolation)
    const regularPatterns = DEFAULT_WORKTREE_COPY_PATTERNS.length + userCopyPatterns.length;
    const disabledFiles = portConfig?.disabledTargetFiles ?? [];
    const enabledTargetFiles = (portConfig?.targetFiles ?? DEFAULT_TARGET_FILES).filter(
      (f) => !disabledFiles.includes(f)
    );
    return regularPatterns + enabledTargetFiles.length;
  }

  function getEnabledCommandCount(): number {
    let count = 0;
    // Check auto-detected package managers
    for (const pm of detectedPackageManagers) {
      const autoOverride = initCommands.find((c) => c.command === pm.command && c.auto);
      if (autoOverride) {
        if (autoOverride.enabled) count++;
      } else {
        const hasUserOverride = initCommands.some((c) => c.command === pm.command);
        if (!hasUserOverride) count++;
      }
    }
    // Count enabled user commands (exclude auto overrides, already counted above)
    count += initCommands.filter((c) => c.enabled && !c.auto).length;
    return count;
  }

  function getPortCount(): number {
    if (!portConfig?.enabled) return 0;
    return portAssignments.length;
  }

  // Tooltip descriptions for execution summary
  function getCopyFilesTooltip(): string {
    const patterns: string[] = [];
    // Default patterns
    patterns.push(...DEFAULT_WORKTREE_COPY_PATTERNS);
    // User patterns
    patterns.push(...userCopyPatterns);
    // Enabled target files
    const disabledFiles = portConfig?.disabledTargetFiles ?? [];
    const enabledTargetFiles = (portConfig?.targetFiles ?? DEFAULT_TARGET_FILES).filter(
      (f) => !disabledFiles.includes(f)
    );
    patterns.push(...enabledTargetFiles);

    if (patterns.length === 0) return 'No files to copy';
    return `Copy to worktree:\n${patterns.map((p) => `  ${p}`).join('\n')}`;
  }

  function getCommandsTooltip(): string {
    const commands: string[] = [];
    // Auto-detected package managers
    for (const pm of detectedPackageManagers) {
      const autoOverride = initCommands.find((c) => c.command === pm.command && c.auto);
      if (autoOverride) {
        if (autoOverride.enabled) commands.push(pm.command);
      } else {
        const hasUserOverride = initCommands.some((c) => c.command === pm.command);
        if (!hasUserOverride) commands.push(pm.command);
      }
    }
    // User commands (exclude auto overrides, already handled above)
    for (const cmd of initCommands.filter((c) => c.enabled && !c.auto)) {
      commands.push(cmd.command);
    }

    if (commands.length === 0) return 'No commands to run';
    return `Run after creation:\n${commands.map((c) => `  ${c}`).join('\n')}`;
  }

  function getPortsTooltip(): string {
    if (portAssignments.length === 0) return 'No ports to isolate';
    const lines = portAssignments.map(
      (a) => `  ${a.variable_name}: ${a.original_value} → ${a.assigned_value}`
    );
    return `Port isolation:\n${lines.join('\n')}`;
  }

  function togglePortIsolation() {
    if (portConfig) {
      portConfig = { ...portConfig, enabled: !portConfig.enabled };
      savePortConfig();
      if (portConfig.enabled) {
        detectPortsForWorktree();
      } else {
        detectedPorts = null;
        portAssignments = [];
      }
    }
  }

  function updateProjectBasePort(newBase: number) {
    if (!portConfig) return;
    // Ensure base port is within valid range and is a multiple of 100
    const validBase = Math.max(1024, Math.floor(newBase / 100) * 100);
    portConfig = {
      ...portConfig,
      portRangeStart: validBase,
      portRangeEnd: validBase + DEFAULT_PORT_BLOCK_SIZE - 1,
    };
    savePortConfig();
    // Re-allocate ports if there are detected ports
    if (detectedPorts) {
      allocatePortsForWorktree();
    }
  }

  function addTargetFile() {
    const pattern = newTargetFile.trim();
    if (!pattern || !portConfig) return;
    // Prevent duplicates
    if (portConfig.targetFiles?.includes(pattern)) {
      newTargetFile = '';
      return;
    }
    portConfig = {
      ...portConfig,
      targetFiles: [...(portConfig.targetFiles ?? DEFAULT_TARGET_FILES), pattern],
    };
    newTargetFile = '';
    savePortConfig();
  }

  function removeTargetFile(pattern: string) {
    if (!portConfig) return;
    // Prevent removing the default .env* pattern
    if (pattern === '.env*') return;
    portConfig = {
      ...portConfig,
      targetFiles: (portConfig.targetFiles ?? []).filter((p) => p !== pattern),
    };
    savePortConfig();
  }

  function isDefaultTargetFile(pattern: string): boolean {
    return DEFAULT_TARGET_FILES.includes(pattern);
  }

  function isTargetFileEnabled(pattern: string): boolean {
    if (!portConfig) return true;
    return !(portConfig.disabledTargetFiles ?? []).includes(pattern);
  }

  function toggleTargetFile(pattern: string) {
    if (!portConfig) return;
    const disabled = portConfig.disabledTargetFiles ?? [];
    if (disabled.includes(pattern)) {
      // Enable it
      portConfig = {
        ...portConfig,
        disabledTargetFiles: disabled.filter((p) => p !== pattern),
      };
    } else {
      // Disable it
      portConfig = {
        ...portConfig,
        disabledTargetFiles: [...disabled, pattern],
      };
    }
    savePortConfig();
    // Re-allocate ports since transformable set may have changed
    allocatePortsForWorktree();
  }

  // ============================================================================
  // Compose isolation functions
  // ============================================================================

  async function loadComposeConfig() {
    try {
      const settings = await loadProjectSettings(projectPath);
      composeConfig = settings.composeIsolationConfig ?? {
        enabled: true,
        disabledFiles: [],
      };
    } catch {
      composeConfig = { enabled: true, disabledFiles: [] };
    }
  }

  async function saveComposeConfig() {
    if (!composeConfig) return;
    try {
      const settings = await loadProjectSettings(projectPath);
      settings.composeIsolationConfig = composeConfig;
      await saveProjectSettings(projectPath, settings);
    } catch (e) {
      console.error('Failed to save compose config:', e);
    }
  }

  async function detectComposeFilesForWorktree(): Promise<void> {
    if (!composeConfig?.enabled) {
      detectedComposeFiles = null;
      composeReplacements = [];
      return;
    }
    isDetectingCompose = true;
    try {
      const detected = await composeIsolationService.detectComposeFiles(projectPath);
      detectedComposeFiles = detected;
      rebuildComposeReplacements();
    } catch (e) {
      console.error('Failed to detect compose files:', e);
      detectedComposeFiles = null;
      composeReplacements = [];
    } finally {
      isDetectingCompose = false;
    }
  }

  function rebuildComposeReplacements() {
    if (!detectedComposeFiles || !composeConfig?.enabled) {
      composeReplacements = [];
      return;
    }
    const wtName = worktreeName();
    if (!wtName) {
      composeReplacements = [];
      return;
    }
    composeReplacements = composeIsolationService.buildReplacements(
      detectedComposeFiles,
      wtName,
      composeConfig.disabledFiles ?? []
    );
  }

  function toggleComposeIsolation() {
    if (composeConfig) {
      composeConfig = { ...composeConfig, enabled: !composeConfig.enabled };
      saveComposeConfig();
      if (composeConfig.enabled) {
        detectComposeFilesForWorktree();
      } else {
        detectedComposeFiles = null;
        composeReplacements = [];
      }
    }
  }

  function toggleComposeFile(filePath: string) {
    if (!composeConfig) return;
    const disabled = composeConfig.disabledFiles ?? [];
    if (disabled.includes(filePath)) {
      composeConfig = {
        ...composeConfig,
        disabledFiles: disabled.filter((f) => f !== filePath),
      };
    } else {
      composeConfig = {
        ...composeConfig,
        disabledFiles: [...disabled, filePath],
      };
    }
    saveComposeConfig();
    rebuildComposeReplacements();
  }

  function getComposeCount(): number {
    if (!composeConfig?.enabled || !detectedComposeFiles) return 0;
    const disabled = composeConfig.disabledFiles ?? [];
    return detectedComposeFiles.files.filter(
      (f) => f.project_name !== null && !disabled.includes(f.file_path)
    ).length;
  }

  function getComposeTooltip(): string {
    if (!detectedComposeFiles || getComposeCount() === 0) return 'No compose files to isolate';
    const disabled = composeConfig?.disabledFiles ?? [];
    const lines = detectedComposeFiles.files
      .filter((f) => f.project_name !== null && !disabled.includes(f.file_path))
      .map((f) => `  ${f.file_path}: ${f.project_name}`);
    return `Compose isolation:\n${lines.join('\n')}`;
  }

  function getComposeWarningCount(): number {
    if (!detectedComposeFiles) return 0;
    return detectedComposeFiles.files.reduce((sum, f) => sum + f.warnings.length, 0);
  }

  function openComposeSettings() {
    showCopySettingsModal = true;
    // Scroll to compose isolation section after modal renders
    requestAnimationFrame(() => {
      const section = document.getElementById('compose-isolation-section');
      if (section) {
        section.scrollIntoView({ behavior: 'smooth', block: 'start' });
      }
    });
  }

  function addInitCommand() {
    const name = newInitCommandName.trim();
    const command = newInitCommandValue.trim();
    if (!name || !command) return;

    // Check for duplicates
    if (initCommands.some((c) => c.command === command)) {
      newInitCommandName = '';
      newInitCommandValue = '';
      return;
    }

    initCommands = [
      ...initCommands,
      {
        name,
        command,
        enabled: true,
        auto: false,
      },
    ];
    newInitCommandName = '';
    newInitCommandValue = '';
    saveInitCommands();
  }

  function removeInitCommand(command: string) {
    initCommands = initCommands.filter((c) => c.command !== command);
    saveInitCommands();
  }

  function toggleInitCommand(command: string) {
    initCommands = initCommands.map((c) =>
      c.command === command ? { ...c, enabled: !c.enabled } : c
    );
    saveInitCommands();
  }

  function toggleAutoCommand(pm: PackageManager) {
    const existing = initCommands.find((c) => c.command === pm.command && c.auto);
    if (existing) {
      // Already overridden: toggle enabled state
      initCommands = initCommands.map((c) =>
        c.command === pm.command && c.auto ? { ...c, enabled: !c.enabled } : c
      );
    } else {
      // First disable: add as disabled auto command
      const cdMatch = pm.command.match(/^cd (.+?) && /);
      const label = cdMatch ? `${pm.name} in ${cdMatch[1]}` : pm.name;
      initCommands = [
        ...initCommands,
        {
          name: `Install dependencies (${label})`,
          command: pm.command,
          enabled: false,
          auto: true,
        },
      ];
    }
    saveInitCommands();
  }

  function getEffectiveInitCommands(): WorktreeInitCommand[] {
    // Combine auto-detected and user-configured commands
    const commands: WorktreeInitCommand[] = [];

    // Add detected package managers if not already configured
    for (const pm of detectedPackageManagers) {
      const hasPackageManagerCommand = initCommands.some((c) => c.command === pm.command);
      if (!hasPackageManagerCommand) {
        // Extract subdirectory from "cd subdir && " prefix if present
        const cdMatch = pm.command.match(/^cd (.+?) && /);
        const label = cdMatch ? `${pm.name} in ${cdMatch[1]}` : pm.name;
        commands.push({
          name: `Install dependencies (${label})`,
          command: pm.command,
          enabled: true,
          auto: true,
        });
      }
    }

    // Add user-configured commands
    commands.push(...initCommands);

    return commands;
  }

  /** Split "Install dependencies (npm)" → { baseName: "Install dependencies", suffix: "npm" } */
  function splitCommandName(name: string): { baseName: string; suffix?: string } {
    const match = name.match(/^(.+?)\s*\((.+)\)$/);
    if (match) {
      return { baseName: match[1].trim(), suffix: match[2] };
    }
    return { baseName: name };
  }

  function buildCreationTaskList(
    branchName: string,
    effectiveCommands: WorktreeInitCommand[],
    hasPortAssignments: boolean,
    hasComposeReplacements: boolean
  ): ProgressTask[] {
    const tasks: ProgressTask[] = [
      {
        id: 'worktree',
        name: `Create worktree '${branchToWorktreeName(branchName)}'`,
        status: 'pending',
      },
      {
        id: 'copy',
        name: 'Copy files',
        status: 'pending',
      },
    ];

    if (hasPortAssignments) {
      tasks.push({
        id: 'port-remap',
        name: 'Remap ports',
        status: 'pending',
      });
    }

    if (hasComposeReplacements) {
      tasks.push({
        id: 'compose-name',
        name: 'Isolate compose names',
        status: 'pending',
      });
    }

    effectiveCommands.forEach((cmd, index) => {
      const { baseName, suffix } = splitCommandName(cmd.name);
      tasks.push({
        id: `init-${index}`,
        name: baseName,
        status: 'pending',
        detail: suffix,
      });
    });

    tasks.push({
      id: 'open-window',
      name: 'Open worktree window',
      status: 'pending',
    });

    return tasks;
  }

  function buildOpenTaskList(effectiveCommands: WorktreeInitCommand[]): ProgressTask[] {
    const tasks: ProgressTask[] = [];

    effectiveCommands.forEach((cmd, index) => {
      const { baseName, suffix } = splitCommandName(cmd.name);
      tasks.push({
        id: `init-${index}`,
        name: baseName,
        status: 'pending',
        detail: suffix,
      });
    });

    tasks.push({
      id: 'open-window',
      name: 'Open worktree window',
      status: 'pending',
    });

    return tasks;
  }

  function updateTask(taskId: string, status: TaskStatus, detail?: string) {
    progressTasks = progressTasks.map((task) =>
      task.id === taskId ? { ...task, status, ...(detail !== undefined ? { detail } : {}) } : task
    );
  }

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
    document.removeEventListener('click', handleDocumentClick, true);
    if (unlistenWorktreeRemoved) {
      unlistenWorktreeRemoved();
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      if (showBranchDropdown) {
        showBranchDropdown = false;
      } else if (showCopySettingsModal) {
        showCopySettingsModal = false;
      } else {
        onClose();
      }
    }
  }

  function handleDocumentClick(_e: MouseEvent) {
    // No longer needed for dropdown - using modal now
  }

  async function loadWorktrees(path?: string) {
    const targetPath = path ?? projectPath;
    if (!targetPath) return;
    await worktreeStore.refresh(targetPath);
  }

  async function loadBranches() {
    try {
      branches = await worktreeService.listBranches(projectPath);
    } catch {
      branches = [];
    }
  }

  function handleSelectBranch(branchName: string) {
    createName = branchName;
    isExistingBranch = true;
    showBranchDropdown = false;
  }

  function handleNameInput() {
    // When user types, treat as new branch
    isExistingBranch = false;
  }

  async function handleCreate() {
    if (!createName.trim()) return;
    if (!projectPath) {
      createError = 'Project path is not available';
      return;
    }

    // Proceed with worktree creation using pre-configured port assignments
    continueWorktreeCreation();
  }

  async function continueWorktreeCreation() {
    isCreating = true;
    createError = null;
    creationCancelled = false;

    const branchName = createName.trim();
    const effectiveCommands = getEffectiveInitCommands().filter((c) => c.enabled);

    // Build and display the full task list immediately
    const currentPortAssignments = portConfig?.enabled ? portAssignments : [];
    const hasPortAssignments = currentPortAssignments.length > 0;
    const currentComposeReplacements = composeConfig?.enabled ? composeReplacements : [];
    const hasComposeReplacements = currentComposeReplacements.length > 0;
    progressTasks = buildCreationTaskList(
      branchName,
      effectiveCommands,
      hasPortAssignments,
      hasComposeReplacements
    );
    isProgressActive = true;
    await forceUIUpdate();

    // Capture projectPath in a local variable to avoid closure issues
    const currentProjectPath = projectPath;

    try {
      const wtName = branchToWorktreeName(branchName);

      // Step 1: Create worktree
      updateTask('worktree', 'running');
      await forceUIUpdate();
      const wt = await worktreeService.create(
        currentProjectPath,
        wtName,
        branchName,
        !isExistingBranch
      );

      if (creationCancelled) {
        await worktreeService.remove(currentProjectPath, wtName);
        resetCreation();
        return;
      }

      updateTask('worktree', 'completed');
      await forceUIUpdate();
      await pauseBetweenTasks();

      // Step 2: Copy files
      updateTask('copy', 'running');
      await forceUIUpdate();
      const regularCopyPatterns = [...DEFAULT_WORKTREE_COPY_PATTERNS, ...userCopyPatterns];
      const disabledFiles = portConfig?.disabledTargetFiles ?? [];
      const enabledTargetFiles = (portConfig?.targetFiles ?? DEFAULT_TARGET_FILES).filter(
        (f) => !disabledFiles.includes(f)
      );
      const allPatterns = [...new Set([...regularCopyPatterns, ...enabledTargetFiles])];
      if (allPatterns.length > 0) {
        try {
          let copyResult;
          if (hasPortAssignments) {
            copyResult = await portIsolationService.copyFilesWithPorts(
              currentProjectPath,
              wt.path,
              allPatterns,
              currentPortAssignments
            );
          } else {
            copyResult = await worktreeService.copyFiles(currentProjectPath, wt.path, allPatterns);
          }
          const detail =
            copyResult.copied_files.length > 0
              ? `${copyResult.copied_files.length} files copied`
              : 'No files to copy';
          updateTask('copy', 'completed', detail);
          if (copyResult.errors.length > 0) {
            console.error('Copy errors:', copyResult.errors);
          }
        } catch (copyError) {
          console.error('Failed to copy files:', copyError);
          updateTask('copy', 'failed', 'Copy failed');
          if (hasPortAssignments) {
            updateTask('port-remap', 'failed', 'Skipped due to copy failure');
          }
        }
      } else {
        updateTask('copy', 'completed', 'No copy patterns configured');
      }
      await forceUIUpdate();
      await pauseBetweenTasks();

      // Step 2.5: Remap ports (register assignments)
      if (hasPortAssignments) {
        updateTask('port-remap', 'running');
        await forceUIUpdate();
        try {
          if (portConfig) {
            portConfig = portIsolationService.registerWorktreeAssignments(
              portConfig,
              wtName,
              currentPortAssignments
            );
            await savePortConfig();
          }
          updateTask(
            'port-remap',
            'completed',
            `${currentPortAssignments.length} variables remapped`
          );
        } catch (portError) {
          console.error('Failed to register port assignments:', portError);
          updateTask('port-remap', 'failed', String(portError));
        }
        await forceUIUpdate();
        await pauseBetweenTasks();
      }

      // Step 2.7: Compose name isolation
      if (hasComposeReplacements) {
        updateTask('compose-name', 'running');
        await forceUIUpdate();
        try {
          const composeResult = await composeIsolationService.applyComposeIsolation(
            wt.path,
            currentComposeReplacements
          );
          updateTask(
            'compose-name',
            'completed',
            `${composeResult.transformed_files.length} files updated`
          );
          if (composeResult.errors.length > 0) {
            console.error('Compose isolation errors:', composeResult.errors);
          }
        } catch (composeError) {
          console.error('Failed to isolate compose names:', composeError);
          updateTask('compose-name', 'failed', String(composeError));
        }
        await forceUIUpdate();
        await pauseBetweenTasks();
      }

      if (creationCancelled) {
        await worktreeService.remove(currentProjectPath, wtName);
        resetCreation();
        return;
      }

      // Step 3: Run initialization commands
      for (let i = 0; i < effectiveCommands.length; i++) {
        const cmd = effectiveCommands[i];
        if (creationCancelled) {
          await worktreeService.remove(currentProjectPath, wtName);
          resetCreation();
          return;
        }

        updateTask(`init-${i}`, 'running');
        await forceUIUpdate();
        try {
          const result = await worktreeService.runInitCommand(wt.path, cmd.command);
          if (result.success) {
            updateTask(`init-${i}`, 'completed');
          } else {
            const errorDetail = result.stderr
              ? result.stderr.trim().split('\n').slice(-1)[0]
              : `exit code: ${result.exit_code}`;
            updateTask(`init-${i}`, 'failed', errorDetail);
          }
        } catch (cmdError) {
          updateTask(`init-${i}`, 'failed', String(cmdError));
        }
        await forceUIUpdate();
        await pauseBetweenTasks();
      }

      if (creationCancelled) {
        await worktreeService.remove(currentProjectPath, wtName);
        resetCreation();
        return;
      }

      // Step 4: Done - open window
      updateTask('open-window', 'running');
      await forceUIUpdate();

      openWorktreeWindow(wt, true);
      loadWorktrees(currentProjectPath).catch(console.error);

      updateTask('open-window', 'completed');
      await forceUIUpdate();

      // Brief delay to show all-complete state, then reset
      await new Promise((resolve) => setTimeout(resolve, 800));
      resetCreation();
      createName = '';
      isExistingBranch = false;
      await detectPortsForWorktree();
      await forceUIUpdate();
    } catch (e) {
      createError = e instanceof Error ? e.message : String(e);
      resetCreation();
      await forceUIUpdate();
    }
  }

  function resetCreation() {
    isCreating = false;
    isProgressActive = false;
    progressTasks = [];
    creationCancelled = false;
  }

  function resetOpen() {
    isProgressActive = false;
    progressTasks = [];
    openCancelled = false;
  }

  async function openWorktreeWindow(wt: WorktreeInfo, skipInit = false) {
    // If called from handleCreate (skipInit=true), just open the window
    if (skipInit) {
      try {
        await windowService.focusOrCreateWindow(wt.path);
      } catch (e) {
        console.error('Failed to open worktree window:', e);
      }
      return;
    }

    // For clicking on existing worktrees, run init commands first
    openCancelled = false;
    const effectiveCommands = getEffectiveInitCommands().filter((c) => c.enabled);

    progressTasks = buildOpenTaskList(effectiveCommands);
    isProgressActive = true;
    await forceUIUpdate();

    try {
      for (let i = 0; i < effectiveCommands.length; i++) {
        const cmd = effectiveCommands[i];
        if (openCancelled) {
          resetOpen();
          return;
        }

        updateTask(`init-${i}`, 'running');
        await forceUIUpdate();
        try {
          const result = await worktreeService.runInitCommand(wt.path, cmd.command);
          if (result.success) {
            updateTask(`init-${i}`, 'completed');
          } else {
            const errorDetail = result.stderr
              ? result.stderr.trim().split('\n').slice(-1)[0]
              : `exit code: ${result.exit_code}`;
            updateTask(`init-${i}`, 'failed', errorDetail);
          }
        } catch (cmdError) {
          updateTask(`init-${i}`, 'failed', String(cmdError));
        }
        await forceUIUpdate();
        await pauseBetweenTasks();
      }

      if (openCancelled) {
        resetOpen();
        return;
      }

      // Done - open window
      updateTask('open-window', 'running');
      await forceUIUpdate();

      try {
        await windowService.focusOrCreateWindow(wt.path);
      } catch (e) {
        console.error('Failed to open worktree window:', e);
      }

      updateTask('open-window', 'completed');
      await forceUIUpdate();

      // Brief delay to show all-complete state, then reset
      await new Promise((resolve) => setTimeout(resolve, 800));
      resetOpen();
    } catch (e) {
      console.error('Failed to open worktree:', e);
      resetOpen();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="worktree-backdrop" class:mounted onclick={handleBackdropClick}>
  <div class="worktree-modal">
    <div class="modal-glow"></div>
    <div class="modal-content">
      <div class="modal-header">
        <div class="header-content">
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <line x1="6" y1="3" x2="6" y2="15"></line>
            <circle cx="18" cy="6" r="3"></circle>
            <circle cx="6" cy="18" r="3"></circle>
            <path d="M18 9a9 9 0 0 1-9 9"></path>
          </svg>
          <span class="title">Worktrees</span>
        </div>
        <div class="header-actions">
          <button
            class="action-btn settings-btn"
            onclick={() => (showCopySettingsModal = true)}
            title="Settings"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <circle cx="12" cy="12" r="3"></circle>
              <path
                d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
              ></path>
            </svg>
          </button>
          <button class="action-btn close-btn" onclick={() => onClose()} title="Close (Esc)">
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
      </div>

      {#if getComposeWarningCount() > 0}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div class="compose-conflict-banner" onclick={() => openComposeSettings()}>
          <span class="compose-conflict-icon">⚠</span>
          <span class="compose-conflict-text"
            >{getComposeWarningCount()}
            {getComposeWarningCount() === 1 ? 'conflict' : 'conflicts'} detected</span
          >
          <span class="compose-conflict-link">Settings →</span>
        </div>
      {/if}

      <div class="modal-body">
        {#if isInitializing}
          <div class="init-overlay">
            <Spinner size="md" />
            <span>Scanning project...</span>
          </div>
        {/if}
        <!-- Worktree Tree View -->
        <div class="worktree-tree">
          <!-- Main repository (parent) -->
          {#if mainWorktree()}
            {@const main = mainWorktree()}
            <div class="tree-item tree-parent" class:is-current={!isCurrentWindowWorktree()}>
              <span class="tree-indicator"></span>
              <span class="tree-branch">{main?.branch ?? 'detached'}</span>
              {#if !isCurrentWindowWorktree()}
                <span class="tree-label label-current">CURRENT</span>
              {/if}
            </div>
          {/if}

          <!-- Linked worktrees (children) -->
          {#each linkedWorktrees() as wt, i (wt.path)}
            {@const isLast = i === linkedWorktrees().length - 1}
            <button
              type="button"
              class="tree-item tree-child"
              onclick={() => openWorktreeWindow(wt)}
              title="Click to open"
            >
              <span class="tree-connector" class:is-last={isLast}></span>
              <span class="tree-indicator"></span>
              <span class="tree-branch">{wt.branch ?? 'detached'}</span>
              {#if wt.is_locked}
                <span class="tree-locked" title="Locked">🔒</span>
              {/if}
              <span class="tree-label label-wt">WT</span>
            </button>
          {/each}
        </div>

        <!-- Separator -->
        <div class="section-divider"></div>

        <!-- Create Form / Progress View -->
        <div class="create-section">
          {#if showProgress}
            <!-- Task List Progress View -->
            <div class="progress-task-list">
              {#each progressTasks as task (task.id)}
                <div
                  class="progress-task-item"
                  class:is-pending={task.status === 'pending'}
                  class:is-running={task.status === 'running'}
                  class:is-completed={task.status === 'completed'}
                  class:is-failed={task.status === 'failed'}
                >
                  <div class="task-status-icon">
                    {#if task.status === 'pending'}
                      <!-- Mist particle dot -->
                      <span class="task-icon-dot"></span>
                    {:else if task.status === 'running'}
                      <Spinner size="sm" />
                    {:else if task.status === 'completed'}
                      <!-- Checkmark only -->
                      <svg
                        width="16"
                        height="16"
                        viewBox="0 0 16 16"
                        fill="none"
                        class="task-icon-completed"
                      >
                        <path
                          d="M4 8.5L6.5 11L12 5"
                          stroke="currentColor"
                          stroke-width="2"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                        />
                      </svg>
                    {:else if task.status === 'failed'}
                      <!-- Filled warning circle with exclamation -->
                      <svg
                        width="16"
                        height="16"
                        viewBox="0 0 16 16"
                        fill="none"
                        class="task-icon-failed"
                      >
                        <circle cx="8" cy="8" r="7.5" fill="currentColor" opacity="0.15" />
                        <line
                          x1="8"
                          y1="5"
                          x2="8"
                          y2="9"
                          stroke="currentColor"
                          stroke-width="1.8"
                          stroke-linecap="round"
                        />
                        <circle cx="8" cy="11.5" r="0.8" fill="currentColor" />
                      </svg>
                    {/if}
                  </div>
                  <div class="task-content">
                    <span class="task-name">{task.name}</span>
                    {#if task.detail}
                      <span class="task-detail">({task.detail})</span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <!-- Create Form -->
            <div class="section-title">New worktree</div>

            <div class="form-group">
              <div class="input-row">
                <div class="input-wrapper">
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    id="wt-name"
                    type="text"
                    class="form-input"
                    bind:value={createName}
                    placeholder="Branch name (e.g. fix-sidebar)"
                    spellcheck="false"
                    autocomplete="off"
                    autocorrect="off"
                    autocapitalize="off"
                    autofocus
                    oninput={() => handleNameInput()}
                    onkeydown={(e) => {
                      if (e.key === 'Enter' && createName.trim()) handleCreate();
                    }}
                  />
                  {#if createName.trim()}
                    <span
                      class="input-indicator"
                      title={isExistingBranch ? 'Existing branch' : 'New branch'}
                    >
                      {isExistingBranch ? 'E' : 'N'}
                    </span>
                  {/if}
                </div>
                <button
                  type="button"
                  class="branch-select-btn"
                  title="Select existing branch"
                  onclick={() => (showBranchDropdown = true)}
                  disabled={availableBranches().length === 0}
                >
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                  >
                    <line x1="6" y1="3" x2="6" y2="15"></line>
                    <circle cx="18" cy="6" r="3"></circle>
                    <circle cx="6" cy="18" r="3"></circle>
                    <path d="M18 9a9 9 0 0 1-9 9"></path>
                  </svg>
                </button>
              </div>
              {#if createName.trim()}
                <div class="path-preview">
                  <span class="preview-label">Path:</span>
                  <span class="preview-path">{pathPreview()}</span>
                </div>
              {/if}
            </div>

            {#if branchValidationError()}
              <div class="form-error form-warning">{branchValidationError()}</div>
            {/if}

            {#if createError}
              <div class="form-error">{createError}</div>
            {/if}

            <div class="form-actions">
              <button
                type="button"
                class="btn btn-primary"
                onclick={() => handleCreate()}
                disabled={!createName.trim() || isCreating || !!branchValidationError()}
              >
                {#if isCreating}
                  <Spinner size="sm" /> Creating...
                {:else}
                  Open
                {/if}
              </button>
            </div>
          {/if}
        </div>
      </div>

      <!-- Initialization summary (above footer) -->
      <div class="init-summary">
        <div class="init-header">
          <span class="init-label">Worktree initialization</span>
          <button
            type="button"
            class="init-settings-btn"
            onclick={() => (showCopySettingsModal = true)}
            title="Settings"
          >
            <svg
              width="12"
              height="12"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <circle cx="12" cy="12" r="3" />
              <path
                d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
              />
            </svg>
          </button>
        </div>
        <div class="init-content">
          <div class="init-stats">
            <span class="stat-item has-tooltip">
              <svg
                class="stat-icon stat-files"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <rect x="8" y="2" width="13" height="18" rx="2" />
                <path d="M16 2v4a2 2 0 0 0 2 2h4" />
                <path d="M5 10H3a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-2" />
              </svg>
              <span class="stat-text"
                >{getCopyFileCount()}
                {getCopyFileCount() === 1 ? 'file' : 'files'}
                <span class="stat-verb">copy</span></span
              >
              <span class="tooltip">{getCopyFilesTooltip()}</span>
            </span>
            <span class="stat-item has-tooltip">
              <svg
                class="stat-icon stat-commands"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <polyline points="4 17 10 11 4 5" />
                <line x1="12" y1="19" x2="20" y2="19" />
              </svg>
              <span class="stat-text"
                >{getEnabledCommandCount()}
                {getEnabledCommandCount() === 1 ? 'command' : 'commands'}
                <span class="stat-verb">run</span></span
              >
              <span class="tooltip">{getCommandsTooltip()}</span>
            </span>
            <span class="stat-item has-tooltip">
              <svg
                class="stat-icon stat-ports"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <circle cx="12" cy="12" r="3" />
                <path
                  d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83"
                />
              </svg>
              <span class="stat-text"
                >{getPortCount()}
                {getPortCount() === 1 ? 'port' : 'ports'}
                <span class="stat-verb">remap</span></span
              >
              <span class="tooltip">{getPortsTooltip()}</span>
            </span>
            <span class="stat-item has-tooltip">
              <svg
                class="stat-icon stat-compose"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path
                  d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
                />
                <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
                <line x1="12" y1="22.08" x2="12" y2="12" />
              </svg>
              <span class="stat-text"
                >{getComposeCount()}
                {getComposeCount() === 1 ? 'compose file' : 'compose files'}
                <span class="stat-verb">rename</span></span
              >
              <span class="tooltip">{getComposeTooltip()}</span>
            </span>
          </div>
        </div>
      </div>

      <!-- Row 2: Standard modal footer -->
      <div class="modal-footer">
        <span class="footer-item">
          <kbd>↵</kbd>
          <span>create</span>
        </span>
        <span class="footer-item">
          <kbd>Esc</kbd>
          <span>close</span>
        </span>
      </div>
    </div>
  </div>
</div>

<!-- Branch Selection Modal -->
{#if showBranchDropdown}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="branch-modal-backdrop" onclick={() => (showBranchDropdown = false)}>
    <div class="branch-modal" onclick={(e) => e.stopPropagation()}>
      <div class="branch-modal-header">
        <h3 class="branch-modal-title">Select Branch</h3>
        <button
          type="button"
          class="btn btn-ghost"
          onclick={() => (showBranchDropdown = false)}
          title="Close"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>
      <div class="branch-modal-body">
        {#each availableBranches() as b (b.name)}
          <button type="button" class="branch-item" onclick={() => handleSelectBranch(b.name)}>
            <svg
              class="branch-icon-small"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <line x1="6" y1="3" x2="6" y2="15"></line>
              <circle cx="18" cy="6" r="3"></circle>
              <circle cx="6" cy="18" r="3"></circle>
              <path d="M18 9a9 9 0 0 1-9 9"></path>
            </svg>
            <span class="branch-item-name">{b.name}</span>
            {#if b.last_commit_time}
              <span class="branch-item-time">{formatRelativeTime(b.last_commit_time)}</span>
            {/if}
          </button>
        {/each}
        {#if availableBranches().length === 0}
          <div class="empty-state">No available branches</div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<!-- Settings Modal -->
{#if showCopySettingsModal}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="settings-modal-backdrop" onclick={() => (showCopySettingsModal = false)}>
    <div class="settings-modal" onclick={(e) => e.stopPropagation()}>
      <div class="settings-modal-header">
        <h3 class="settings-modal-title">Settings</h3>
        <button
          type="button"
          class="btn btn-ghost"
          onclick={() => (showCopySettingsModal = false)}
          title="Close"
        >
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>
      <div class="settings-modal-body">
        <!-- Copy Files Section -->
        <div class="settings-section">
          <div class="settings-section-title">Copy files</div>
          <p class="settings-section-description">
            Files matching these patterns will be copied from the main repository when creating a
            new worktree.
          </p>

          <!-- Default patterns (cannot be removed) -->
          {#each DEFAULT_WORKTREE_COPY_PATTERNS as pattern (pattern)}
            <div class="pattern-item pattern-default">
              <span class="pattern-text">{pattern}</span>
              <span class="pattern-badge">default</span>
            </div>
          {/each}

          <!-- User patterns (can be removed) -->
          {#each userCopyPatterns as pattern (pattern)}
            <div class="pattern-item pattern-user">
              <span class="pattern-text">{pattern}</span>
              <button
                type="button"
                class="pattern-remove"
                onclick={() => removeCopyPattern(pattern)}
                title="Remove pattern"
              >
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <line x1="18" y1="6" x2="6" y2="18"></line>
                  <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
              </button>
            </div>
          {/each}

          <!-- Add new pattern -->
          <div class="pattern-add">
            <input
              type="text"
              class="pattern-input"
              bind:value={newCopyPattern}
              placeholder="Add pattern (e.g. config/*.local)"
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
              onkeydown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  addCopyPattern();
                }
              }}
            />
            <button
              type="button"
              class="pattern-add-btn"
              onclick={() => addCopyPattern()}
              disabled={!newCopyPattern.trim()}
            >
              Add
            </button>
          </div>
        </div>

        <!-- Initialization Commands Section -->
        <div class="settings-section">
          <div class="settings-section-title">Initialization commands</div>
          <p class="settings-section-description">
            These commands will run in the new worktree after creation.
          </p>

          <!-- Auto-detected package managers -->
          {#each detectedPackageManagers as pm (pm.command)}
            {@const autoOverride = initCommands.find((c) => c.command === pm.command && c.auto)}
            {@const hasUserOverride = initCommands.some((c) => c.command === pm.command && !c.auto)}
            {@const cdMatch = pm.command.match(/^cd (.+?) && /)}
            {@const label = cdMatch ? `${pm.name} in ${cdMatch[1]}` : pm.name}
            {@const isEnabled = autoOverride ? autoOverride.enabled : true}
            {#if !hasUserOverride}
              <div class="command-item command-auto" class:command-disabled={!isEnabled}>
                <input
                  type="checkbox"
                  class="command-checkbox"
                  checked={isEnabled}
                  onchange={() => toggleAutoCommand(pm)}
                />
                <div class="command-info">
                  <span class="command-name">Install dependencies ({label})</span>
                  <span class="command-value">{pm.command}</span>
                </div>
                <span class="pattern-badge">auto</span>
              </div>
            {/if}
          {/each}

          <!-- User-configured commands -->
          {#each initCommands.filter((c) => !c.auto) as cmd (cmd.command)}
            <div class="command-item command-user">
              <input
                type="checkbox"
                class="command-checkbox"
                checked={cmd.enabled}
                onchange={() => toggleInitCommand(cmd.command)}
              />
              <div class="command-info">
                <span class="command-name">{cmd.name}</span>
                <span class="command-value">{cmd.command}</span>
              </div>
              <button
                type="button"
                class="pattern-remove"
                onclick={() => removeInitCommand(cmd.command)}
                title="Remove command"
              >
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <line x1="18" y1="6" x2="6" y2="18"></line>
                  <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
              </button>
            </div>
          {/each}

          <!-- Add new command -->
          <div class="command-add-section">
            <span class="command-add-title">Add new command</span>
            <input
              type="text"
              class="pattern-input"
              bind:value={newInitCommandName}
              placeholder="Name (e.g. Build project)"
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
            />
            <input
              type="text"
              class="pattern-input"
              bind:value={newInitCommandValue}
              placeholder="Command (e.g. cargo build)"
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
              onkeydown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  addInitCommand();
                }
              }}
            />
            <button
              type="button"
              class="pattern-add-btn command-add-btn"
              onclick={() => addInitCommand()}
              disabled={!newInitCommandName.trim() || !newInitCommandValue.trim()}
            >
              Add
            </button>
          </div>
        </div>

        <!-- Port Isolation Section -->
        <div class="settings-section">
          <div class="settings-section-header">
            <div class="settings-section-title">Port isolation</div>
            <label class="toggle-switch">
              <input
                type="checkbox"
                checked={portConfig?.enabled ?? true}
                onchange={() => togglePortIsolation()}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
          <p class="settings-section-description">
            Auto-replace port values to prevent conflicts between worktrees.
          </p>

          {#if portConfig?.enabled}
            <!-- Project port range setting -->
            <div class="port-range-setting">
              <label class="port-range-label">
                <span>Project port range:</span>
                <input
                  type="number"
                  class="port-range-input"
                  value={portConfig.portRangeStart}
                  min="1024"
                  max="65435"
                  step="100"
                  onchange={(e) => updateProjectBasePort(parseInt(e.currentTarget.value, 10))}
                  spellcheck="false"
                  autocomplete="off"
                />
                <span class="port-range-display">– {portConfig.portRangeEnd}</span>
              </label>
              <span class="port-range-hint"
                >100 ports per project (e.g., 20000, 20100, 20200...)</span
              >
            </div>

            {#if isDetectingPorts}
              <div class="port-loading">
                <Spinner size="sm" />
                <span>Detecting ports...</span>
              </div>
            {:else if getUniquePorts().length > 0}
              <div class="port-table-wrap">
                <table class="port-table">
                  <thead>
                    <tr>
                      <th class="port-col-check"></th>
                      <th class="port-col-var">Variable</th>
                      <th class="port-col-before">Before</th>
                      <th class="port-col-after">After</th>
                      <th class="port-col-source">Source</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each getUniquePorts() as port (port.variable_name)}
                      {@const isSelected = selectedPorts.get(port.variable_name) ?? true}
                      {@const transformable = isPortTransformable(port.variable_name)}
                      {@const assigned = getAssignedPortValue(port.variable_name)}
                      {@const sources = getPortSourceFiles(port.variable_name)}
                      <tr
                        class="port-table-row"
                        class:disabled={!isSelected && transformable}
                        class:non-transformable={!transformable}
                      >
                        <td class="port-col-check">
                          <input
                            type="checkbox"
                            class="port-checkbox"
                            checked={isSelected && transformable}
                            disabled={!transformable}
                            onchange={() => togglePortSelection(port.variable_name)}
                          />
                        </td>
                        <td class="port-col-var">
                          <code class="port-var-name">{port.variable_name}</code>
                        </td>
                        <td class="port-col-before">
                          <span class="port-value port-original">{port.port_value}</span>
                        </td>
                        <td class="port-col-after">
                          {#if transformable && isSelected && assigned !== null}
                            <span class="port-value port-new">{assigned}</span>
                          {:else}
                            <span class="port-value port-unchanged">-</span>
                          {/if}
                        </td>
                        <td class="port-col-source">
                          <span class="port-source-files">{sources.join(', ')}</span>
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
              {#if getSelectedPortCount() > 0}
                <div class="port-summary">
                  {getSelectedPortCount()} port{getSelectedPortCount() !== 1 ? 's' : ''} will be transformed
                </div>
              {/if}
            {:else}
              <div class="port-empty">No port variables detected in target files</div>
            {/if}

            <!-- Target files configuration -->
            <div class="target-files-section">
              <div class="target-files-header">Target files</div>
              <div class="target-files-list">
                {#each portConfig.targetFiles ?? DEFAULT_TARGET_FILES as pattern (pattern)}
                  {@const enabled = isTargetFileEnabled(pattern)}
                  <div class="target-file-item" class:disabled={!enabled}>
                    <input
                      type="checkbox"
                      class="target-file-checkbox"
                      checked={enabled}
                      onchange={() => toggleTargetFile(pattern)}
                      title={enabled ? 'Disable' : 'Enable'}
                    />
                    <code class="target-file-pattern">{pattern}</code>
                    {#if isDefaultTargetFile(pattern)}
                      <span class="target-file-default">default</span>
                    {:else}
                      <button
                        type="button"
                        class="target-file-remove"
                        onclick={() => removeTargetFile(pattern)}
                        title="Remove"
                      >
                        ×
                      </button>
                    {/if}
                  </div>
                {/each}
              </div>
              <div class="target-files-add">
                <input
                  type="text"
                  class="target-file-input"
                  placeholder="docker-compose.yml"
                  bind:value={newTargetFile}
                  onkeydown={(e) => e.key === 'Enter' && addTargetFile()}
                  spellcheck="false"
                  autocomplete="off"
                  autocorrect="off"
                  autocapitalize="off"
                />
                <button
                  type="button"
                  class="target-file-add-btn"
                  onclick={() => addTargetFile()}
                  disabled={!newTargetFile.trim()}
                >
                  Add
                </button>
              </div>
              <span class="target-files-hint">
                Detected ports will be replaced in these files
              </span>
            </div>
          {/if}
        </div>

        <!-- Compose Isolation Section -->
        <div class="settings-section" id="compose-isolation-section">
          <div class="settings-section-header">
            <div class="settings-section-title">Compose isolation</div>
            <label class="toggle-switch">
              <input
                type="checkbox"
                checked={composeConfig?.enabled ?? true}
                onchange={() => toggleComposeIsolation()}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
          <p class="settings-section-description">
            Auto-replace project name in docker-compose files to prevent conflicts between
            worktrees.
          </p>

          {#if composeConfig?.enabled}
            {#if isDetectingCompose}
              <div class="port-loading">
                <Spinner size="sm" />
                <span>Scanning compose files...</span>
              </div>
            {:else if detectedComposeFiles && detectedComposeFiles.files.length > 0}
              <div class="compose-list">
                {#each detectedComposeFiles.files as file (file.file_path)}
                  {@const replacement = composeReplacements.find(
                    (r) => r.file_path === file.file_path
                  )}
                  {@const isDisabled = (composeConfig.disabledFiles ?? []).includes(file.file_path)}
                  <div class="compose-file-group" class:disabled={isDisabled}>
                    <div class="compose-file-row">
                      <div class="compose-file-check">
                        {#if file.project_name !== null}
                          <input
                            type="checkbox"
                            class="port-checkbox"
                            checked={!isDisabled}
                            onchange={() => toggleComposeFile(file.file_path)}
                          />
                        {/if}
                      </div>
                      <div class="compose-file-info">
                        <code class="compose-file-path">{file.file_path}</code>
                        {#if file.project_name !== null && replacement && !isDisabled}
                          <span class="compose-name-transform">
                            <code class="compose-name-original">{file.project_name}</code>
                            <span class="compose-arrow">→</span>
                            <code class="compose-name-new">{replacement.new_name}</code>
                          </span>
                        {:else if file.project_name !== null}
                          <span class="compose-name-transform">
                            <code class="compose-name-original">name</code>
                            <span class="compose-will-rename">will be replaced</span>
                          </span>
                        {:else}
                          <span class="compose-note">No name (directory-based)</span>
                        {/if}
                      </div>
                    </div>
                    {#if file.warnings.length > 0}
                      <div class="compose-file-warnings">
                        {#each file.warnings as warning (`${file.file_path}-${warning.line_number}-${warning.value}`)}
                          <div class="compose-warning-item">
                            <span class="compose-warning-icon">⚠</span>
                            <span class="compose-warning-text"
                              >static <code
                                >{warning.warning_type === 'ContainerName'
                                  ? 'container_name'
                                  : 'volume name'}</code
                              > may conflict</span
                            >
                          </div>
                        {/each}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {:else}
              <div class="port-empty">No docker-compose files detected</div>
            {/if}
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .worktree-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .worktree-backdrop.mounted {
    opacity: 1;
  }

  .worktree-modal {
    position: relative;
    width: min(440px, 90vw);
    max-height: 80vh;
    animation: modalSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes modalSlideIn {
    from {
      opacity: 0;
      transform: translateY(-20px) scale(0.95);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .modal-glow {
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: calc(var(--radius-xl) + 2px);
    opacity: 0.06;
    filter: blur(5px);
    z-index: -1;
    transition: opacity 0.3s ease;
  }

  .worktree-modal:hover .modal-glow {
    opacity: 0.1;
  }

  .modal-content {
    display: flex;
    flex-direction: column;
    background: var(--bg-glass);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-glow);
    border-radius: var(--radius-xl);
    overflow: visible;
    box-shadow: var(--shadow-lg);
  }

  /* Top border shine effect */
  .modal-content::before {
    content: '';
    position: absolute;
    top: 0;
    left: 10%;
    right: 10%;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent-color), transparent);
    opacity: 0.6;
    z-index: 1;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid var(--border-color);
    border-radius: var(--radius-xl) var(--radius-xl) 0 0;
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .header-content svg {
    color: var(--accent-color);
    opacity: 0.8;
  }

  .title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .action-btn:hover {
    background: rgba(125, 211, 252, 0.1);
    color: var(--text-secondary);
  }

  .close-btn:hover {
    background: rgba(248, 113, 113, 0.15);
    color: var(--git-deleted);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .settings-btn {
    background: transparent;
    border: none;
    color: var(--accent-color);
    transition: all var(--transition-fast);
  }

  .settings-btn:hover {
    color: var(--accent-color);
    filter: brightness(1.2);
  }

  .settings-btn:hover svg {
    transform: rotate(45deg);
  }

  .settings-btn svg {
    transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .modal-body {
    position: relative;
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
    min-height: 220px;
  }

  /* Initialization summary (above footer) */
  .init-summary {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-4);
    background: rgba(255, 255, 255, 0.02);
    border-top: 1px solid var(--border-subtle);
  }

  .init-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .init-label {
    font-size: 10px;
    color: var(--text-muted);
  }

  .init-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .init-stats {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-1) var(--space-3);
    flex: 1;
  }

  .init-settings-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: color var(--transition-fast);
    flex-shrink: 0;
  }

  .init-settings-btn:hover {
    color: var(--text-secondary);
  }

  .init-settings-btn:active {
    transform: scale(0.95);
  }

  .stat-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    white-space: nowrap;
    cursor: help;
    transition: opacity var(--transition-fast);
  }

  .stat-item:hover {
    opacity: 0.8;
  }

  .stat-icon {
    flex-shrink: 0;
    opacity: 0.6;
    color: var(--text-muted);
    position: relative;
    top: 2px;
  }

  .stat-text {
    color: var(--text-secondary);
  }

  .stat-verb {
    color: var(--text-muted);
    font-style: italic;
  }

  /* Tooltip for stat items */
  .stat-item .tooltip {
    position: absolute;
    bottom: 100%;
    left: 50%;
    transform: translateX(-50%);
    margin-bottom: 8px;
    padding: 8px 12px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    white-space: pre;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    opacity: 0;
    visibility: hidden;
    transition:
      opacity var(--transition-fast),
      visibility var(--transition-fast);
    z-index: 9999;
    pointer-events: none;
  }

  .stat-item .tooltip::before {
    content: '';
    position: absolute;
    bottom: -5px;
    left: 50%;
    transform: translateX(-50%);
    border-width: 5px 5px 0 5px;
    border-style: solid;
    border-color: var(--border-color) transparent transparent transparent;
  }

  .stat-item .tooltip::after {
    content: '';
    position: absolute;
    bottom: -4px;
    left: 50%;
    transform: translateX(-50%);
    border-width: 4px 4px 0 4px;
    border-style: solid;
    border-color: var(--bg-elevated) transparent transparent transparent;
  }

  .stat-item.has-tooltip:hover .tooltip {
    opacity: 1;
    visibility: visible;
  }

  /* Row 2: Standard modal footer */
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-5);
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-top: 1px solid var(--border-subtle);
    border-radius: 0 0 var(--radius-xl) var(--radius-xl);
  }

  .footer-item {
    font-size: 11px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .footer-item kbd {
    padding: 2px 6px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
    box-shadow: 0 1px 0 var(--bg-primary);
    transition: all var(--transition-fast);
  }

  .footer-item:hover kbd {
    color: var(--accent-color);
    border-color: var(--accent-subtle);
    transform: translateY(-1px);
    box-shadow: 0 2px 0 var(--bg-primary);
  }

  .footer-item span {
    margin-left: 2px;
  }

  /* Buttons */
  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: all var(--transition-fast);
    background: var(--bg-elevated);
    color: var(--text-secondary);
  }

  .btn:hover {
    background: var(--bg-glass-hover);
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--accent-subtle);
    border-color: var(--accent-color);
    color: var(--accent-color);
  }

  .btn-primary:hover {
    background: var(--accent-muted);
  }

  .btn-ghost {
    background: transparent;
    border-color: transparent;
    padding: var(--space-1);
    color: var(--text-muted);
  }

  .btn-ghost:hover {
    background: rgba(125, 211, 252, 0.05);
    color: var(--text-secondary);
  }

  /* Worktree Tree View */
  .worktree-tree {
    display: flex;
    flex-direction: column;
    position: relative;
  }

  .tree-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-family: var(--font-mono);
    position: relative;
    border: none;
    background: transparent;
    width: 100%;
    text-align: left;
    transition: background var(--transition-fast);
  }

  /* Parent (Main repository) */
  .tree-parent {
    padding-left: var(--space-3);
  }

  .tree-parent.is-current {
    background: rgba(74, 222, 128, 0.05);
  }

  /* Children (Linked worktrees) */
  .tree-child {
    padding-left: calc(var(--space-3) + 24px);
    cursor: pointer;
  }

  .tree-child:hover {
    background: rgba(251, 191, 36, 0.08);
  }

  /* Connector line for parent-child relationship */
  .tree-connector {
    position: absolute;
    left: calc(var(--space-3) + 2px);
    width: 14px;
    height: 100%;
  }

  .tree-connector::before {
    content: '';
    position: absolute;
    left: 0;
    top: -50%;
    width: 1px;
    height: 100%;
    background: var(--border-color);
  }

  .tree-connector::after {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    width: 10px;
    height: 1px;
    background: var(--border-color);
  }

  .tree-connector.is-last::before {
    height: 50%;
    top: 0;
  }

  .tree-indicator {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .tree-parent .tree-indicator {
    background: var(--git-added);
    box-shadow: 0 0 6px rgba(74, 222, 128, 0.4);
  }

  .tree-child .tree-indicator {
    background: var(--git-modified);
    opacity: 0.6;
  }

  .tree-child:hover .tree-indicator {
    opacity: 1;
    box-shadow: 0 0 6px rgba(251, 191, 36, 0.4);
  }

  .tree-branch {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-primary);
  }

  .tree-parent.is-current .tree-branch {
    color: var(--git-added);
  }

  .tree-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 2px 6px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .tree-label.label-current {
    background: rgba(74, 222, 128, 0.15);
    color: var(--git-added);
  }

  .tree-label.label-wt {
    background: rgba(251, 191, 36, 0.15);
    color: var(--git-modified);
  }

  .tree-locked {
    font-size: 11px;
    flex-shrink: 0;
  }

  /* Section Divider */
  .section-divider {
    height: 1px;
    background: linear-gradient(
      to right,
      transparent 0%,
      var(--border-color) 20%,
      var(--border-color) 80%,
      transparent 100%
    );
    margin: var(--space-4) 0;
  }

  /* Create Section */
  .create-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-height: 150px;
  }

  .section-title {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .form-input {
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-family: var(--font-mono);
    color: var(--text-primary);
    outline: none;
    transition: border-color var(--transition-fast);
    box-sizing: border-box;
  }

  .form-input:focus {
    border-color: var(--accent-color);
  }

  .form-input::placeholder {
    color: var(--text-muted);
  }

  .input-row {
    display: flex;
    align-items: stretch;
    gap: 8px;
  }

  .input-wrapper {
    position: relative;
    flex: 1;
  }

  .input-wrapper .form-input {
    width: 100%;
    padding-right: var(--space-6);
  }

  .input-indicator {
    position: absolute;
    right: var(--space-3);
    top: 50%;
    transform: translateY(-50%);
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    opacity: 0.5;
    pointer-events: none;
    font-family: var(--font-mono);
  }

  .branch-select-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .branch-select-btn:hover:not(:disabled) {
    border-color: var(--accent-color);
    color: var(--accent-color);
  }

  .branch-select-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .path-preview {
    display: flex;
    gap: var(--space-2);
    font-size: 11px;
  }

  .preview-label {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .preview-path {
    color: var(--text-secondary);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .form-error {
    padding: var(--space-2) var(--space-3);
    background: rgba(248, 113, 113, 0.1);
    border: 1px solid rgba(248, 113, 113, 0.3);
    border-radius: var(--radius-sm);
    color: var(--git-deleted);
    font-size: 12px;
  }

  .form-warning {
    background: rgba(251, 191, 36, 0.1);
    border-color: rgba(251, 191, 36, 0.3);
    color: var(--git-modified);
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  /* Loading State */
  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-6);
    color: var(--text-muted);
    font-size: 13px;
  }

  /* Initialization Overlay */
  .init-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    background: var(--bg-glass);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border-radius: var(--radius-xl);
    color: var(--text-muted);
    font-size: 13px;
    z-index: 10;
  }

  /* Settings Modal */
  .settings-modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
    opacity: 0;
    animation: fadeIn 0.2s ease forwards;
  }

  .settings-modal {
    position: relative;
    width: min(380px, 85vw);
    max-height: 70vh;
    animation: modalSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .settings-modal::before {
    content: '';
    position: absolute;
    inset: -1px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: calc(var(--radius-lg) + 1px);
    opacity: 0.08;
    filter: blur(3px);
    z-index: -1;
  }

  .settings-modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid var(--border-color);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  }

  .settings-modal-header .btn-ghost {
    width: 24px;
    height: 24px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .settings-modal-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .settings-modal-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    max-height: 500px;
    overflow-y: auto;
    background: var(--bg-glass);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-glow);
    border-top: none;
    border-radius: 0 0 var(--radius-lg) var(--radius-lg);
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .settings-section-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .settings-section-description {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
    line-height: 1.5;
  }

  .pattern-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-family: var(--font-mono);
  }

  .pattern-default {
    opacity: 0.7;
  }

  .pattern-text {
    flex: 1;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pattern-badge {
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 2px 6px;
    background: rgba(125, 211, 252, 0.1);
    border-radius: 3px;
    color: var(--text-muted);
  }

  .pattern-remove {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .pattern-remove:hover {
    background: rgba(248, 113, 113, 0.15);
    color: var(--git-deleted);
  }

  .pattern-add {
    display: flex;
    align-items: stretch;
    gap: var(--space-2);
    margin-top: var(--space-1);
  }

  .pattern-input {
    flex: 1;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--text-primary);
    outline: none;
    transition: border-color var(--transition-fast);
  }

  .pattern-input:focus {
    border-color: var(--accent-color);
  }

  .pattern-input::placeholder {
    color: var(--text-muted);
  }

  .pattern-add-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-2) var(--space-4);
    background: linear-gradient(
      135deg,
      rgba(125, 211, 252, 0.08) 0%,
      rgba(196, 181, 253, 0.05) 100%
    );
    border: 1px solid rgba(125, 211, 252, 0.25);
    border-radius: var(--radius-sm);
    color: var(--accent-color);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
    transition: all var(--transition-fast);
    position: relative;
    overflow: hidden;
  }

  .pattern-add-btn::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      135deg,
      rgba(125, 211, 252, 0.15) 0%,
      rgba(196, 181, 253, 0.1) 100%
    );
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .pattern-add-btn:hover:not(:disabled) {
    border-color: rgba(125, 211, 252, 0.5);
    color: var(--accent-color);
    box-shadow: 0 0 12px rgba(125, 211, 252, 0.15);
    transform: translateY(-1px);
  }

  .pattern-add-btn:hover:not(:disabled)::before {
    opacity: 1;
  }

  .pattern-add-btn:active:not(:disabled) {
    transform: translateY(0);
    box-shadow: 0 0 6px rgba(125, 211, 252, 0.1);
  }

  .pattern-add-btn:disabled {
    opacity: 0.25;
    cursor: not-allowed;
    background: var(--bg-tertiary);
    border-color: var(--border-subtle);
    color: var(--text-muted);
  }

  /* Branch Selection Modal */
  .branch-modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
    opacity: 0;
    animation: fadeIn 0.2s ease forwards;
  }

  @keyframes fadeIn {
    to {
      opacity: 1;
    }
  }

  .branch-modal {
    position: relative;
    width: min(380px, 85vw);
    max-height: 50vh;
    animation: modalSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .branch-modal::before {
    content: '';
    position: absolute;
    inset: -1px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: calc(var(--radius-lg) + 1px);
    opacity: 0.08;
    filter: blur(3px);
    z-index: -1;
  }

  .branch-modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid var(--border-color);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
  }

  .branch-modal-header .btn-ghost {
    width: 24px;
    height: 24px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .branch-modal-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .branch-modal-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2);
    max-height: 260px;
    background: var(--bg-glass);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid var(--border-glow);
    border-top: none;
    border-radius: 0 0 var(--radius-lg) var(--radius-lg);
  }

  .branch-item {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    font-size: 13px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: left;
  }

  .branch-item:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .branch-item::after {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 0;
    background: var(--accent-color);
    border-radius: 0 2px 2px 0;
    transition: height var(--transition-fast);
  }

  .branch-item:hover::after {
    height: 60%;
  }

  .branch-item-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .branch-item-time {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-sans);
    opacity: 0.7;
    transition: opacity var(--transition-fast);
  }

  .branch-item:hover .branch-item-time {
    opacity: 1;
  }

  .branch-icon-small {
    flex-shrink: 0;
    color: var(--text-muted);
    transition: all var(--transition-fast);
  }

  .branch-item:hover .branch-icon-small {
    color: var(--accent-color);
    transform: scale(1.1);
  }

  .empty-state {
    text-align: center;
    padding: var(--space-6);
    color: var(--text-muted);
    font-size: 13px;
  }

  /* Scrollbar */
  .modal-body::-webkit-scrollbar,
  .branch-modal-body::-webkit-scrollbar {
    width: 6px;
  }

  .modal-body::-webkit-scrollbar-track,
  .branch-modal-body::-webkit-scrollbar-track {
    background: transparent;
  }

  .modal-body::-webkit-scrollbar-thumb,
  .branch-modal-body::-webkit-scrollbar-thumb {
    background: linear-gradient(180deg, var(--border-color), var(--border-subtle));
    border-radius: 3px;
    transition: all var(--transition-normal);
  }

  .modal-body:hover::-webkit-scrollbar-thumb,
  .branch-modal-body:hover::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.3);
  }

  /* Task List Progress View */
  .progress-task-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) 0;
  }

  .progress-task-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    transition: all var(--transition-normal);
    animation: taskFadeIn 0.3s cubic-bezier(0.16, 1, 0.3, 1) both;
  }

  .progress-task-item:nth-child(1) {
    animation-delay: 0ms;
  }
  .progress-task-item:nth-child(2) {
    animation-delay: 50ms;
  }
  .progress-task-item:nth-child(3) {
    animation-delay: 100ms;
  }
  .progress-task-item:nth-child(4) {
    animation-delay: 150ms;
  }
  .progress-task-item:nth-child(5) {
    animation-delay: 200ms;
  }
  .progress-task-item:nth-child(6) {
    animation-delay: 250ms;
  }
  .progress-task-item:nth-child(7) {
    animation-delay: 300ms;
  }
  .progress-task-item:nth-child(8) {
    animation-delay: 350ms;
  }

  @keyframes taskFadeIn {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* Status-specific styles */
  .progress-task-item.is-pending {
    opacity: 0.35;
  }

  .progress-task-item.is-running {
    opacity: 1;
    background: rgba(125, 211, 252, 0.05);
    border: 1px solid rgba(125, 211, 252, 0.08);
  }

  .progress-task-item.is-completed {
    opacity: 0.85;
  }

  .progress-task-item.is-completed .task-status-icon {
    color: var(--git-added);
    filter: drop-shadow(0 0 4px rgba(74, 222, 128, 0.3));
  }

  .progress-task-item.is-failed {
    opacity: 0.9;
  }

  .progress-task-item.is-failed .task-status-icon {
    color: var(--git-modified);
    filter: drop-shadow(0 0 4px rgba(251, 191, 36, 0.3));
  }

  .task-status-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 20px;
    flex-shrink: 0;
    transition: all var(--transition-normal);
  }

  /* Pending: mist particle dot */
  .task-icon-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-muted);
    opacity: 0.5;
  }

  /* Completed: checkmark with entrance animation */
  .task-icon-completed {
    animation: taskCheckIn 0.35s cubic-bezier(0.16, 1, 0.3, 1) both;
  }

  @keyframes taskCheckIn {
    from {
      opacity: 0;
      transform: scale(0.5);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  /* Failed: subtle pulse on the icon */
  .task-icon-failed {
    animation: taskCheckIn 0.35s cubic-bezier(0.16, 1, 0.3, 1) both;
  }

  .task-content {
    display: flex;
    flex-direction: row;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    line-height: 20px;
    overflow: hidden;
  }

  .task-name {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .progress-task-item.is-running .task-name {
    color: var(--text-primary);
  }

  .task-detail {
    font-size: 10px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .progress-task-item.is-failed .task-detail {
    color: var(--git-modified);
  }

  /* Init Commands Settings */
  .command-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
  }

  .command-auto {
    opacity: 0.85;
  }

  .command-auto.command-disabled {
    opacity: 0.4;
  }

  .command-auto.command-disabled .command-name,
  .command-auto.command-disabled .command-value {
    text-decoration: line-through;
  }

  .command-checkbox {
    width: 14px;
    height: 14px;
    margin-top: 3px;
    accent-color: var(--accent-color);
    cursor: pointer;
  }

  .command-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .command-name {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .command-value {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .command-add-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .command-add-title {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
    letter-spacing: 0.02em;
  }

  .command-add-btn {
    align-self: flex-end;
  }

  /* Port Isolation Section */
  .settings-section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .toggle-switch {
    position: relative;
    display: inline-block;
    width: 36px;
    height: 20px;
  }

  .toggle-switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 20px;
    transition: all var(--transition-fast);
  }

  .toggle-slider::before {
    position: absolute;
    content: '';
    height: 14px;
    width: 14px;
    left: 2px;
    bottom: 2px;
    background-color: var(--text-muted);
    border-radius: 50%;
    transition: all var(--transition-fast);
  }

  .toggle-switch input:checked + .toggle-slider {
    background-color: rgba(125, 211, 252, 0.2);
    border-color: var(--accent-color);
  }

  .toggle-switch input:checked + .toggle-slider::before {
    transform: translateX(16px);
    background-color: var(--accent-color);
  }

  .port-loading {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    color: var(--text-muted);
    font-size: 12px;
  }

  .port-range-setting {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    margin-bottom: var(--space-3);
  }

  .port-range-label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 13px;
    color: var(--text-secondary);
  }

  .port-range-input {
    width: 80px;
    padding: var(--space-1) var(--space-2);
    background: var(--bg-primary);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-mono);
    text-align: center;
  }

  .port-range-input:focus {
    outline: none;
    border-color: var(--accent-color);
    box-shadow: 0 0 0 2px var(--accent-color-alpha);
  }

  .port-range-display {
    font-family: var(--font-mono);
    color: var(--text-muted);
  }

  .port-range-hint {
    font-size: 11px;
    color: var(--text-muted);
  }

  .port-table-wrap {
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    overflow-x: auto;
    overflow-y: hidden;
  }

  .port-table {
    width: 100%;
    border-collapse: collapse;
    white-space: nowrap;
    font-size: 11px;
  }

  .port-table thead th {
    padding: var(--space-2);
    background: rgba(0, 0, 0, 0.15);
    font-size: 9px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    text-align: left;
  }

  .port-table tbody tr {
    border-top: 1px solid var(--border-subtle);
    transition: opacity var(--transition-fast);
  }

  .port-table tbody td {
    padding: var(--space-2);
  }

  .port-table-row.disabled {
    opacity: 0.4;
    pointer-events: auto;
    cursor: default;
  }

  .port-table-row.non-transformable {
    opacity: 0.3;
    pointer-events: auto;
    cursor: default;
  }

  .port-table-row.non-transformable .port-checkbox {
    pointer-events: none;
    cursor: not-allowed;
  }

  .port-col-check {
    width: 20px;
  }

  .port-col-var {
    width: 23ch;
    max-width: 23ch;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-mono);
  }

  .port-col-before,
  .port-col-after {
    text-align: right;
  }

  .port-col-source {
    padding-left: var(--space-3);
  }

  .port-checkbox {
    width: 12px;
    height: 12px;
    accent-color: var(--accent-color);
    cursor: pointer;
  }

  .port-var-name {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-primary);
  }

  .port-value {
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .port-original {
    color: var(--text-muted);
  }

  .port-new {
    color: var(--accent-color);
    font-weight: 500;
  }

  .port-unchanged {
    color: var(--text-muted);
    opacity: 0.5;
  }

  .port-source-files {
    font-size: 9px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .port-summary {
    font-size: 11px;
    color: var(--accent-color);
    padding: var(--space-2) 0;
  }

  .port-empty {
    padding: var(--space-3);
    text-align: center;
    color: var(--text-muted);
    font-size: 11px;
  }

  /* Target files section */
  .target-files-section {
    margin-top: var(--space-3);
    padding-top: var(--space-3);
    border-top: 1px solid var(--border-subtle);
  }

  .target-files-header {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: var(--space-2);
  }

  .target-files-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    margin-bottom: var(--space-2);
  }

  .target-file-item {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: 2px 6px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-size: 11px;
  }

  .target-file-item.disabled {
    opacity: 0.5;
    pointer-events: auto;
    cursor: default;
  }

  .target-file-checkbox {
    width: 12px;
    height: 12px;
    accent-color: var(--accent-color);
    cursor: pointer;
    margin: 0;
  }

  .target-file-pattern {
    font-family: var(--font-mono);
    color: var(--text-primary);
    font-size: 10px;
  }

  .target-file-item.disabled .target-file-pattern {
    text-decoration: line-through;
  }

  .target-file-default {
    font-size: 8px;
    font-weight: 500;
    text-transform: uppercase;
    color: var(--text-muted);
    opacity: 0.7;
  }

  .target-file-remove {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0 2px;
    font-size: 12px;
    line-height: 1;
    opacity: 0.6;
    transition: opacity 0.15s;
  }

  .target-file-remove:hover {
    opacity: 1;
    color: var(--text-primary);
  }

  .target-files-add {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-1);
  }

  .target-file-input {
    flex: 1;
    height: 26px;
    padding: 0 var(--space-2);
    font-size: 11px;
    font-family: var(--font-mono);
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    outline: none;
    transition: border-color var(--transition-fast);
  }

  .target-file-input:focus {
    border-color: var(--accent-color);
  }

  .target-file-input::placeholder {
    color: var(--text-muted);
    opacity: 0.6;
  }

  .target-file-add-btn {
    height: 26px;
    padding: 0 var(--space-2);
    font-size: 11px;
    font-weight: 500;
    color: var(--text-primary);
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .target-file-add-btn:hover:not(:disabled) {
    background: var(--bg-tertiary);
    border-color: var(--border-default);
  }

  .target-file-add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .target-files-hint {
    font-size: 10px;
    color: var(--text-muted);
    display: block;
  }

  /* Compose isolation list */
  .compose-list {
    display: flex;
    flex-direction: column;
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .compose-file-group {
    transition: opacity var(--transition-fast);
  }

  .compose-file-group + .compose-file-group {
    border-top: 1px solid var(--border-subtle);
  }

  .compose-file-group.disabled {
    opacity: 0.4;
    pointer-events: auto;
    cursor: default;
  }

  .compose-file-row {
    display: flex;
    align-items: flex-start;
    padding: var(--space-2);
    gap: var(--space-1);
  }

  .compose-file-check {
    width: 20px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding-top: 1px;
  }

  .compose-file-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .compose-file-path {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
  }

  .compose-name-transform {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
  }

  .compose-name-original {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent-color);
  }

  .compose-arrow {
    color: var(--text-muted);
    opacity: 0.5;
    font-size: 10px;
  }

  .compose-name-new {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent-color);
    font-weight: 500;
  }

  .compose-will-rename {
    font-size: 10px;
    color: var(--text-muted);
    font-style: italic;
  }

  .compose-note {
    font-size: 10px;
    color: var(--text-muted);
    font-style: italic;
  }

  /* Compose file-level warnings */
  .compose-file-warnings {
    padding: 0 var(--space-2) var(--space-2) 28px;
  }

  .compose-warning-item {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 1px 0;
    font-size: 10px;
  }

  .compose-warning-icon {
    flex-shrink: 0;
    font-size: 9px;
    color: var(--accent3-color);
  }

  .compose-warning-text {
    color: var(--text-muted);
    font-size: 10px;
  }

  .compose-warning-text code {
    font-family: var(--font-mono);
    color: var(--accent3-color);
    font-size: 10px;
  }

  .compose-conflict-banner {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    background: rgba(252, 211, 77, 0.08);
    border-bottom: 1px solid rgba(252, 211, 77, 0.15);
    font-size: 11px;
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .compose-conflict-banner:hover {
    background: rgba(252, 211, 77, 0.12);
  }

  .compose-conflict-icon {
    font-size: 12px;
    flex-shrink: 0;
  }

  .compose-conflict-text {
    color: var(--accent3-color);
  }

  .compose-conflict-link {
    color: var(--accent-color);
    margin-left: auto;
    font-size: 10px;
    opacity: 0.7;
  }

  .compose-conflict-banner:hover .compose-conflict-link {
    opacity: 1;
  }
</style>
