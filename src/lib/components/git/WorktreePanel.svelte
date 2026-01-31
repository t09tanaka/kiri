<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { Spinner } from '@/lib/components/ui';
  import { worktreeStore } from '@/lib/stores/worktreeStore';
  import { worktreeViewStore } from '@/lib/stores/worktreeViewStore';
  import { worktreeService } from '@/lib/services/worktreeService';
  import { windowService } from '@/lib/services/windowService';
  import type { WorktreeInfo, BranchInfo, WorktreeContext } from '@/lib/services/worktreeService';
  import { branchToWorktreeName } from '@/lib/utils/gitWorktree';

  interface Props {
    projectPath: string;
    onClose: () => void;
  }

  let { projectPath, onClose }: Props = $props();

  let isLoading = $state(false);
  let mounted = $state(false);

  // Create form state
  let createName = $state('');
  let isCreating = $state(false);
  let branches = $state<BranchInfo[]>([]);
  let createError = $state<string | null>(null);
  let showBranchDropdown = $state(false);
  let isExistingBranch = $state(false);

  // Worktree context for current window
  let currentContext = $state<WorktreeContext | null>(null);

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
    await loadWorktrees();
    await loadBranches();
    await loadContext();
  });

  async function loadContext() {
    try {
      currentContext = await worktreeService.getContext(projectPath);
    } catch {
      currentContext = null;
    }
  }

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
    document.removeEventListener('click', handleDocumentClick, true);
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      if (showBranchDropdown) {
        showBranchDropdown = false;
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
    isLoading = true;
    try {
      await worktreeStore.refresh(targetPath);
    } finally {
      isLoading = false;
    }
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
    isCreating = true;
    createError = null;

    // Capture projectPath in a local variable to avoid closure issues
    const currentProjectPath = projectPath;

    try {
      const branchName = createName.trim();
      const wtName = branchToWorktreeName(branchName);
      const wt = await worktreeService.create(
        currentProjectPath,
        wtName, // worktree name (with '/' replaced by '-')
        branchName, // branch name (original, may contain '/')
        !isExistingBranch // create new branch if not selecting existing
      );

      // Close modal and ensure UI updates before opening new window
      worktreeViewStore.close();
      await tick();

      // Background tasks after modal is closed
      loadWorktrees(currentProjectPath).catch(console.error);
      openWorktreeWindow(wt);
    } catch (e) {
      createError = e instanceof Error ? e.message : String(e);
      isCreating = false;
      // Force UI update after state change in async context
      await tick();
    }
  }

  async function openWorktreeWindow(wt: WorktreeInfo) {
    try {
      await windowService.createWindow({ projectPath: wt.path });
    } catch (e) {
      console.error('Failed to open worktree window:', e);
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

      <div class="modal-body">
        <!-- Worktree Tree View -->
        {#if isLoading && !mounted}
          <div class="loading-state">
            <Spinner size={24} />
            <span>Loading worktrees...</span>
          </div>
        {:else}
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
        {/if}

        <!-- Separator -->
        <div class="section-divider"></div>

        <!-- Create Form -->
        <div class="create-section">
          <div class="section-title">New worktree</div>

          <div class="form-group">
            <div class="input-row">
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
            {#if isExistingBranch}
              <span class="input-hint">Using existing branch</span>
            {:else if createName.trim()}
              <span class="input-hint">Will create new branch</span>
            {/if}
          </div>

          {#if createName.trim()}
            <div class="path-preview">
              <span class="preview-label">Path:</span>
              <span class="preview-path">{pathPreview()}</span>
            </div>
          {/if}

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
                <Spinner size={12} /> Creating...
              {:else}
                Open
              {/if}
            </button>
          </div>
        </div>
      </div>

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
        <button type="button" class="btn btn-ghost" onclick={() => (showBranchDropdown = false)}>
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
          </button>
        {/each}
        {#if availableBranches().length === 0}
          <div class="empty-state">No available branches</div>
        {/if}
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
    overflow: hidden;
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

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-5);
    padding: var(--space-3) var(--space-4);
    background: rgba(0, 0, 0, 0.2);
    border-top: 1px solid var(--border-subtle);
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

  .input-row .form-input {
    flex: 1;
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

  .input-hint {
    font-size: 11px;
    color: var(--text-muted);
  }

  .path-preview {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
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
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
</style>
