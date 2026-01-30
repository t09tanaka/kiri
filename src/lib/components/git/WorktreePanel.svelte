<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { Spinner } from '@/lib/components/ui';
  import { worktreeStore } from '@/lib/stores/worktreeStore';
  import { worktreeViewStore } from '@/lib/stores/worktreeViewStore';
  import { worktreeService } from '@/lib/services/worktreeService';
  import { windowService } from '@/lib/services/windowService';
  import { eventService } from '@/lib/services/eventService';
  import { confirmDialogStore } from '@/lib/stores/confirmDialogStore';
  import type { WorktreeInfo, BranchInfo } from '@/lib/services/worktreeService';

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

  const worktrees = $derived($worktreeStore.worktrees);

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

  // Compute worktree path preview
  const pathPreview = $derived(() => {
    if (!createName || !projectPath) return '';
    const parts = projectPath.split('/');
    const repoName = parts[parts.length - 1] || parts[parts.length - 2] || 'repo';
    const parentPath = parts.slice(0, -1).join('/');
    return `${parentPath}/${repoName}-${createName}`;
  });

  onMount(async () => {
    mounted = true;
    document.addEventListener('keydown', handleKeyDown, true);
    document.addEventListener('click', handleDocumentClick, true);
    await loadWorktrees();
    await loadBranches();
  });

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
      // Branch name = Worktree name (unified concept)
      const wt = await worktreeService.create(
        currentProjectPath,
        branchName, // worktree name = branch name
        branchName,
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

  async function handleRemove(wt: WorktreeInfo) {
    const confirmed = await confirmDialogStore.confirm({
      title: 'Remove Worktree',
      message: `Remove worktree "${wt.name}" and its directory?\n\nPath: ${wt.path}`,
      confirmLabel: 'Remove',
      cancelLabel: 'Cancel',
      kind: 'warning',
    });

    if (!confirmed) return;

    try {
      await worktreeService.remove(projectPath, wt.name);
      // Notify other windows that this worktree was removed
      await eventService.emit('worktree-removed', { path: wt.path });
      await loadWorktrees();
    } catch (e) {
      console.error('Failed to remove worktree:', e);
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
<div class="modal-backdrop" onclick={handleBackdropClick}>
  <div class="modal-container">
    <div class="modal-header">
      <h2 class="modal-title">Worktrees</h2>
      <button type="button" class="btn btn-ghost" onclick={() => onClose()} title="Close (Esc)">
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
      <div class="create-form">
        <div class="form-group">
          <label class="form-label" for="wt-name">Branch Name</label>
          <div class="input-row">
            <!-- svelte-ignore a11y_autofocus -->
            <input
              id="wt-name"
              type="text"
              class="form-input"
              bind:value={createName}
              placeholder="e.g. fix-sidebar"
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
              Create & Open
            {/if}
          </button>
        </div>
      </div>

      {#if isLoading && !mounted}
        <div class="loading-state">
          <Spinner size={24} />
          <span>Loading worktrees...</span>
        </div>
      {:else}
        <div class="worktree-list">
          <!-- Main repository (current) -->
          {#each worktrees.filter((w) => w.is_main) as wt (wt.path)}
            <div
              class="main-repo-wrapper"
              class:has-worktrees={worktrees.filter((w) => !w.is_main).length > 0}
            >
              <div class="main-repo-row">
                <div class="main-tree-connector">
                  <div class="main-tree-horizontal"></div>
                </div>
                <div class="main-repo-card">
                  <div class="main-repo-indicator"></div>
                  <svg
                    class="branch-icon"
                    width="14"
                    height="14"
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
                  <span class="branch-name">{wt.branch ?? 'detached'}</span>
                  <span class="label label-current">Current</span>
                </div>
              </div>
              <div class="tree-stem"></div>
            </div>
          {/each}

          <!-- Worktrees (clickable) with tree connector -->
          {#each worktrees.filter((w) => !w.is_main) as wt, index (wt.path)}
            {@const isLast = index === worktrees.filter((w) => !w.is_main).length - 1}
            <div class="worktree-row" class:is-last={isLast}>
              <div class="tree-connector">
                <div class="tree-vertical"></div>
                <div class="tree-horizontal"></div>
              </div>
              <button
                type="button"
                class="worktree-card"
                class:is-invalid={!wt.is_valid}
                onclick={() => openWorktreeWindow(wt)}
                title="Click to open worktree"
              >
                <div class="worktree-indicator"></div>
                <span class="label label-worktree">WT</span>
                <span class="branch-name">{wt.branch ?? 'detached'}</span>
                {#if wt.is_locked}
                  <span class="label label-locked" title="Locked">🔒</span>
                {/if}
                {#if !wt.is_valid}
                  <span class="label label-invalid">invalid</span>
                {/if}
                <div class="wt-actions">
                  <span class="open-hint">Open →</span>
                  {#if !wt.is_locked}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <span
                      role="button"
                      tabindex="0"
                      class="remove-btn"
                      title="Remove worktree"
                      onclick={(e) => {
                        e.stopPropagation();
                        handleRemove(wt);
                      }}
                    >
                      <svg
                        width="14"
                        height="14"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                      >
                        <polyline points="3 6 5 6 21 6"></polyline>
                        <path
                          d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                        ></path>
                      </svg>
                    </span>
                  {/if}
                </div>
              </button>
            </div>
          {/each}

          {#if worktrees.length === 0 && !isLoading}
            <div class="empty-state">No worktrees found</div>
          {/if}
        </div>
      {/if}
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
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.15s ease;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .modal-container {
    width: min(560px, 90vw);
    max-height: 80vh;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow:
      0 24px 48px rgba(0, 0, 0, 0.4),
      0 0 1px rgba(125, 211, 252, 0.1);
    animation: slideUp 0.2s ease;
  }

  @keyframes slideUp {
    from {
      transform: translateY(12px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-tertiary);
  }

  .modal-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    letter-spacing: 0.02em;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4) var(--space-5);
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
    transform: translateY(-1px);
  }

  .btn:active {
    transform: translateY(0) scale(0.98);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
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
    color: var(--text-muted);
  }

  .btn-ghost:hover {
    background: rgba(125, 211, 252, 0.05);
    color: var(--text-secondary);
  }

  /* Create Form */
  .create-form {
    padding: var(--space-4);
    margin-bottom: var(--space-4);
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    overflow: visible;
  }

  .form-group {
    margin-bottom: var(--space-3);
  }

  .mode-fieldset {
    border: none;
    padding: 0;
    margin: 0;
  }

  .form-label {
    display: block;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
    margin-bottom: var(--space-1);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .form-input {
    width: 100%;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-primary);
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

  /* Input row with separate dropdown */
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
    min-height: 34px;
    align-self: stretch;
    background: linear-gradient(
      135deg,
      rgba(125, 211, 252, 0.08) 0%,
      rgba(125, 211, 252, 0.02) 100%
    );
    border: 1px solid rgba(125, 211, 252, 0.15);
    border-radius: var(--radius-sm);
    color: rgba(125, 211, 252, 0.6);
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    overflow: hidden;
  }

  .branch-select-btn::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(125, 211, 252, 0.15) 0%, transparent 50%);
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .branch-select-btn:hover:not(:disabled) {
    background: linear-gradient(
      135deg,
      rgba(125, 211, 252, 0.15) 0%,
      rgba(125, 211, 252, 0.05) 100%
    );
    border-color: rgba(125, 211, 252, 0.4);
    color: var(--accent-color);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(125, 211, 252, 0.15);
  }

  .branch-select-btn:hover:not(:disabled)::before {
    opacity: 1;
  }

  .branch-select-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .branch-select-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  /* Branch Selection Modal */
  .branch-modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
    animation: modalFadeIn 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  @keyframes modalFadeIn {
    from {
      opacity: 0;
      backdrop-filter: blur(0);
    }
    to {
      opacity: 1;
      backdrop-filter: blur(8px);
    }
  }

  .branch-modal {
    width: min(380px, 85vw);
    max-height: 50vh;
    background: linear-gradient(180deg, rgba(30, 41, 59, 0.98) 0%, rgba(15, 23, 42, 0.98) 100%);
    border: 1px solid rgba(125, 211, 252, 0.1);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow:
      0 0 0 1px rgba(125, 211, 252, 0.05),
      0 24px 64px rgba(0, 0, 0, 0.5),
      0 0 80px rgba(125, 211, 252, 0.03);
    animation: modalSlideIn 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  @keyframes modalSlideIn {
    from {
      opacity: 0;
      transform: scale(0.95) translateY(10px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  .branch-modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    border-bottom: 1px solid rgba(125, 211, 252, 0.08);
    background: rgba(125, 211, 252, 0.02);
  }

  .branch-modal-header .btn-ghost {
    width: 28px;
    height: 28px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    color: var(--text-muted);
  }

  .branch-modal-header .btn-ghost:hover {
    background: rgba(248, 113, 113, 0.1);
    color: var(--git-deleted);
  }

  .branch-modal-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .branch-modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    max-height: 220px;
  }

  .branch-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 10px 12px;
    background: transparent;
    border: none;
    border-radius: 8px;
    font-size: 13px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
    position: relative;
  }

  .branch-item::before {
    content: '';
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    width: 3px;
    height: 0;
    background: var(--accent-color);
    border-radius: 0 2px 2px 0;
    transition: height 0.15s ease;
  }

  .branch-item:hover {
    background: rgba(125, 211, 252, 0.06);
    color: var(--text-primary);
    padding-left: 16px;
  }

  .branch-item:hover::before {
    height: 60%;
  }

  .branch-item:active {
    background: rgba(125, 211, 252, 0.1);
  }

  .branch-item:not(:last-child)::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 1px;
    background: linear-gradient(
      to right,
      transparent 0%,
      rgba(125, 211, 252, 0.015) 50%,
      transparent 100%
    );
  }

  .branch-item-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .branch-icon-small {
    flex-shrink: 0;
    color: rgba(125, 211, 252, 0.4);
    transition: color 0.15s ease;
  }

  .branch-item:hover .branch-icon-small {
    color: var(--accent-color);
  }

  .input-hint {
    display: block;
    font-size: 11px;
    color: var(--text-muted);
    margin-top: var(--space-1);
  }

  .path-preview {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-primary);
    border-radius: var(--radius-sm);
    font-size: 11px;
    margin-bottom: var(--space-3);
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
    margin-bottom: var(--space-3);
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

  /* Worktree List */
  .worktree-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  /* Main Repository Card */
  .main-repo-wrapper {
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .main-repo-row {
    display: flex;
    align-items: center;
  }

  .main-tree-connector {
    display: none;
  }

  .main-tree-horizontal {
    display: none;
  }

  .main-repo-card {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 1;
    padding: var(--space-3) var(--space-4);
    padding-left: calc(var(--space-4) + 8px);
    background: var(--bg-secondary);
    border: 1px solid rgba(74, 222, 128, 0.25);
    border-radius: var(--radius-sm);
    position: relative;
    overflow: hidden;
  }

  /* Vertical line from main repo down to worktrees */
  .tree-stem {
    display: none;
  }

  .main-repo-wrapper.has-worktrees::after {
    content: '';
    position: absolute;
    left: 30px;
    top: 100%;
    width: 1px;
    height: var(--space-2);
    background: var(--git-modified);
    opacity: 0.4;
  }

  .main-repo-indicator {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    background: linear-gradient(180deg, var(--git-added) 0%, rgba(74, 222, 128, 0.4) 100%);
  }

  .main-repo-card .branch-icon {
    color: var(--git-added);
    flex-shrink: 0;
  }

  .main-repo-card .branch-name {
    color: var(--git-added);
  }

  .main-repo-card:hover {
    border-color: rgba(74, 222, 128, 0.5);
    background: rgba(74, 222, 128, 0.05);
  }

  /* Tree connector for worktrees */
  .worktree-row {
    display: flex;
    align-items: center;
    position: relative;
    padding-left: 24px;
  }

  /* Vertical line connecting worktrees (except last) */
  .worktree-row::before {
    content: '';
    position: absolute;
    left: 30px;
    top: 0;
    bottom: calc(-1 * var(--space-2));
    width: 1px;
    background: var(--git-modified);
    opacity: 0.4;
  }

  /* Last item: vertical line only goes to center */
  .worktree-row.is-last::before {
    bottom: 50%;
  }

  .tree-connector {
    display: flex;
    align-items: center;
    width: 24px;
    flex-shrink: 0;
    position: relative;
  }

  /* Horizontal line from vertical to card */
  .tree-connector::before {
    content: '';
    position: absolute;
    left: 6px;
    width: 18px;
    height: 1px;
    background: var(--git-modified);
    opacity: 0.4;
  }

  .tree-vertical,
  .tree-horizontal {
    display: none;
  }

  /* Worktree Card (Clickable) */
  .worktree-card {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 1;
    padding: var(--space-2) var(--space-4);
    padding-left: calc(var(--space-4) + 4px);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    position: relative;
    overflow: hidden;
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: left;
    font-family: inherit;
  }

  .worktree-card:hover {
    border-color: rgba(251, 191, 36, 0.5);
    background: rgba(251, 191, 36, 0.08);
    transform: translateX(4px);
  }

  .worktree-card:active {
    transform: translateX(4px) scale(0.99);
  }

  .worktree-indicator {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--git-modified);
    opacity: 0.6;
    transition: opacity var(--transition-fast);
  }

  .worktree-card:hover .worktree-indicator {
    opacity: 1;
  }

  .worktree-card.is-invalid {
    opacity: 0.6;
    border-color: rgba(248, 113, 113, 0.3);
  }

  .worktree-card.is-invalid .worktree-indicator {
    background: var(--git-deleted);
  }

  .branch-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .worktree-card .branch-name {
    color: var(--text-primary);
  }

  .wt-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-shrink: 0;
    margin-left: var(--space-3);
  }

  .open-hint {
    font-size: 11px;
    color: var(--text-muted);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .worktree-card:hover .open-hint {
    opacity: 1;
    color: var(--git-modified);
  }

  .remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    opacity: 0;
    transition: all var(--transition-fast);
    cursor: pointer;
  }

  .worktree-card:hover .remove-btn {
    opacity: 1;
  }

  .remove-btn:hover {
    background: rgba(248, 113, 113, 0.15);
    color: var(--git-deleted);
  }

  /* Labels */
  .label {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 600;
    padding: 0 5px;
    height: 16px;
    line-height: 1;
    border-radius: 3px;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    flex-shrink: 0;
    position: relative;
    top: 5px;
  }

  .label-current {
    background: rgba(74, 222, 128, 0.2);
    color: var(--git-added);
  }

  .label-worktree {
    background: rgba(251, 191, 36, 0.3);
    color: var(--git-modified);
  }

  .label-locked {
    font-size: 12px;
    padding: 0;
    height: auto;
    background: none;
  }

  .label-invalid {
    background: rgba(248, 113, 113, 0.2);
    color: var(--git-deleted);
  }

  /* States */
  .loading-state {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-8);
    color: var(--text-muted);
    font-size: 13px;
  }

  .empty-state {
    text-align: center;
    padding: var(--space-8);
    color: var(--text-muted);
    font-size: 13px;
  }

  /* Scrollbar */
  .modal-body::-webkit-scrollbar {
    width: 6px;
  }

  .modal-body::-webkit-scrollbar-track {
    background: transparent;
  }

  .modal-body::-webkit-scrollbar-thumb {
    background: rgba(125, 211, 252, 0.1);
    border-radius: 3px;
  }

  .modal-body::-webkit-scrollbar-thumb:hover {
    background: rgba(125, 211, 252, 0.2);
  }
</style>
