<script lang="ts">
  import { toastStore } from '@/lib/stores/toastStore';
  import Toast from './Toast.svelte';
</script>

<div class="toast-container">
  {#each $toastStore as toast (toast.id)}
    <Toast
      message={toast.message}
      type={toast.type}
      duration={toast.duration}
      onClose={() => toastStore.remove(toast.id)}
    />
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    top: var(--space-4);
    right: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    z-index: 9999;
    pointer-events: none;
  }

  .toast-container :global(.toast) {
    pointer-events: auto;
  }
</style>
