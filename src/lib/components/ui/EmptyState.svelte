<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Headline text describing the empty state. */
    title: string;
    /** Secondary explanatory text. Optional. */
    hint?: string;
    /** Optional icon snippet — usually an inline svg. */
    icon?: Snippet;
    /** Tone of the surrounding mist animation. */
    tone?: 'mist' | 'warning';
  }

  let { title, hint, icon, tone = 'mist' }: Props = $props();
</script>

<div class="empty-state" class:warning={tone === 'warning'}>
  <div class="mist mist-1"></div>
  <div class="mist mist-2"></div>
  {#if icon}
    <div class="icon">
      {@render icon()}
    </div>
  {/if}
  <span class="title">{title}</span>
  {#if hint}
    <span class="hint">{hint}</span>
  {/if}
</div>

<style>
  .empty-state {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-6) var(--space-4);
    text-align: center;
    overflow: hidden;
  }

  .mist {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: radial-gradient(
      ellipse 60% 40% at 50% 50%,
      rgba(125, 211, 252, 0.06) 0%,
      transparent 70%
    );
    opacity: 0.5;
    animation: mistDrift 9s ease-in-out infinite;
  }

  .mist-2 {
    background: radial-gradient(
      ellipse 50% 30% at 30% 60%,
      rgba(196, 181, 253, 0.05) 0%,
      transparent 70%
    );
    animation-duration: 12s;
    animation-direction: reverse;
    opacity: 0.4;
  }

  .empty-state.warning .mist {
    background: radial-gradient(
      ellipse 60% 40% at 50% 50%,
      rgba(248, 113, 113, 0.05) 0%,
      transparent 70%
    );
  }

  @keyframes mistDrift {
    0%,
    100% {
      transform: translate(-4%, 2%) scale(1.05);
      opacity: 0.35;
    }
    50% {
      transform: translate(4%, -2%) scale(1);
      opacity: 0.55;
    }
  }

  .icon {
    position: relative;
    color: var(--accent-color);
    opacity: 0.4;
    margin-bottom: var(--space-2);
    animation: iconFloat 4s ease-in-out infinite;
  }

  .empty-state.warning .icon {
    color: var(--git-deleted);
  }

  @keyframes iconFloat {
    0%,
    100% {
      transform: translateY(0);
      opacity: 0.4;
    }
    50% {
      transform: translateY(-3px);
      opacity: 0.55;
    }
  }

  .title {
    position: relative;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .hint {
    position: relative;
    font-size: 12px;
    color: var(--text-muted);
    opacity: 0.85;
  }
</style>
