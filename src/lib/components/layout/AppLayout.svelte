<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { appStore } from '@/lib/stores/appStore';
  import { tabStore } from '@/lib/stores/tabStore';
  import { editorModalStore } from '@/lib/stores/editorModalStore';
  import { currentProjectPath } from '@/lib/stores/projectStore';
  import { confirmDialogStore } from '@/lib/stores/confirmDialogStore';
  import Sidebar from '@/lib/components/layout/Sidebar.svelte';
  import MainContent from '@/lib/components/layout/MainContent.svelte';
  import StatusBar from '@/lib/components/layout/StatusBar.svelte';
  import KeyboardShortcuts from '@/lib/components/ui/KeyboardShortcuts.svelte';
  import ConfirmDialog from '@/lib/components/ui/ConfirmDialog.svelte';

  let isResizing = $state(false);
  let sidebarWidth = $state($appStore.sidebarWidth);
  let showShortcuts = $state(false);
  let isWindowFocused = $state(true);

  function handleFileSelect(path: string) {
    editorModalStore.open(path);
  }

  async function handleKeyDown(e: KeyboardEvent) {
    // Skip if typing in an input
    const target = e.target as HTMLElement;
    const isTyping =
      target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable;

    // Ctrl/Cmd + ` to add new terminal
    if ((e.ctrlKey || e.metaKey) && e.key === '`') {
      e.preventDefault();
      tabStore.addTerminalTab();
    }
    // Ctrl/Cmd + W to close current tab
    if ((e.ctrlKey || e.metaKey) && e.key === 'w') {
      e.preventDefault();
      const activeId = $tabStore.activeTabId;
      if (activeId) {
        const activeTab = $tabStore.tabs.find((t) => t.id === activeId);
        // Show confirmation dialog for terminal tabs
        if (activeTab?.type === 'terminal') {
          const confirmed = await confirmDialogStore.confirm({
            title: 'Close Terminal',
            message:
              'Are you sure you want to close this terminal? Any running processes will be terminated.',
            confirmLabel: 'Close',
            cancelLabel: 'Cancel',
            kind: 'warning',
          });
          if (!confirmed) {
            return;
          }
        }
        tabStore.closeTab(activeId);
      }
    }
    // Ctrl/Cmd + B to toggle sidebar
    if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
      e.preventDefault();
      appStore.toggleSidebar();
    }
    // ? to show keyboard shortcuts (only when not typing)
    if (e.key === '?' && !isTyping && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      showShortcuts = true;
    }
    // Ctrl/Cmd + / to show keyboard shortcuts
    if ((e.ctrlKey || e.metaKey) && e.key === '/') {
      e.preventDefault();
      showShortcuts = !showShortcuts;
    }
    // Number keys to switch tabs
    if ((e.ctrlKey || e.metaKey) && /^[1-9]$/.test(e.key)) {
      e.preventDefault();
      const index = parseInt(e.key) - 1;
      const tabs = $tabStore.tabs;
      if (index < tabs.length) {
        tabStore.setActiveTab(tabs[index].id);
      }
    }
  }

  function startResize(e: MouseEvent) {
    e.preventDefault();
    isResizing = true;
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', stopResize);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isResizing) return;

    const minWidth = 160;
    const maxWidth = 400;
    const newWidth = Math.min(Math.max(e.clientX, minWidth), maxWidth);
    sidebarWidth = newWidth;
    appStore.setSidebarWidth(newWidth);
  }

  function stopResize() {
    isResizing = false;
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', stopResize);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }

  // Keep sidebarWidth in sync with store
  $effect(() => {
    if (!isResizing) {
      sidebarWidth = $appStore.sidebarWidth;
    }
  });

  function handleWindowFocus() {
    isWindowFocused = true;
  }

  function handleWindowBlur() {
    isWindowFocused = false;
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('focus', handleWindowFocus);
    window.addEventListener('blur', handleWindowBlur);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeyDown);
    window.removeEventListener('focus', handleWindowFocus);
    window.removeEventListener('blur', handleWindowBlur);
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', stopResize);
  });
</script>

<div class="app-layout" class:window-blurred={!isWindowFocused}>
  <div class="vignette"></div>

  <div class="app-body">
    {#if $appStore.showSidebar}
      <div class="sidebar-container" style="width: {sidebarWidth}px">
        <Sidebar
          width={sidebarWidth}
          rootPath={$currentProjectPath ?? ''}
          onFileSelect={handleFileSelect}
        />
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          class="resize-handle"
          class:active={isResizing}
          onmousedown={startResize}
          role="separator"
          aria-orientation="vertical"
          aria-valuenow={sidebarWidth}
          tabindex="-1"
        >
          <div class="resize-line"></div>
        </div>
      </div>
    {/if}
    <MainContent />
  </div>
  <StatusBar onShowShortcuts={() => (showShortcuts = true)} />
</div>

<KeyboardShortcuts isOpen={showShortcuts} onClose={() => (showShortcuts = false)} />
<ConfirmDialog />

<style>
  .app-layout {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-primary);
    transition:
      filter 0.4s ease,
      opacity 0.4s ease;
  }

  /* Window blur effect when app loses focus */
  .app-layout.window-blurred {
    filter: brightness(0.85) saturate(0.8);
  }

  /* Vignette effect for depth */
  .vignette {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 1;
    background: radial-gradient(
      ellipse 80% 60% at 50% 50%,
      transparent 50%,
      rgba(0, 0, 0, 0.3) 100%
    );
  }

  /* Subtle scan line effect */
  .vignette::before {
    content: '';
    position: absolute;
    inset: 0;
    background: repeating-linear-gradient(
      0deg,
      transparent,
      transparent 2px,
      rgba(0, 0, 0, 0.015) 2px,
      rgba(0, 0, 0, 0.015) 4px
    );
    pointer-events: none;
  }

  /* Noise texture overlay */
  .vignette::after {
    content: '';
    position: absolute;
    inset: 0;
    background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E");
    background-size: 256px 256px;
    opacity: 0.02;
    pointer-events: none;
  }

  /* Top glow line */
  .app-layout::after {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: linear-gradient(
      90deg,
      transparent 10%,
      rgba(125, 211, 252, 0.4) 50%,
      transparent 90%
    );
    opacity: 0.25;
    z-index: 10;
    transition: opacity 0.3s ease;
  }

  .app-layout.window-blurred::after {
    opacity: 0.08;
  }

  .app-body {
    position: relative;
    flex: 1;
    display: flex;
    overflow: hidden;
    z-index: 2;
  }

  .sidebar-container {
    position: relative;
    display: flex;
    flex-shrink: 0;
  }

  .resize-handle {
    position: absolute;
    right: -3px;
    top: 0;
    bottom: 0;
    width: 6px;
    cursor: col-resize;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .resize-line {
    width: 1px;
    height: 100%;
    background: transparent;
    transition: all var(--transition-fast);
    position: relative;
  }

  .resize-handle:hover .resize-line {
    width: 2px;
    background: linear-gradient(180deg, var(--gradient-start), var(--gradient-end));
    border-radius: 2px;
  }

  .resize-handle.active .resize-line {
    width: 3px;
    background: linear-gradient(180deg, var(--gradient-start), var(--gradient-end));
    border-radius: 2px;
  }

  .resize-handle:hover,
  .resize-handle.active {
    background: linear-gradient(90deg, transparent, var(--accent-subtle), transparent);
  }

  /* Resize handle grip dots */
  .resize-handle::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 3px;
    height: 24px;
    background: repeating-linear-gradient(
      180deg,
      var(--text-muted) 0px,
      var(--text-muted) 2px,
      transparent 2px,
      transparent 6px
    );
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .resize-handle:hover::before,
  .resize-handle.active::before {
    opacity: 0.4;
  }
</style>
