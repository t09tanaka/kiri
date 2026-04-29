<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Spinner from '@/lib/components/ui/Spinner.svelte';
  import type { SkillStatus } from '@/lib/services/skillInstallService';

  interface Props {
    status: SkillStatus;
    onAccept: () => void | Promise<void>;
    onDismiss: () => void;
  }

  let { status, onAccept, onDismiss }: Props = $props();

  let mounted = $state(false);
  let installing = $state(false);
  let errorMessage = $state<string | null>(null);

  const title = $derived(
    status.action === 'install' ? 'Claude skill をインストール' : 'Claude skill をアップデート'
  );
  const primaryLabel = $derived(status.action === 'install' ? 'インストール' : 'アップデート');

  async function handleAccept() {
    if (installing) return;
    installing = true;
    errorMessage = null;
    try {
      await onAccept();
    } catch (e) {
      errorMessage = `${e}`;
    } finally {
      installing = false;
    }
  }

  function handleDismiss() {
    if (installing) return;
    onDismiss();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleDismiss();
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      handleDismiss();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      void handleAccept();
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

<div
  class="dialog-backdrop"
  class:mounted
  onclick={handleBackdropClick}
  onkeydown={() => {}}
  role="dialog"
  aria-modal="true"
  aria-labelledby="skill-install-dialog-title"
  tabindex="-1"
>
  <div class="dialog-container">
    <div class="dialog-ambient"></div>

    <div class="dialog-content">
      <!-- Icon area -->
      <div class="dialog-icon-area">
        <div class="icon-container">
          <div class="icon-ring"></div>
          <svg
            class="icon"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M12 2L2 7l10 5 10-5-10-5z"></path>
            <path d="M2 17l10 5 10-5"></path>
            <path d="M2 12l10 5 10-5"></path>
          </svg>
        </div>
      </div>

      <!-- Text content -->
      <div class="dialog-text">
        <h2 id="skill-install-dialog-title" class="dialog-title">{title}</h2>

        <p class="dialog-message">
          {#if status.action === 'install'}
            kiri ターミナル内で <code>kiri</code> コマンドの使い方を Claude に教える skill を
            <code>~/.claude/skills/kiri-cli/</code> にインストールします。
          {:else}
            Claude skill <code>kiri-cli</code> をバージョン {status.installed_version ??
              '未インストール'} から {status.source_version ?? '?'} に更新します。
          {/if}
        </p>

        <div class="install-path-pill">
          <span class="install-path-label">インストール先</span>
          <code class="install-path-value">{status.install_path}</code>
        </div>

        {#if errorMessage}
          <p class="error-message">{errorMessage}</p>
        {/if}
      </div>

      <!-- Action buttons -->
      <div class="dialog-actions">
        <button class="btn btn-ghost" onclick={handleDismiss} disabled={installing}>
          <span class="btn-label">あとで</span>
          <span class="btn-hint">esc</span>
        </button>
        <button class="btn btn-confirm" onclick={handleAccept} disabled={installing}>
          {#if installing}
            <Spinner size="xs" />
            <span class="btn-label">インストール中…</span>
          {:else}
            <span class="btn-label">{primaryLabel}</span>
            <span class="btn-hint">↵</span>
          {/if}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(8, 12, 16, 0.75);
    backdrop-filter: blur(8px) saturate(120%);
    -webkit-backdrop-filter: blur(8px) saturate(120%);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
    opacity: 0;
    transition: opacity 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .dialog-backdrop.mounted {
    opacity: 1;
  }

  .dialog-container {
    position: relative;
    width: 90%;
    max-width: 440px;
    transform: translateY(12px) scale(0.96);
    opacity: 0;
    animation: dialogSlideIn 0.35s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  @keyframes dialogSlideIn {
    to {
      transform: translateY(0) scale(1);
      opacity: 1;
    }
  }

  .dialog-ambient {
    position: absolute;
    inset: -20px;
    background: radial-gradient(
      ellipse 60% 50% at 50% 0%,
      rgba(125, 211, 252, 0.12) 0%,
      transparent 60%
    );
    border-radius: 50%;
    pointer-events: none;
    z-index: -1;
    animation: ambientPulse 4s ease-in-out infinite;
  }

  @keyframes ambientPulse {
    0%,
    100% {
      opacity: 0.8;
      transform: scale(1);
    }
    50% {
      opacity: 1;
      transform: scale(1.05);
    }
  }

  .dialog-content {
    background: linear-gradient(180deg, rgba(26, 32, 41, 0.95) 0%, rgba(19, 25, 32, 0.98) 100%);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(125, 211, 252, 0.08);
    border-radius: var(--radius-lg);
    overflow: hidden;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.3),
      0 8px 40px rgba(0, 0, 0, 0.5),
      inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  .dialog-icon-area {
    display: flex;
    justify-content: center;
    padding: var(--space-5) var(--space-4) var(--space-3);
  }

  .icon-container {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 52px;
    height: 52px;
  }

  .icon-ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent-color), var(--accent2-color));
    opacity: 0.08;
    animation: ringPulse 3s ease-in-out infinite;
  }

  @keyframes ringPulse {
    0%,
    100% {
      transform: scale(1);
      opacity: 0.08;
    }
    50% {
      transform: scale(1.15);
      opacity: 0.04;
    }
  }

  .icon {
    position: relative;
    z-index: 1;
    color: var(--accent-color);
    filter: drop-shadow(0 2px 8px rgba(125, 211, 252, 0.3));
    animation: iconFloat 3s ease-in-out infinite;
  }

  @keyframes iconFloat {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-2px);
    }
  }

  .dialog-text {
    text-align: center;
    padding: 0 var(--space-5) var(--space-4);
  }

  .dialog-title {
    margin: 0 0 var(--space-2);
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 500;
    color: var(--text-primary);
    letter-spacing: 0.02em;
  }

  .dialog-message {
    margin: 0 auto var(--space-3);
    font-size: 13px;
    line-height: 1.7;
    color: var(--text-secondary);
    max-width: 340px;
  }

  .dialog-message code {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent-color);
    background: rgba(125, 211, 252, 0.08);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .install-path-pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    background: rgba(125, 211, 252, 0.05);
    border: 1px solid rgba(125, 211, 252, 0.12);
    border-radius: var(--radius-sm);
    padding: var(--space-1) var(--space-3);
    margin-top: var(--space-2);
    max-width: 100%;
    overflow: hidden;
  }

  .install-path-label {
    font-size: 11px;
    color: var(--text-muted, var(--text-secondary));
    opacity: 0.6;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .install-path-value {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error-message {
    margin: var(--space-3) auto 0;
    font-size: 12px;
    color: #f87171;
    max-width: 340px;
    line-height: 1.5;
  }

  .dialog-actions {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4) var(--space-4);
    justify-content: center;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    min-width: 110px;
    padding: var(--space-2) var(--space-4);
    font-family: var(--font-sans);
    font-size: 13px;
    font-weight: 500;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    overflow: hidden;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-label {
    position: relative;
    z-index: 1;
  }

  .btn-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 400;
    opacity: 0.4;
    margin-left: var(--space-1);
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
  }

  .btn-ghost:not(:disabled):hover {
    background: rgba(125, 211, 252, 0.05);
    color: var(--text-primary);
    border-color: var(--border-glow);
  }

  .btn-ghost:not(:disabled):active {
    transform: scale(0.97);
  }

  .btn-confirm {
    background: linear-gradient(
      135deg,
      rgba(125, 211, 252, 0.15) 0%,
      rgba(196, 181, 253, 0.15) 100%
    );
    color: var(--accent-color);
    border: 1px solid rgba(125, 211, 252, 0.2);
  }

  .btn-confirm::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(125, 211, 252, 0.2) 0%, rgba(196, 181, 253, 0.2) 100%);
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .btn-confirm:not(:disabled):hover::before {
    opacity: 1;
  }

  .btn-confirm:not(:disabled):hover {
    border-color: rgba(125, 211, 252, 0.35);
    box-shadow: 0 0 20px rgba(125, 211, 252, 0.15);
  }

  .btn-confirm:not(:disabled):active {
    transform: scale(0.97);
  }

  .btn:focus {
    outline: none;
  }

  .btn:focus-visible {
    box-shadow:
      0 0 0 2px var(--bg-primary),
      0 0 0 4px var(--accent-color);
  }
</style>
