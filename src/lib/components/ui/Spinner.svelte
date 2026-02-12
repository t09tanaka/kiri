<script lang="ts">
  interface Props {
    size?: 'xs' | 'sm' | 'md' | 'lg';
    color?: string;
  }

  let { size = 'md', color = 'var(--accent-color)' }: Props = $props();

  const sizeMap = {
    xs: { width: 12, stroke: 1.5 },
    sm: { width: 16, stroke: 2 },
    md: { width: 24, stroke: 2.5 },
    lg: { width: 36, stroke: 3 },
  };

  const s = $derived(sizeMap[size]);
</script>

<div
  class="spinner-container"
  class:xs={size === 'xs'}
  class:sm={size === 'sm'}
  class:md={size === 'md'}
  class:lg={size === 'lg'}
>
  <div class="spinner-glow"></div>
  <svg width={s.width} height={s.width} viewBox="0 0 24 24" fill="none" class="spinner">
    <!-- Background circle -->
    <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width={s.stroke} opacity="0.1" />
    <!-- Spinning arc -->
    <circle
      cx="12"
      cy="12"
      r="10"
      stroke={color}
      stroke-width={s.stroke}
      stroke-linecap="round"
      stroke-dasharray="62.83"
      stroke-dashoffset="47.12"
      class="spinner-arc"
    />
  </svg>
</div>

<style>
  .spinner-container {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .spinner-glow {
    position: absolute;
    inset: -4px;
    background: radial-gradient(
      circle,
      rgba(125, 211, 252, 0.08) 0%,
      rgba(196, 181, 253, 0.04) 50%,
      transparent 70%
    );
    border-radius: 50%;
    filter: blur(4px);
    animation: glowPulse 2s ease-in-out infinite;
  }

  .xs .spinner-glow {
    inset: -1px;
    filter: blur(1px);
  }

  .sm .spinner-glow {
    inset: -2px;
    filter: blur(2px);
  }

  .lg .spinner-glow {
    inset: -6px;
    filter: blur(6px);
  }

  @keyframes glowPulse {
    0%,
    100% {
      opacity: 0.4;
      transform: scale(1);
    }
    50% {
      opacity: 0.7;
      transform: scale(1.05);
    }
  }

  .spinner {
    position: relative;
    z-index: 1;
    animation: spinnerRotate 1.4s linear infinite;
  }

  @keyframes spinnerRotate {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }

  .spinner-arc {
    animation: spinnerDash 1.8s ease-in-out infinite;
    filter: drop-shadow(0 0 2px currentColor);
  }

  @keyframes spinnerDash {
    0% {
      stroke-dashoffset: 62.83;
      opacity: 0.7;
    }
    50% {
      stroke-dashoffset: 15.7;
      opacity: 1;
    }
    100% {
      stroke-dashoffset: 62.83;
      opacity: 0.7;
    }
  }
</style>
