<script lang="ts">
  interface Suggestion {
    text: string;
    kind: 'command' | 'history' | 'path';
  }

  interface Props {
    suggestion: Suggestion | null;
    currentInput: string;
    visible: boolean;
    cursorPosition: { x: number; y: number } | null;
  }

  let {
    suggestion = null,
    currentInput = '',
    visible = false,
    cursorPosition = null,
  }: Props = $props();

  // Calculate the ghost text (remaining part of suggestion)
  const ghostText = $derived(() => {
    if (!suggestion || !visible || !currentInput) return '';

    const suggestionText = suggestion.text;

    // For path completion, only show the remaining part
    if (suggestion.kind === 'path') {
      const parts = currentInput.split(/\s+/);
      const lastPart = parts[parts.length - 1];
      if (suggestionText.toLowerCase().startsWith(lastPart.toLowerCase())) {
        return suggestionText.slice(lastPart.length);
      }
    }

    // For command completion, show remaining part of the command
    if (suggestionText.toLowerCase().startsWith(currentInput.toLowerCase())) {
      return suggestionText.slice(currentInput.length);
    }

    return '';
  });
</script>

{#if visible && cursorPosition && ghostText()}
  <div class="ghost-overlay" style="left: {cursorPosition.x}px; top: {cursorPosition.y}px;">
    <span class="ghost-text">{ghostText()}</span>
    <span class="ghost-hint">Tab</span>
  </div>
{/if}

<style>
  .ghost-overlay {
    position: absolute;
    display: flex;
    align-items: center;
    gap: 8px;
    pointer-events: none;
    z-index: 100;
  }

  .ghost-text {
    color: rgba(125, 211, 252, 0.35);
    font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', 'Menlo', monospace;
    font-size: 13px;
    line-height: 1.4;
    white-space: pre;
  }

  .ghost-hint {
    padding: 1px 4px;
    background: rgba(125, 211, 252, 0.1);
    border-radius: 3px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 9px;
    color: rgba(125, 211, 252, 0.4);
  }
</style>
