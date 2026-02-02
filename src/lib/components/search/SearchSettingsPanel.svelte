<script lang="ts">
  import { contentSearchStore } from '@/lib/stores/contentSearchStore';
  import { DEFAULT_EXCLUDE_PATTERNS } from '@/lib/services/persistenceService';

  const store = $derived($contentSearchStore);

  let newPattern = $state('');
  let inputElement: HTMLInputElement | null = $state(null);

  function handleAddPattern() {
    if (newPattern.trim()) {
      contentSearchStore.addExcludePattern(newPattern.trim());
      newPattern = '';
      inputElement?.focus();
    }
  }

  function handleRemovePattern(pattern: string) {
    contentSearchStore.removeExcludePattern(pattern);
  }

  function handleResetToDefaults() {
    contentSearchStore.resetExcludePatterns();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleAddPattern();
    }
  }

  function isDefaultPattern(pattern: string): boolean {
    return DEFAULT_EXCLUDE_PATTERNS.includes(pattern);
  }
</script>

<div class="settings-panel">
  <div class="settings-header">
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
        <circle cx="12" cy="12" r="3"></circle>
        <path
          d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
        ></path>
      </svg>
      <span class="title">Search Settings</span>
    </div>
    <button
      class="back-btn"
      onclick={() => contentSearchStore.closeSettings()}
      title="Back to search"
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

  <div class="settings-content">
    <div class="settings-section">
      <div class="section-header">
        <h3 class="section-title">Exclude Patterns</h3>
        <span class="section-description">
          Files matching these patterns will be excluded from search results.
        </span>
      </div>

      <div class="add-pattern-form">
        <input
          bind:this={inputElement}
          type="text"
          class="pattern-input"
          placeholder="Add pattern (e.g., *.log, vendor)"
          bind:value={newPattern}
          onkeydown={handleKeyDown}
        />
        <button class="add-btn" onclick={handleAddPattern} disabled={!newPattern.trim()}>
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
          </svg>
          Add
        </button>
      </div>

      <div class="pattern-hint">
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="16" x2="12" y2="12"></line>
          <line x1="12" y1="8" x2="12.01" y2="8"></line>
        </svg>
        <span>
          Use <code>*</code> for wildcards. Examples: <code>*.min.js</code>, <code>vendor</code>,
          <code>*.log</code>
        </span>
      </div>

      <div class="pattern-list">
        {#each store.excludePatterns as pattern (pattern)}
          <div class="pattern-item" class:default={isDefaultPattern(pattern)}>
            <span class="pattern-text">{pattern}</span>
            {#if isDefaultPattern(pattern)}
              <span class="default-badge">default</span>
            {/if}
            <button
              class="remove-btn"
              onclick={() => handleRemovePattern(pattern)}
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
        {:else}
          <div class="no-patterns">
            <span>No exclude patterns configured</span>
          </div>
        {/each}
      </div>

      <div class="section-actions">
        <button class="reset-btn" onclick={handleResetToDefaults}>
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <polyline points="23 4 23 10 17 10"></polyline>
            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
          </svg>
          Reset to Defaults
        </button>
      </div>
    </div>

    <div class="settings-section info-section">
      <h3 class="section-title">Built-in Exclusions</h3>
      <p class="info-text">The following directories are always excluded from search:</p>
      <div class="builtin-list">
        <span class="builtin-item">node_modules</span>
        <span class="builtin-item">target</span>
        <span class="builtin-item">.git</span>
        <span class="builtin-item">dist</span>
        <span class="builtin-item">build</span>
      </div>
    </div>
  </div>
</div>

<style>
  .settings-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
    overflow: hidden;
  }

  .settings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .header-content svg {
    color: var(--accent-color);
  }

  .title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .back-btn {
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

  .back-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .settings-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
  }

  .settings-section {
    margin-bottom: var(--space-6);
  }

  .section-header {
    margin-bottom: var(--space-3);
  }

  .section-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 var(--space-1) 0;
  }

  .section-description {
    font-size: 12px;
    color: var(--text-muted);
  }

  .add-pattern-form {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .pattern-input {
    flex: 1;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-mono);
    outline: none;
    transition: border-color var(--transition-fast);
  }

  .pattern-input:focus {
    border-color: var(--accent-color);
  }

  .pattern-input::placeholder {
    color: var(--text-muted);
    font-family: var(--font-sans);
  }

  .add-btn {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    background: rgba(125, 211, 252, 0.1);
    border: 1px solid rgba(125, 211, 252, 0.3);
    border-radius: var(--radius-md);
    color: var(--accent-color);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .add-btn:hover:not(:disabled) {
    background: rgba(125, 211, 252, 0.2);
  }

  .add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .pattern-hint {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 11px;
    color: var(--text-muted);
    margin-bottom: var(--space-3);
  }

  .pattern-hint code {
    padding: 1px 4px;
    background: var(--bg-tertiary);
    border-radius: var(--radius-xs);
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .pattern-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    max-height: 300px;
    overflow-y: auto;
    padding: var(--space-2);
    background: var(--bg-secondary);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .pattern-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: var(--bg-tertiary);
    border-radius: var(--radius-sm);
    transition: background var(--transition-fast);
  }

  .pattern-item:hover {
    background: var(--bg-elevated);
  }

  .pattern-item.default {
    opacity: 0.8;
  }

  .pattern-text {
    flex: 1;
    font-size: 12px;
    font-family: var(--font-mono);
    color: var(--text-primary);
  }

  .default-badge {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 2px 4px;
    background: rgba(125, 211, 252, 0.1);
    color: var(--accent-color);
    border-radius: var(--radius-xs);
  }

  .remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--radius-xs);
    color: var(--text-muted);
    cursor: pointer;
    opacity: 0;
    transition: all var(--transition-fast);
  }

  .pattern-item:hover .remove-btn {
    opacity: 1;
  }

  .remove-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    color: #f87171;
  }

  .no-patterns {
    padding: var(--space-4);
    text-align: center;
    color: var(--text-muted);
    font-size: 12px;
  }

  .section-actions {
    margin-top: var(--space-3);
  }

  .reset-btn {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .reset-btn:hover {
    background: var(--bg-elevated);
    border-color: var(--border-color);
  }

  .info-section {
    padding: var(--space-3);
    background: var(--bg-secondary);
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
  }

  .info-text {
    font-size: 12px;
    color: var(--text-muted);
    margin: var(--space-2) 0;
  }

  .builtin-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .builtin-item {
    font-size: 11px;
    font-family: var(--font-mono);
    padding: 2px 8px;
    background: var(--bg-tertiary);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
  }
</style>
