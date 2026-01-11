<script lang="ts">
  interface Props {
    width?: string;
    height?: string;
    borderRadius?: string;
    variant?: 'default' | 'text' | 'circular';
  }

  let { width = '100%', height = '16px', borderRadius, variant = 'default' }: Props = $props();

  const getRadius = () => {
    if (borderRadius) return borderRadius;
    if (variant === 'circular') return '50%';
    if (variant === 'text') return 'var(--radius-sm)';
    return 'var(--radius-md)';
  };
</script>

<div
  class="skeleton"
  style="
    width: {width};
    height: {height};
    border-radius: {getRadius()};
  "
>
  <div class="shimmer"></div>
</div>

<style>
  .skeleton {
    position: relative;
    overflow: hidden;
    background: linear-gradient(
      90deg,
      var(--bg-tertiary) 0%,
      rgba(125, 211, 252, 0.04) 50%,
      var(--bg-tertiary) 100%
    );
    background-size: 200% 100%;
    animation: skeletonPulse 2.5s ease-in-out infinite;
    border: 1px solid rgba(125, 211, 252, 0.03);
  }

  @keyframes skeletonPulse {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  .shimmer {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(125, 211, 252, 0.06) 40%,
      rgba(196, 181, 253, 0.04) 60%,
      transparent 100%
    );
    animation: shimmerMove 2.5s ease-in-out infinite;
    filter: blur(1px);
  }

  @keyframes shimmerMove {
    0% {
      transform: translateX(-100%);
      opacity: 0.6;
    }
    50% {
      opacity: 1;
    }
    100% {
      transform: translateX(100%);
      opacity: 0.6;
    }
  }
</style>
