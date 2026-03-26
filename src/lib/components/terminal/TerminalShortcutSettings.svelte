<script lang="ts">
  import type { TerminalShortcut } from '@/lib/stores/shortcutStore.svelte';
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    open: boolean;
    shortcuts: TerminalShortcut[];
    onClose: () => void;
    onAdd: (label: string, text: string) => void;
    onUpdate: (id: string, label: string, text: string) => void;
    onRemove: (id: string) => void;
  }

  let { open, shortcuts, onClose, onAdd, onUpdate, onRemove }: Props = $props();

  let mounted = $state(false);

  // New shortcut form
  let newLabel = $state('');
  let newText = $state('');

  // Edit state
  let editingId = $state<string | null>(null);
  let editLabel = $state('');
  let editText = $state('');

  function startEdit(shortcut: TerminalShortcut) {
    editingId = shortcut.id;
    editLabel = shortcut.label;
    editText = shortcut.text;
  }

  function cancelEdit() {
    editingId = null;
    editLabel = '';
    editText = '';
  }

  function saveEdit() {
    if (editingId && editLabel.trim() && editText.trim()) {
      onUpdate(editingId, editLabel.trim(), editText.trim());
      cancelEdit();
    }
  }

  function handleAdd() {
    if (newLabel.trim() && newText.trim()) {
      onAdd(newLabel.trim(), newText.trim());
      newLabel = '';
      newText = '';
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!open) return;

    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      if (editingId) {
        cancelEdit();
      } else {
        onClose();
      }
    } else if (e.key === 'Enter') {
      if (editingId) {
        e.preventDefault();
        saveEdit();
      }
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function handleAddKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' && newLabel.trim() && newText.trim()) {
      e.preventDefault();
      handleAdd();
    }
  }

  onMount(() => {
    mounted = true;
    document.addEventListener('keydown', handleKeyDown, true);
  });

  onDestroy(() => {
    document.removeEventListener('keydown', handleKeyDown, true);
  });
</script>

{#if open}
  <div
    class="modal-backdrop"
    class:mounted
    onclick={handleBackdropClick}
    onkeydown={() => {}}
    role="dialog"
    aria-modal="true"
    aria-labelledby="shortcut-settings-title"
    tabindex="-1"
  >
    <div class="modal-container">
      <!-- Glow effect -->
      <div class="modal-glow"></div>

      <div class="modal-content">
        <!-- Shine line -->
        <div class="modal-shine"></div>

        <!-- Header -->
        <div class="modal-header">
          <h2 id="shortcut-settings-title" class="modal-title">Shortcut Settings</h2>
          <button class="close-btn" onclick={onClose} aria-label="Close">
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
              <path d="M18 6L6 18" />
              <path d="M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Shortcut list -->
        <div class="shortcut-list">
          {#each shortcuts as shortcut (shortcut.id)}
            <div
              class="shortcut-row"
              class:builtin={shortcut.builtin}
              class:editing={editingId === shortcut.id}
            >
              {#if editingId === shortcut.id}
                <!-- Edit mode -->
                <input
                  class="edit-input"
                  type="text"
                  bind:value={editLabel}
                  placeholder="Label"
                  spellcheck="false"
                  autocomplete="off"
                  autocorrect="off"
                  autocapitalize="off"
                />
                <input
                  class="edit-input text-input"
                  type="text"
                  bind:value={editText}
                  placeholder="Text to send"
                  spellcheck="false"
                  autocomplete="off"
                  autocorrect="off"
                  autocapitalize="off"
                />
                <div class="row-actions">
                  <button
                    class="action-btn save-btn"
                    onclick={saveEdit}
                    title="Save"
                    aria-label="Save"
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
                      <polyline points="20 6 9 17 4 12" />
                    </svg>
                  </button>
                  <button
                    class="action-btn cancel-btn"
                    onclick={cancelEdit}
                    title="Cancel"
                    aria-label="Cancel edit"
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
                      <path d="M18 6L6 18" />
                      <path d="M6 6l12 12" />
                    </svg>
                  </button>
                </div>
              {:else}
                <!-- Display mode -->
                <span class="shortcut-label">{shortcut.label}</span>
                <span class="shortcut-text">{shortcut.text}</span>
                <div class="row-actions">
                  {#if shortcut.builtin}
                    <span class="builtin-badge">built-in</span>
                  {:else}
                    <button
                      class="action-btn edit-btn"
                      onclick={() => startEdit(shortcut)}
                      title="Edit"
                      aria-label="Edit shortcut"
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
                        <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
                        <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
                      </svg>
                    </button>
                    <button
                      class="action-btn delete-btn"
                      onclick={() => onRemove(shortcut.id)}
                      title="Delete"
                      aria-label="Delete shortcut"
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
                        <polyline points="3 6 5 6 21 6" />
                        <path
                          d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                        />
                      </svg>
                    </button>
                  {/if}
                </div>
              {/if}
            </div>
          {/each}

          <!-- Add new shortcut row -->
          <div class="shortcut-row add-row">
            <input
              class="edit-input"
              type="text"
              bind:value={newLabel}
              placeholder="Label"
              onkeydown={handleAddKeyDown}
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
            />
            <input
              class="edit-input text-input"
              type="text"
              bind:value={newText}
              placeholder="Text to send"
              onkeydown={handleAddKeyDown}
              spellcheck="false"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
            />
            <div class="row-actions">
              <button
                class="action-btn add-btn"
                onclick={handleAdd}
                disabled={!newLabel.trim() || !newText.trim()}
                title="Add shortcut"
                aria-label="Add shortcut"
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
                  <line x1="12" y1="5" x2="12" y2="19" />
                  <line x1="5" y1="12" x2="19" y2="12" />
                </svg>
              </button>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="modal-footer">
          <span class="footer-item">
            <kbd>Shift+Click</kbd>
            <span>input only</span>
          </span>
          <span class="footer-item">
            <kbd>Esc</kbd>
            <span>close</span>
          </span>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(8, 12, 16, 0.7);
    backdrop-filter: blur(8px) saturate(120%);
    -webkit-backdrop-filter: blur(8px) saturate(120%);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
    opacity: 0;
    transition: opacity 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .modal-backdrop.mounted {
    opacity: 1;
  }

  .modal-container {
    position: relative;
    width: 90%;
    max-width: 480px;
    max-height: 80vh;
    transform: translateY(-20px) scale(0.95);
    opacity: 0;
    animation: modalSlideIn 0.3s cubic-bezier(0.16, 1, 0.3, 1) forwards;
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

  /* Glow effect */
  .modal-glow {
    position: absolute;
    inset: -2px;
    background: linear-gradient(135deg, var(--gradient-start), var(--gradient-end));
    border-radius: calc(var(--radius-xl) + 2px);
    opacity: 0.06;
    filter: blur(5px);
    pointer-events: none;
  }

  .modal-content {
    position: relative;
    background: linear-gradient(180deg, rgba(26, 32, 41, 0.95) 0%, rgba(19, 25, 32, 0.98) 100%);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(125, 211, 252, 0.08);
    border-radius: var(--radius-xl);
    overflow: hidden;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.3),
      0 8px 40px rgba(0, 0, 0, 0.5),
      inset 0 1px 0 rgba(255, 255, 255, 0.03);
    display: flex;
    flex-direction: column;
    max-height: 80vh;
  }

  /* Shine line */
  .modal-shine {
    position: absolute;
    top: 0;
    left: 10%;
    right: 10%;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent-color), transparent);
    opacity: 0.6;
    pointer-events: none;
  }

  /* Header */
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--border-subtle);
  }

  .modal-title {
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 500;
    color: var(--text-primary);
    letter-spacing: 0.02em;
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    color: var(--text-muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition:
      color var(--transition-fast),
      background var(--transition-fast);
  }

  .close-btn:hover {
    color: var(--text-secondary);
    background: rgba(125, 211, 252, 0.08);
  }

  /* Shortcut list */
  .shortcut-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-3) var(--space-4);
    scrollbar-width: thin;
    scrollbar-color: var(--border-color) transparent;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-2);
    border-radius: var(--radius-sm);
    transition: background var(--transition-fast);
  }

  .shortcut-row:hover {
    background: rgba(125, 211, 252, 0.04);
  }

  .shortcut-row.builtin {
    opacity: 0.6;
  }

  .shortcut-row.editing {
    background: rgba(125, 211, 252, 0.06);
  }

  .shortcut-label {
    flex-shrink: 0;
    min-width: 80px;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .shortcut-text {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .builtin-badge {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-muted);
    padding: 1px 6px;
    border: 1px solid var(--border-color);
    border-radius: 8px;
  }

  /* Edit inputs */
  .edit-input {
    flex-shrink: 0;
    min-width: 80px;
    max-width: 120px;
    padding: 3px 8px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
    background: rgba(125, 211, 252, 0.06);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-xs);
    outline: none;
    transition:
      border-color var(--transition-fast),
      box-shadow var(--transition-fast);
  }

  .edit-input:focus {
    border-color: var(--border-glow);
    box-shadow: 0 0 8px rgba(125, 211, 252, 0.1);
  }

  .edit-input.text-input {
    flex: 1;
    max-width: none;
  }

  /* Row actions */
  .row-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-shrink: 0;
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    color: var(--text-muted);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-xs);
    cursor: pointer;
    transition:
      color var(--transition-fast),
      background var(--transition-fast),
      border-color var(--transition-fast);
  }

  .action-btn:hover {
    color: var(--text-secondary);
    background: rgba(125, 211, 252, 0.08);
    border-color: var(--border-color);
  }

  .action-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .action-btn:disabled:hover {
    background: transparent;
    border-color: transparent;
  }

  .save-btn:hover {
    color: var(--git-added);
  }

  .delete-btn:hover {
    color: var(--git-deleted);
  }

  .add-btn:hover:not(:disabled) {
    color: var(--accent-color);
  }

  .add-row {
    margin-top: var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border-subtle);
  }

  /* Footer */
  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    border-top: 1px solid var(--border-subtle);
  }

  .footer-item {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: 11px;
    color: var(--text-muted);
  }

  .footer-item kbd {
    display: inline-flex;
    align-items: center;
    padding: 1px 5px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
    background: rgba(125, 211, 252, 0.06);
    border: 1px solid var(--border-color);
    border-radius: 4px;
  }
</style>
