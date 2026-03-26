<script lang="ts">
  import type { TerminalShortcut, ShortcutType } from '@/lib/stores/shortcutStore.svelte';
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    open: boolean;
    shortcuts: TerminalShortcut[];
    onClose: () => void;
    onAdd: (label: string, text: string, type: ShortcutType) => void;
    onUpdate: (id: string, label: string, text: string) => void;
    onRemove: (id: string) => void;
  }

  let { open, shortcuts, onClose, onAdd, onUpdate, onRemove }: Props = $props();

  let mounted = $state(false);

  // Derived sections
  const replyShortcuts = $derived(shortcuts.filter((s) => s.type === 'reply'));
  const commandShortcuts = $derived(shortcuts.filter((s) => s.type === 'command'));

  // New shortcut form (per section)
  let newReplyLabel = $state('');
  let newReplyText = $state('');
  let newCmdLabel = $state('');
  let newCmdText = $state('');

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

  function handleAddReply() {
    if (newReplyLabel.trim() && newReplyText.trim()) {
      onAdd(newReplyLabel.trim(), newReplyText.trim(), 'reply');
      newReplyLabel = '';
      newReplyText = '';
    }
  }

  function ensureSlashPrefix(text: string): string {
    return text.startsWith('/') ? text : `/${text}`;
  }

  function handleAddCommand() {
    if (newCmdLabel.trim() && newCmdText.trim()) {
      onAdd(newCmdLabel.trim(), ensureSlashPrefix(newCmdText.trim()), 'command');
      newCmdLabel = '';
      newCmdText = '';
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

  function handleAddReplyKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' && newReplyLabel.trim() && newReplyText.trim()) {
      e.preventDefault();
      handleAddReply();
    }
  }

  function handleAddCmdKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' && newCmdLabel.trim() && newCmdText.trim()) {
      e.preventDefault();
      handleAddCommand();
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

{#snippet shortcutRow(shortcut: TerminalShortcut)}
  <div
    class="shortcut-row"
    class:builtin={shortcut.builtin}
    class:editing={editingId === shortcut.id}
  >
    {#if editingId === shortcut.id}
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
        <button class="action-btn save-btn" onclick={saveEdit} title="Save" aria-label="Save">
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
            <path d="M18 6L6 18" /><path d="M6 6l12 12" />
          </svg>
        </button>
      </div>
    {:else}
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
{/snippet}

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
          <!-- Reply Section -->
          <div class="section section-reply">
            <div class="section-header">
              <span class="section-badge reply">REPLY</span>
              <span class="section-desc">Quick phrases</span>
            </div>

            <div class="shortcut-list-items">
              {#each replyShortcuts as shortcut (shortcut.id)}
                {@render shortcutRow(shortcut)}
              {/each}

              <!-- Add reply -->
              <div class="shortcut-row add-row">
                <input
                  class="edit-input"
                  type="text"
                  bind:value={newReplyLabel}
                  placeholder="Label"
                  onkeydown={handleAddReplyKeyDown}
                  spellcheck="false"
                  autocomplete="off"
                  autocorrect="off"
                  autocapitalize="off"
                />
                <input
                  class="edit-input text-input"
                  type="text"
                  bind:value={newReplyText}
                  placeholder="Text to send"
                  onkeydown={handleAddReplyKeyDown}
                  spellcheck="false"
                  autocomplete="off"
                  autocorrect="off"
                  autocapitalize="off"
                />
                <div class="row-actions">
                  <button
                    class="action-btn add-btn"
                    onclick={handleAddReply}
                    disabled={!newReplyLabel.trim() || !newReplyText.trim()}
                    title="Add reply"
                    aria-label="Add reply"
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
          </div>

          <!-- Command Section -->
          <div class="section section-command">
            <div class="section-header">
              <span class="section-badge command">CMD</span>
              <span class="section-desc">Slash commands</span>
            </div>

            <div class="shortcut-list-items">
              {#each commandShortcuts as shortcut (shortcut.id)}
                {@render shortcutRow(shortcut)}
              {/each}

              {#if commandShortcuts.length === 0}
                <div class="empty-hint">No commands yet</div>
              {/if}

              <!-- Add command -->
              <div class="shortcut-row add-row">
                <input
                  class="edit-input"
                  type="text"
                  bind:value={newCmdLabel}
                  placeholder="Label"
                  onkeydown={handleAddCmdKeyDown}
                  spellcheck="false"
                  autocomplete="off"
                  autocorrect="off"
                  autocapitalize="off"
                />
                <input
                  class="edit-input text-input"
                  type="text"
                  bind:value={newCmdText}
                  placeholder="Command to send"
                  onkeydown={handleAddCmdKeyDown}
                  spellcheck="false"
                  autocomplete="off"
                  autocorrect="off"
                  autocapitalize="off"
                />
                <div class="row-actions">
                  <button
                    class="action-btn add-btn"
                    onclick={handleAddCommand}
                    disabled={!newCmdLabel.trim() || !newCmdText.trim()}
                    title="Add command"
                    aria-label="Add command"
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

  .section-reply .shortcut-list-items {
    border-left: 2px solid rgba(125, 211, 252, 0.3);
    margin-left: var(--space-2);
    padding-left: var(--space-2);
  }

  .section-command .shortcut-list-items {
    border-left: 2px solid rgba(196, 181, 253, 0.3);
    margin-left: var(--space-2);
    padding-left: var(--space-2);
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

  .section {
    margin-bottom: var(--space-5);
  }

  .section:last-child {
    margin-bottom: 0;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
    padding: 0 var(--space-2);
  }

  .section-badge {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    padding: 2px 8px;
    border-radius: 6px;
    user-select: none;
  }

  .section-badge.reply {
    color: var(--accent-color);
    background: rgba(125, 211, 252, 0.1);
    border: 1px solid rgba(125, 211, 252, 0.25);
  }

  .section-badge.command {
    color: var(--accent2-color, #c4b5fd);
    background: rgba(196, 181, 253, 0.1);
    border: 1px solid rgba(196, 181, 253, 0.25);
  }

  .section-desc {
    font-size: 11px;
    color: var(--text-muted);
  }

  .empty-hint {
    font-size: 11px;
    color: var(--text-muted);
    padding: var(--space-2) var(--space-2);
    text-align: center;
    opacity: 0.6;
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
