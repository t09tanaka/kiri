<script lang="ts">
  import Editor from './Editor.svelte';
  import { editorModalStore } from '@/lib/stores/editorModalStore';
  import { dialogService } from '@/lib/services/dialogService';
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    filePath: string;
    onClose: () => void;
  }

  let { filePath, onClose }: Props = $props();

  let mounted = $state(false);

  function getFileName(path: string): string {
    return path.split('/').pop() || path;
  }

  function handleModifiedChange(modified: boolean) {
    editorModalStore.setModified(modified);
  }

  function handleSave() {
    // Trigger save via keyboard event dispatch to Editor
    const event = new KeyboardEvent('keydown', {
      key: 's',
      metaKey: true,
      bubbles: true,
    });
    document.dispatchEvent(event);
  }

  function handleEditorSave() {
    // Called when editor successfully saves
    editorModalStore.setModified(false);
  }

  async function handleClose() {
    if ($editorModalStore.modified) {
      const result = await showUnsavedChangesDialog();
      if (result === 'cancel') {
        return;
      }
      if (result === 'save') {
        handleSave();
        // Wait a bit for save to complete, then close
        setTimeout(() => {
          onClose();
        }, 100);
        return;
      }
      // result === 'discard'
    }
    onClose();
  }

  async function showUnsavedChangesDialog(): Promise<'save' | 'discard' | 'cancel'> {
    // Use a simple confirm dialog for now
    // In a real implementation, you might want a custom dialog with 3 options
    const confirmed = await dialogService.confirm(
      'You have unsaved changes. Do you want to save before closing?',
      {
        title: 'Unsaved Changes',
        okLabel: 'Save and Close',
        cancelLabel: 'Discard Changes',
      }
    );

    if (confirmed) {
      return 'save';
    }
    return 'discard';
  }

  function handleKeyDown(e: KeyboardEvent) {
    // Escape to close (with unsaved check)
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      handleClose();
    }
    // Cmd+S to save - let it bubble to Editor component
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleClose();
    }
  }

  onMount(() => {
    mounted = true;
    // Use capture phase to intercept before other handlers
    document.addEventListener('keydown', handleKeyDown, true);
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
  });
</script>

<div
  class="editor-backdrop"
  class:mounted
  onclick={handleBackdropClick}
  onkeydown={() => {}}
  role="button"
  tabindex="-1"
>
  <div class="editor-modal">
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
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
            <polyline points="14 2 14 8 20 8"></polyline>
          </svg>
          <span class="title">{getFileName(filePath)}</span>
          {#if $editorModalStore.modified}
            <span class="modified-indicator" title="Unsaved changes"></span>
          {/if}
          <span class="file-path">{filePath}</span>
        </div>
        <div class="header-actions">
          <button
            class="action-btn save-btn"
            onclick={handleSave}
            disabled={!$editorModalStore.modified}
            title="Save (Cmd+S)"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
              <polyline points="17 21 17 13 7 13 7 21"></polyline>
              <polyline points="7 3 7 8 15 8"></polyline>
            </svg>
          </button>
          <button class="action-btn close-btn" onclick={handleClose} title="Close (Esc)">
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

      <div class="modal-body">
        <Editor {filePath} onSave={handleEditorSave} onModifiedChange={handleModifiedChange} />
      </div>

      <div class="modal-footer">
        <span class="footer-item">
          <kbd>Esc</kbd>
          <span>close</span>
        </span>
        <span class="footer-item">
          <kbd>Cmd+S</kbd>
          <span>save</span>
        </span>
      </div>
    </div>
  </div>
</div>

<style>
  .editor-backdrop {
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

  .editor-backdrop.mounted {
    opacity: 1;
  }

  .editor-modal {
    position: relative;
    width: 90%;
    max-width: 1200px;
    height: 85%;
    max-height: 900px;
    min-height: 400px;
    opacity: 1;
    transform: translateY(0) scale(1);
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

  .editor-modal:hover .modal-glow {
    opacity: 0.1;
  }

  .modal-content {
    display: flex;
    flex-direction: column;
    height: 100%;
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
    overflow: hidden;
  }

  .header-content svg {
    color: var(--accent-color);
    opacity: 0.8;
    flex-shrink: 0;
  }

  .title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.02em;
  }

  .modified-indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-color);
    flex-shrink: 0;
    animation: modifiedPulse 2s ease-in-out infinite;
  }

  @keyframes modifiedPulse {
    0%,
    100% {
      opacity: 0.6;
      transform: scale(1);
    }
    50% {
      opacity: 1;
      transform: scale(1.15);
    }
  }

  .file-path {
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    padding: 2px 6px;
    background: var(--bg-elevated);
    border-radius: var(--radius-sm);
    margin-left: var(--space-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
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

  .action-btn:hover:not(:disabled) {
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
  }

  .action-btn:disabled {
    cursor: not-allowed;
    opacity: 0.3;
  }

  .action-btn.save-btn:not(:disabled):hover {
    background: rgba(74, 222, 128, 0.1);
    color: #4ade80;
  }

  .action-btn.close-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .modal-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
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
  }

  .footer-item span {
    margin-left: 2px;
  }
</style>
