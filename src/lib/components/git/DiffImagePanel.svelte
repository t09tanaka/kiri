<script lang="ts">
  import { getImageMimeType } from './diffParser';

  interface Props {
    path: string;
    originalBase64: string | null | undefined;
    currentBase64: string | null | undefined;
  }

  let { path, originalBase64, currentBase64 }: Props = $props();
  const mime = $derived(getImageMimeType(path));
  const hasImage = $derived(Boolean(originalBase64) || Boolean(currentBase64));
</script>

<div class="binary-diff">
  {#if hasImage}
    <div class="image-comparison">
      {#if originalBase64}
        <div class="image-panel original">
          <div class="image-label">Original</div>
          <img src="data:image/{mime};base64,{originalBase64}" alt="Original: {path}" />
        </div>
      {/if}
      {#if currentBase64}
        <div class="image-panel current">
          <div class="image-label">
            {originalBase64 ? 'Current' : 'New'}
          </div>
          <img src="data:image/{mime};base64,{currentBase64}" alt="Current: {path}" />
        </div>
      {/if}
    </div>
  {:else}
    <div class="binary-notice">
      <svg
        width="24"
        height="24"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
        <circle cx="8.5" cy="8.5" r="1.5"></circle>
        <polyline points="21 15 16 10 5 21"></polyline>
      </svg>
      <span>Binary file changed</span>
    </div>
  {/if}
</div>

<style>
  .binary-diff {
    padding: var(--space-4);
  }

  .image-comparison {
    display: flex;
    gap: var(--space-4);
    justify-content: center;
    flex-wrap: wrap;
  }

  .image-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--bg-tertiary);
    max-width: 400px;
  }

  .image-panel.original {
    border: 1px solid var(--git-deleted);
  }

  .image-panel.current {
    border: 1px solid var(--git-added);
  }

  .image-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }

  .image-panel.original .image-label {
    color: var(--git-deleted);
  }

  .image-panel.current .image-label {
    color: var(--git-added);
  }

  .image-panel img {
    max-width: 100%;
    max-height: 300px;
    object-fit: contain;
    border-radius: var(--radius-sm);
    background: repeating-conic-gradient(#808080 0% 25%, transparent 0% 50%) 50% / 16px 16px;
  }

  .binary-notice {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-6);
    color: var(--text-muted);
    font-size: 12px;
  }

  .binary-notice svg {
    opacity: 0.5;
  }
</style>
