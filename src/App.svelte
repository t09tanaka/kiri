<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, emit } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-dialog';
  import { AppLayout, StartScreen } from '@/lib/components';
  import QuickOpen from '@/lib/components/search/QuickOpen.svelte';
  import KeyboardShortcuts from '@/lib/components/ui/KeyboardShortcuts.svelte';
  import { searchStore, isQuickOpenVisible } from '@/lib/stores/searchStore';
  import { tabStore } from '@/lib/stores/tabStore';
  import { appStore } from '@/lib/stores/appStore';
  import { projectStore, isProjectOpen } from '@/lib/stores/projectStore';
  import {
    loadMultiWindowSession,
    saveMainWindowState,
    saveOtherWindowState,
    clearOtherWindows,
    removeOtherWindow,
    type PersistedWindowState,
  } from '@/lib/services/persistenceService';

  let showShortcuts = $state(false);
  let windowLabel = $state('');
  let windowIndex = $state(-1); // -1 for main, 0+ for other windows
  let nextWindowIndex = $state(0); // Track next index for new windows (main only)

  /**
   * Get current window state for persistence
   */
  function getCurrentWindowState(): PersistedWindowState {
    const currentPath = projectStore.getCurrentPath();
    const { tabs, activeTabId } = tabStore.getStateForPersistence();
    const ui = appStore.getUIForPersistence();

    return {
      label: windowLabel,
      currentProject: currentPath,
      tabs,
      activeTabId,
      ui,
    };
  }

  /**
   * Save current window's state to disk
   */
  async function saveCurrentWindowState() {
    // Don't save if window label is not yet set
    if (!windowLabel) {
      return;
    }

    const state = getCurrentWindowState();
    // Don't save empty state - only save when there's meaningful data
    if (!state.currentProject && state.tabs.length === 0) {
      return;
    }

    if (windowLabel === 'main') {
      await saveMainWindowState(state);
    } else if (windowIndex >= 0) {
      await saveOtherWindowState(windowIndex, state);
    }
  }

  /**
   * Restore window state from persisted data
   */
  function restoreWindowState(state: PersistedWindowState) {
    // Restore UI settings
    if (state.ui) {
      appStore.restoreUI(state.ui);
    }

    // Restore project path
    if (state.currentProject) {
      projectStore.setCurrentPath(state.currentProject);
    }

    // Restore tabs
    if (state.tabs && state.tabs.length > 0) {
      tabStore.restoreState(state.tabs, state.activeTabId);
    }
  }

  /**
   * Check if window state has meaningful data
   */
  function hasWindowData(win: Omit<PersistedWindowState, 'label'> | null): boolean {
    if (!win) return false;
    return win.tabs.length > 0 || win.currentProject !== null;
  }

  /**
   * Restore session - main window restores all windows
   */
  async function restoreSession() {
    const session = await loadMultiWindowSession();
    if (!session) {
      return;
    }

    // Filter out empty entries from other windows
    const otherWindowsData = (session.otherWindows || []).filter(hasWindowData);

    // Clear other windows to start fresh
    await clearOtherWindows();

    // Restore main window state
    if (session.mainWindow) {
      restoreWindowState(session.mainWindow);
    }

    // Create and restore other windows (with new sequential indices)
    for (let i = 0; i < otherWindowsData.length; i++) {
      const winState = otherWindowsData[i];
      try {
        await invoke('create_window');
        // Send state and index to the new window after a short delay
        setTimeout(async () => {
          await emit('restore-window-state', { index: i, state: winState });
        }, 500);
      } catch (error) {
        console.error('Failed to create window:', error);
      }
    }

    // Set next index for new windows created at runtime
    nextWindowIndex = otherWindowsData.length;
  }

  async function handleOpenDirectory() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Open Directory',
    });

    if (selected && typeof selected === 'string') {
      await projectStore.openProject(selected);
    }
  }

  async function handleKeyDown(e: KeyboardEvent) {
    // Cmd+O: Open directory
    if ((e.metaKey || e.ctrlKey) && e.key === 'o') {
      e.preventDefault();
      await handleOpenDirectory();
      return;
    }

    // Cmd+Shift+W: Close project (return to start screen)
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'w') {
      e.preventDefault();
      projectStore.closeProject();
      return;
    }

    // Cmd+P: Quick open (only when project is open)
    if ((e.metaKey || e.ctrlKey) && e.key === 'p' && $isProjectOpen) {
      e.preventDefault();
      if ($isQuickOpenVisible) {
        searchStore.closeQuickOpen();
      } else {
        const path = projectStore.getCurrentPath();
        if (path) {
          searchStore.setRootPath(path);
        }
        searchStore.openQuickOpen();
      }
      return;
    }

    // Cmd+Shift+N: New window
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'n') {
      e.preventDefault();
      try {
        await invoke('create_window');
        // If main window, assign index to new window
        if (windowLabel === 'main') {
          const indexToAssign = nextWindowIndex;
          nextWindowIndex++;
          setTimeout(async () => {
            await emit('assign-window-index', { index: indexToAssign });
          }, 500);
        }
      } catch (error) {
        console.error('Failed to create window:', error);
      }
      return;
    }

    // Skip if typing in an input for global shortcuts
    const target = e.target as HTMLElement;
    const isTyping =
      target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable;

    // ? to show keyboard shortcuts (only when not typing and no project open)
    if (e.key === '?' && !isTyping && !e.ctrlKey && !e.metaKey && !$isProjectOpen) {
      e.preventDefault();
      showShortcuts = true;
      return;
    }

    // Cmd+/ to toggle keyboard shortcuts
    if ((e.metaKey || e.ctrlKey) && e.key === '/') {
      e.preventDefault();
      showShortcuts = !showShortcuts;
    }
  }

  function handleFileSelect(path: string) {
    tabStore.addEditorTab(path);
  }

  onMount(async () => {
    // Initialize project store first (loads recent projects)
    await projectStore.init();

    const currentWindow = getCurrentWindow();
    windowLabel = currentWindow.label;
    const isMainWindow = windowLabel === 'main';

    // Auto-save when tab state changes (debounced)
    let saveTimeout: ReturnType<typeof setTimeout> | null = null;
    let isRestoring = true;

    // Listen for state restore event and index assignment (for non-main windows)
    let unlistenRestore: (() => void) | null = null;
    let unlistenAssignIndex: (() => void) | null = null;
    let wasRestored = false;
    if (!isMainWindow) {
      // Listen for state restore (when app starts)
      unlistenRestore = await listen<{ index: number; state: Omit<PersistedWindowState, 'label'> }>(
        'restore-window-state',
        (event) => {
          // Receive index and state
          windowIndex = event.payload.index;
          restoreWindowState({ ...event.payload.state, label: windowLabel });
          wasRestored = true;
        }
      );
      // Listen for index assignment (when created at runtime)
      unlistenAssignIndex = await listen<{ index: number }>('assign-window-index', (event) => {
        // Only accept if we don't have an index yet
        if (windowIndex === -1) {
          windowIndex = event.payload.index;
        }
      });
    }

    // Restore session (main window only)
    if (isMainWindow) {
      await restoreSession();
    }

    window.addEventListener('keydown', handleKeyDown);

    // Wait a bit before enabling auto-save, then save restored state
    setTimeout(() => {
      isRestoring = false;
      // If window was restored, save its state to ensure persistence
      if (wasRestored || isMainWindow) {
        saveCurrentWindowState();
      }
    }, 1500);

    const unsubscribeTabStore = tabStore.subscribe(() => {
      if (isRestoring) return;

      if (saveTimeout) {
        clearTimeout(saveTimeout);
      }
      saveTimeout = setTimeout(() => {
        saveCurrentWindowState();
      }, 500);
    });

    // Save state before window closes
    const unlistenCloseRequested = await currentWindow.onCloseRequested(async (event) => {
      event.preventDefault();
      try {
        if (isMainWindow) {
          // Main window: save state
          await saveCurrentWindowState();
        } else if (windowIndex >= 0) {
          // Non-main window: remove from session instead of saving
          await removeOtherWindow(windowIndex);
        }
      } catch (error) {
        console.error('Failed to handle window close:', error);
      }
      await currentWindow.destroy();
    });

    // Save on visibility change
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'hidden') {
        saveCurrentWindowState();
      }
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);

    // Listen for menu events from Rust
    const unlistenMenu = await listen('menu-open', () => {
      handleOpenDirectory();
    });

    return () => {
      if (saveTimeout) clearTimeout(saveTimeout);
      unsubscribeTabStore();
      unlistenRestore?.();
      unlistenAssignIndex?.();
      window.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      unlistenCloseRequested();
      unlistenMenu();
    };
  });
</script>

{#if $isProjectOpen}
  <AppLayout />

  {#if $isQuickOpenVisible}
    <QuickOpen onSelect={handleFileSelect} />
  {/if}
{:else}
  <StartScreen />
  <KeyboardShortcuts isOpen={showShortcuts} onClose={() => (showShortcuts = false)} />
{/if}
