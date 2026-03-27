<script lang="ts">
  import type { InputRecord } from '@/lib/services/persistenceService';

  interface Props {
    suggestions: InputRecord[];
    onAdd: (suggestion: InputRecord) => void;
    onDismiss: (suggestion: InputRecord) => void;
  }

  let { suggestions, onAdd, onDismiss }: Props = $props();

  let open = $state(false);

  function togglePopover() {
    open = !open;
  }
</script>

{#if suggestions.length > 0}
  <div class="suggestion-wrapper">
    <button class="suggestion-badge" onclick={togglePopover} title="Shortcut suggestions">
      +{suggestions.length}
    </button>

    {#if open}
      <div class="suggestion-popover">
        <div class="popover-header">Suggestions</div>
        <ul class="suggestion-list">
          {#each suggestions as suggestion (suggestion.text)}
            <li class="suggestion-item">
              <span class="suggestion-text">{suggestion.rawText}</span>
              <span class="suggestion-count">{suggestion.count}</span>
              <button
                class="suggestion-add-btn"
                onclick={() => onAdd(suggestion)}
                title="Add as shortcut"
                aria-label="Add {suggestion.rawText} as shortcut"
              >
                +
              </button>
              <button
                class="suggestion-dismiss-btn"
                onclick={() => onDismiss(suggestion)}
                title="Dismiss suggestion"
                aria-label="Dismiss {suggestion.rawText}"
              >
                ×
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>
{/if}

<style>
  .suggestion-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .suggestion-badge {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 10px;
    cursor: pointer;
    color: var(--accent-color, #7dd3fc);
    background: rgba(125, 211, 252, 0.12);
    border: 1px solid rgba(125, 211, 252, 0.35);
    box-shadow: 0 0 8px rgba(125, 211, 252, 0.15);
    transition:
      background var(--transition-fast, 180ms ease),
      border-color var(--transition-fast, 180ms ease),
      box-shadow var(--transition-fast, 180ms ease);
    user-select: none;
  }

  .suggestion-badge:hover {
    background: rgba(125, 211, 252, 0.2);
    border-color: rgba(125, 211, 252, 0.55);
    box-shadow: 0 0 14px rgba(125, 211, 252, 0.28);
  }

  .suggestion-badge:active {
    transform: scale(0.94);
  }

  .suggestion-popover {
    position: absolute;
    bottom: calc(100% + 8px);
    right: 0;
    min-width: 220px;
    background: rgba(13, 17, 23, 0.92);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border: 1px solid rgba(125, 211, 252, 0.2);
    border-radius: 12px;
    padding: 10px;
    z-index: 100;
    animation: slideIn 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .popover-header {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: rgba(125, 211, 252, 0.6);
    margin-bottom: 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid rgba(125, 211, 252, 0.12);
  }

  .suggestion-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .suggestion-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px;
    border-radius: 6px;
    transition: background var(--transition-fast, 180ms ease);
  }

  .suggestion-item:hover {
    background: rgba(125, 211, 252, 0.06);
  }

  .suggestion-text {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary, #e6edf3);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .suggestion-count {
    font-family: var(--font-mono);
    font-size: 10px;
    color: rgba(125, 211, 252, 0.5);
    flex-shrink: 0;
  }

  .suggestion-add-btn,
  .suggestion-dismiss-btn {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    font-size: 14px;
    line-height: 1;
    border-radius: 4px;
    cursor: pointer;
    transition:
      color var(--transition-fast, 180ms ease),
      background var(--transition-fast, 180ms ease);
  }

  .suggestion-add-btn {
    color: rgba(125, 211, 252, 0.5);
    background: transparent;
    border: 1px solid rgba(125, 211, 252, 0.2);
  }

  .suggestion-add-btn:hover {
    color: var(--accent-color, #7dd3fc);
    background: rgba(125, 211, 252, 0.12);
    border-color: rgba(125, 211, 252, 0.4);
  }

  .suggestion-dismiss-btn {
    color: rgba(230, 237, 243, 0.25);
    background: transparent;
    border: 1px solid rgba(230, 237, 243, 0.1);
  }

  .suggestion-dismiss-btn:hover {
    color: rgba(230, 237, 243, 0.6);
    background: rgba(230, 237, 243, 0.06);
    border-color: rgba(230, 237, 243, 0.2);
  }

  .suggestion-add-btn:active,
  .suggestion-dismiss-btn:active {
    transform: scale(0.9);
  }
</style>
