<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, emit } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-dialog';
  import { AppLayout, StartScreen } from '@/lib/components';
  import QuickOpen from '@/lib/components/search/QuickOpen.svelte';
  import ContentSearchModal from '@/lib/components/search/ContentSearchModal.svelte';
  import KeyboardShortcuts from '@/lib/components/ui/KeyboardShortcuts.svelte';
  import ToastContainer from '@/lib/components/ui/ToastContainer.svelte';
  import DiffViewModal from '@/lib/components/git/DiffViewModal.svelte';
  import CommitHistoryModal from '@/lib/components/git/CommitHistoryModal.svelte';
  import WorktreePanel from '@/lib/components/git/WorktreePanel.svelte';
  import EditorModal from '@/lib/components/editor/EditorModal.svelte';
  import { searchStore, isQuickOpenVisible } from '@/lib/stores/searchStore';
  import { contentSearchStore, isContentSearchOpen } from '@/lib/stores/contentSearchStore';
  import { tabStore } from '@/lib/stores/tabStore';
  import { editorModalStore } from '@/lib/stores/editorModalStore';
  import { peekStore } from '@/lib/stores/peekStore';
  import { diffViewStore } from '@/lib/stores/diffViewStore';
  import { commitHistoryStore } from '@/lib/stores/commitHistoryStore';
  import { worktreeViewStore } from '@/lib/stores/worktreeViewStore';
  import { worktreeStore, isWorktree, isSubdirectoryOfRepo } from '@/lib/stores/worktreeStore';
  import { toastStore } from '@/lib/stores/toastStore';
  import { worktreeService } from '@/lib/services/worktreeService';
  import { eventService } from '@/lib/services/eventService';
  import { windowService } from '@/lib/services/windowService';
  import { PeekEditor } from '@/lib/components/peek';
  import { appStore } from '@/lib/stores/appStore';
  import { gitStore } from '@/lib/stores/gitStore';
  import { projectStore, isProjectOpen } from '@/lib/stores/projectStore';
  import { settingsStore } from '@/lib/stores/settingsStore';
  import { performanceService } from '@/lib/services/performanceService';
  import { setupLongTaskObserver } from '@/lib/utils/performanceMarker';
  import {
    loadMultiWindowSession,
    saveMainWindowState,
    saveOtherWindowState,
    saveFullSession,
    clearOtherWindows,
    removeOtherWindow,
    loadSettings,
    saveSettings,
    loadProjectSettings,
    saveProjectSettings,
    type PersistedWindowState,
    type PersistedWindowGeometry,
    type PersistedPane,
  } from '@/lib/services/persistenceService';
  import { portIsolationService } from '@/lib/services/portIsolationService';
  import { terminalService } from '@/lib/services/terminalService';
  import { confirmDialogStore } from '@/lib/stores/confirmDialogStore';
  import { getAllTerminalIds, getPaneTerminalIdMap } from '@/lib/stores/tabStore';
  import { get } from 'svelte/store';

  let showShortcuts = $state(false);
  let windowLabel = $state('');
  let windowIndex = $state(-1); // -1 for main, 0+ for other windows
  let nextWindowIndex = $state(0); // Track next index for new windows (main only)
  let isAppQuitting = $state(false);

  /**
   * Get current window geometry (position and size)
   */
  async function getWindowGeometry(): Promise<PersistedWindowGeometry | undefined> {
    try {
      const result = await invoke<[number, number, number, number]>('get_window_geometry', {
        label: windowLabel,
      });
      return {
        x: result[0],
        y: result[1],
        width: result[2],
        height: result[3],
      };
    } catch (error) {
      console.error('Failed to get window geometry:', error);
      return undefined;
    }
  }

  /**
   * Recursively add CWD to persisted pane leaves from active terminals
   */
  async function addCwdToPersistedPane(
    pane: PersistedPane,
    paneTerminalMap: Map<string, number>
  ): Promise<PersistedPane> {
    if (pane.type === 'terminal') {
      const terminalId = paneTerminalMap.get(pane.id);
      if (terminalId !== undefined) {
        try {
          const cwd = await terminalService.getCwd(terminalId);
          if (cwd) {
            return { ...pane, cwd };
          }
        } catch {
          // Terminal might be closing, ignore
        }
      }
      return pane;
    }
    const children = await Promise.all(
      pane.children.map((child) => addCwdToPersistedPane(child, paneTerminalMap))
    );
    return { ...pane, children };
  }

  /**
   * Get current window state for persistence
   */
  async function getCurrentWindowState(): Promise<PersistedWindowState> {
    const currentPath = projectStore.getCurrentPath();
    const { tabs, activeTabId } = tabStore.getStateForPersistence();
    const ui = appStore.getUIForPersistence();
    const geometry = await getWindowGeometry();

    // Collect CWD for each terminal pane
    const runtimeState = get(tabStore);
    for (const tab of tabs) {
      if (tab.type === 'terminal' && tab.rootPane) {
        const runtimeTab = runtimeState.tabs.find((t) => t.id === tab.id);
        if (runtimeTab) {
          const paneTerminalMap = getPaneTerminalIdMap(runtimeTab.rootPane);
          tab.rootPane = await addCwdToPersistedPane(tab.rootPane, paneTerminalMap);
        }
      }
    }

    return {
      label: windowLabel,
      currentProject: currentPath,
      tabs,
      activeTabId,
      ui,
      geometry,
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

    const state = await getCurrentWindowState();

    // For main window, always save (to preserve geometry even on start screen)
    // For other windows, only save if they have content
    if (windowLabel === 'main') {
      await saveMainWindowState(state);
    } else if (windowIndex >= 0) {
      // Don't save empty non-main windows (no project and no tabs)
      if (!state.currentProject && state.tabs.length === 0) {
        return;
      }
      await saveOtherWindowState(windowIndex, state);
    }
  }

  /**
   * Restore window geometry (position and size)
   */
  async function restoreWindowGeometry(geometry: PersistedWindowGeometry) {
    try {
      // Delay to ensure window is fully initialized
      await new Promise((resolve) => setTimeout(resolve, 300));
      await invoke('set_window_geometry', {
        label: windowLabel,
        x: geometry.x,
        y: geometry.y,
        width: geometry.width,
        height: geometry.height,
      });
      // Trigger terminal resize after geometry change
      // Use multiple dispatches to ensure terminals catch the new size
      setTimeout(() => {
        window.dispatchEvent(new Event('terminal-resize'));
      }, 100);
      setTimeout(() => {
        window.dispatchEvent(new Event('terminal-resize'));
      }, 300);
    } catch (error) {
      console.error('Failed to restore window geometry:', error);
    }
  }

  /**
   * Restore window state from persisted data
   */
  async function restoreWindowState(state: PersistedWindowState, restoreGeometry = false) {
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
    } else if (state.currentProject) {
      // If project is open but no tabs, open a default terminal
      tabStore.addTerminalTab();
    }

    // Restore window geometry (only for main window, other windows get geometry at creation)
    if (restoreGeometry && state.geometry) {
      await restoreWindowGeometry(state.geometry);
    }
  }

  /**
   * Check if window state should be restored (not closed and has data)
   */
  function shouldRestoreWindow(win: Omit<PersistedWindowState, 'label'> | null): boolean {
    if (!win) return false;
    if (win.closed) return false;
    // Must have either project or tabs
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

    // Filter out closed and empty windows
    const otherWindowsData = (session.otherWindows || []).filter(shouldRestoreWindow);

    // Clear other windows to start fresh
    await clearOtherWindows();

    // Restore main window state (including geometry)
    if (session.mainWindow) {
      await restoreWindowState(session.mainWindow, true);
    }

    // Create and restore other windows (with new sequential indices)
    for (let i = 0; i < otherWindowsData.length; i++) {
      const winState = otherWindowsData[i];
      try {
        // Create window with geometry and index via URL parameter
        const geometry = winState.geometry;
        await invoke('create_window', {
          x: geometry?.x ?? null,
          y: geometry?.y ?? null,
          width: geometry?.width ?? null,
          height: geometry?.height ?? null,
          windowIndex: i,
        });
        // Send state to the new window after a short delay
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
      // Open a default terminal tab when opening a new project (if no tabs exist)
      const { tabs } = tabStore.getStateForPersistence();
      if (tabs.length === 0) {
        tabStore.addTerminalTab();
      }
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
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === 'w') {
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

    // Cmd+D: Toggle Diff View (only when project is open)
    if ((e.metaKey || e.ctrlKey) && e.key === 'd' && $isProjectOpen) {
      e.preventDefault();
      const path = projectStore.getCurrentPath();
      if (path) {
        if ($diffViewStore.isOpen) {
          diffViewStore.close();
        } else {
          diffViewStore.open(path);
        }
      }
      return;
    }

    // Cmd+H: Toggle Commit History (only when project is open)
    if ((e.metaKey || e.ctrlKey) && e.key === 'h' && $isProjectOpen) {
      e.preventDefault();
      const path = projectStore.getCurrentPath();
      if (path) {
        if ($commitHistoryStore.isOpen) {
          commitHistoryStore.close();
        } else {
          commitHistoryStore.open(path);
        }
      }
      return;
    }

    // Cmd+G: Toggle Worktrees (only when project is open and not in worktree)
    if ((e.metaKey || e.ctrlKey) && e.key === 'g' && $isProjectOpen && !$isWorktree) {
      e.preventDefault();
      const path = projectStore.getCurrentPath();
      if (path) {
        // Check if opened from a subdirectory of the repo
        if ($isSubdirectoryOfRepo) {
          toastStore.warning(
            'Worktrees can only be managed from the repository root. Please open the project from the root directory.',
            5000
          );
          return;
        }
        if ($worktreeViewStore.isOpen) {
          worktreeViewStore.close();
        } else {
          worktreeViewStore.open(path);
        }
      }
      return;
    }

    // Cmd+Shift+F: Toggle Content Search (only when project is open)
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === 'f' && $isProjectOpen) {
      e.preventDefault();
      const path = projectStore.getCurrentPath();
      if (path) {
        await contentSearchStore.toggle(path);
      }
      return;
    }

    // Cmd+Shift+N: New window
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      try {
        if (windowLabel === 'main') {
          const indexToAssign = nextWindowIndex;
          nextWindowIndex++;
          await invoke('create_window', { windowIndex: indexToAssign });
        } else {
          await invoke('create_window');
        }
      } catch (error) {
        console.error('Failed to create window:', error);
      }
      return;
    }

    // Cmd+= or Cmd+Shift+=: Zoom in (increase font size)
    if ((e.metaKey || e.ctrlKey) && (e.key === '=' || e.key === '+')) {
      e.preventDefault();
      settingsStore.zoomIn();
      return;
    }

    // Cmd+-: Zoom out (decrease font size)
    if ((e.metaKey || e.ctrlKey) && e.key === '-') {
      e.preventDefault();
      settingsStore.zoomOut();
      return;
    }

    // Cmd+0: Reset zoom
    if ((e.metaKey || e.ctrlKey) && e.key === '0') {
      e.preventDefault();
      settingsStore.resetZoom();
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
    editorModalStore.open(path);
  }

  onMount(async () => {
    performanceService.markStartupPhase('app-mount-start');

    // Setup long task observer (dev only)
    const cleanupLongTaskObserver = setupLongTaskObserver();

    // Initialize project store first (loads recent projects)
    await projectStore.init();

    // Load global settings (font size)
    const settings = await loadSettings();
    settingsStore.restoreState(settings);

    const currentWindow = getCurrentWindow();
    windowLabel = currentWindow.label;
    const isMainWindow = windowLabel === 'main';

    // Auto-save when tab state changes (debounced)
    let saveTimeout: ReturnType<typeof setTimeout> | null = null;
    let isRestoring = true;

    // Listen for state restore event and index assignment (for non-main windows)
    let unlistenRestore: (() => void) | null = null;
    let unlistenAssignIndex: (() => void) | null = null;
    let unlistenAppQuitting: (() => void) | null = null;
    if (!isMainWindow) {
      // Listen for state restore (when app starts)
      unlistenRestore = await listen<{ index: number; state: Omit<PersistedWindowState, 'label'> }>(
        'restore-window-state',
        async (event) => {
          // Receive index and state
          windowIndex = event.payload.index;
          await restoreWindowState({ ...event.payload.state, label: windowLabel });
        }
      );
      // Listen for index assignment (when created at runtime)
      unlistenAssignIndex = await listen<{ index: number }>('assign-window-index', (event) => {
        // Only accept if we don't have an index yet
        if (windowIndex === -1) {
          windowIndex = event.payload.index;
        }
      });
      // Listen for app-quitting event: report state to main window (don't write to store directly)
      unlistenAppQuitting = await listen('app-quitting', async () => {
        isAppQuitting = true;
        try {
          const state = await getCurrentWindowState();
          if (windowIndex >= 0) {
            await eventService.emit('window-state-report', {
              index: windowIndex,
              state: {
                currentProject: state.currentProject,
                tabs: state.tabs,
                activeTabId: state.activeTabId,
                ui: state.ui,
                geometry: state.geometry,
              },
            });
          }
        } catch {
          // Ignore errors during shutdown
        }
      });
    }

    // Restore session (main window only)
    if (isMainWindow) {
      await restoreSession();
    }

    // Handle URL parameters
    const params = new URLSearchParams(window.location.search);

    // Read windowIndex from URL parameter (reliable, no race condition)
    const windowIndexParam = params.get('windowIndex');
    if (windowIndexParam && !isMainWindow) {
      windowIndex = parseInt(windowIndexParam, 10);
    }

    // Handle ?project= URL parameter (for worktree windows)
    const projectParam = params.get('project');
    if (projectParam) {
      const decodedPath = decodeURIComponent(projectParam);

      // Check if this is a worktree - worktrees should not be added to project history
      const worktreeContext = await worktreeService.getContext(decodedPath);
      if (worktreeContext?.is_worktree) {
        // Worktree: just set current path without updating history
        projectStore.setCurrentPath(decodedPath);
      } else {
        // Normal project: add to history
        await projectStore.openProject(decodedPath);
      }

      // Register this window with the project path (for focus_or_create_window)
      // Note: Windows created via create_window are already registered in the backend,
      // but we register again here to ensure the mapping exists
      try {
        await windowService.registerWindow(windowLabel, decodedPath);
      } catch (e) {
        console.error('Failed to register window:', e);
      }

      const { tabs } = tabStore.getStateForPersistence();
      if (tabs.length === 0) {
        tabStore.addTerminalTab();
      }
    }

    // Listen for worktree-removed event (close window if its worktree was removed)
    const unlistenWorktreeRemoved = await listen<{ path: string }>('worktree-removed', (event) => {
      const currentPath = projectStore.getCurrentPath();
      if (currentPath && currentPath === event.payload.path) {
        projectStore.closeProject();
      }
    });

    // Load worktree info when project is open, or resize to start screen size
    const currentPath = projectStore.getCurrentPath();
    if (currentPath) {
      worktreeStore.refresh(currentPath);
    } else if (isMainWindow) {
      // No project open, resize to start screen size and center
      try {
        await windowService.setSizeAndCenter(800, 600);
      } catch (error) {
        console.error('Failed to resize to start screen size:', error);
      }
    }

    window.addEventListener('keydown', handleKeyDown);

    // Wait a bit before enabling auto-save, then save state
    setTimeout(() => {
      isRestoring = false;
      // Save state for all windows that have meaningful data
      saveCurrentWindowState();
    }, 1500);

    // Debounced save function
    const debouncedSave = () => {
      if (isRestoring) return;

      if (saveTimeout) {
        clearTimeout(saveTimeout);
      }
      saveTimeout = setTimeout(() => {
        saveCurrentWindowState();
      }, 500);
    };

    // Auto-save when tab state changes
    const unsubscribeTabStore = tabStore.subscribe(debouncedSave);

    // Auto-save when project changes (for new windows opening projects)
    const unsubscribeProjectStore = projectStore.subscribe((state) => {
      debouncedSave();
      // Refresh worktree info when project changes
      if (state.currentPath) {
        worktreeStore.refresh(state.currentPath);
      }
    });

    // Auto-save settings when they change (main window only)
    let unsubscribeSettingsStore: (() => void) | null = null;
    if (isMainWindow) {
      unsubscribeSettingsStore = settingsStore.subscribe((state) => {
        if (!isRestoring) {
          saveSettings(state);
        }
      });
    }

    // Save state before window closes
    const unlistenCloseRequested = await currentWindow.onCloseRequested(async (event) => {
      // Check for running terminal commands before closing
      const state = get(tabStore);
      const allTerminalIds = state.tabs.flatMap((tab) => getAllTerminalIds(tab.rootPane));
      if (allTerminalIds.length > 0) {
        const aliveChecks = await Promise.all(
          allTerminalIds.map((id) => terminalService.isTerminalAlive(id))
        );
        const hasRunningCommands = aliveChecks.some((alive) => alive);

        if (hasRunningCommands) {
          event.preventDefault();
          const confirmed = await confirmDialogStore.confirm({
            title: 'Close Window?',
            message: 'There are running commands in the terminal. Are you sure you want to close?',
            confirmLabel: 'Close',
            cancelLabel: 'Cancel',
            kind: 'warning',
          });
          if (!confirmed) return;
        }
      }

      event.preventDefault();

      // Main window: collect states from all non-main windows, then save in one write
      if (isMainWindow) {
        const nonMainCount = nextWindowIndex;
        // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local non-reactive collection in event handler
        const collectedStates = new Map<number, Omit<PersistedWindowState, 'label'>>();

        let resolveCollection: (() => void) | null = null;
        const collectionPromise = new Promise<void>((resolve) => {
          resolveCollection = resolve;
        });

        const unlistenReport = await eventService.listen<{
          index: number;
          state: Omit<PersistedWindowState, 'label'>;
        }>('window-state-report', (event) => {
          collectedStates.set(event.payload.index, event.payload.state);
          if (collectedStates.size >= nonMainCount && resolveCollection) {
            resolveCollection();
          }
        });

        // Notify non-main windows to report their state
        await eventService.emit('app-quitting', {});

        // Wait for all reports or timeout
        if (nonMainCount > 0) {
          await Promise.race([
            collectionPromise,
            new Promise<void>((resolve) => setTimeout(resolve, 500)),
          ]);
        }

        unlistenReport();

        // Save all states in one write
        const mainState = await getCurrentWindowState();
        await saveFullSession(mainState, collectedStates);
      }

      // For worktree windows, automatically delete the worktree (skip when app is quitting)
      if (!isAppQuitting) {
        const currentPath = projectStore.getCurrentPath();
        if (currentPath) {
          try {
            // Get fresh worktree context directly from Tauri (don't rely on store state)
            const worktreeContext = await worktreeService.getContext(currentPath);
            if (
              worktreeContext?.is_worktree &&
              worktreeContext?.main_repo_path &&
              worktreeContext?.worktree_name
            ) {
              // Remove worktree using main repo path and internal worktree name
              await worktreeService.remove(
                worktreeContext.main_repo_path,
                worktreeContext.worktree_name
              );

              // Release port assignments for this worktree
              try {
                const settings = await loadProjectSettings(worktreeContext.main_repo_path);
                if (settings.portConfig) {
                  settings.portConfig = portIsolationService.removeWorktreeAssignments(
                    settings.portConfig,
                    worktreeContext.worktree_name
                  );
                  await saveProjectSettings(worktreeContext.main_repo_path, settings);
                }
              } catch (portError) {
                console.error('Failed to release port assignments:', portError);
              }

              // Emit event to notify other windows
              await eventService.emit('worktree-removed', { path: currentPath });
            }
          } catch (error) {
            console.error('Failed to remove worktree:', error);
          }
        }
      }

      try {
        if (isMainWindow) {
          // Main window already saved all states via saveFullSession above
        } else if (windowIndex >= 0) {
          // Non-main window: save state, then mark as closed only if not quitting
          await saveCurrentWindowState();
          if (!isAppQuitting) {
            await removeOtherWindow(windowIndex);
          }
        }
      } catch (error) {
        console.error('Failed to handle window close:', error);
      }

      // Unregister window from the registry
      try {
        await windowService.unregisterWindow(windowLabel);
      } catch (error) {
        console.error('Failed to unregister window:', error);
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

    // Save when window is resized or moved (debounced)
    const unlistenResized = await currentWindow.onResized(debouncedSave);
    const unlistenMoved = await currentWindow.onMoved(debouncedSave);

    // Listen for menu events from Rust
    const unlistenMenu = await listen('menu-open', () => {
      handleOpenDirectory();
    });

    // Listen for menu-new-window event from Rust menu handler
    // Only the main window handles this to assign proper windowIndex
    let unlistenMenuNewWindow: (() => void) | null = null;
    if (isMainWindow) {
      unlistenMenuNewWindow = await listen('menu-new-window', async () => {
        try {
          const indexToAssign = nextWindowIndex;
          nextWindowIndex++;
          await invoke('create_window', { windowIndex: indexToAssign });
        } catch (error) {
          console.error('Failed to create window from menu:', error);
        }
      });
    }

    // Listen for menu-open-recent event from Rust menu handler
    // Opens in a new window (or focuses existing) to avoid overwriting the current project
    const unlistenOpenRecent = await listen<string>('menu-open-recent', async (event) => {
      const path = event.payload;
      if (path) {
        try {
          await invoke('focus_or_create_window', { projectPath: path });
        } catch (error) {
          console.error('Failed to open recent project:', error);
        }
      }
    });

    // Listen for menu-clear-recent event from Rust menu handler
    const unlistenClearRecent = await listen('menu-clear-recent', async () => {
      await projectStore.clearRecentProjects();
    });

    performanceService.markStartupPhase('app-mount-complete');

    return () => {
      if (saveTimeout) clearTimeout(saveTimeout);
      unsubscribeTabStore();
      unsubscribeProjectStore();
      unsubscribeSettingsStore?.();
      unlistenRestore?.();
      unlistenAssignIndex?.();
      unlistenAppQuitting?.();
      unlistenWorktreeRemoved();
      window.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      unlistenCloseRequested();
      unlistenResized();
      unlistenMoved();
      unlistenMenu();
      unlistenMenuNewWindow?.();
      unlistenOpenRecent();
      unlistenClearRecent();
      cleanupLongTaskObserver();
    };
  });
</script>

{#if $isProjectOpen}
  <div class="app-container">
    {#if $isWorktree}
      <div class="worktree-banner">
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <line x1="6" y1="3" x2="6" y2="15"></line>
          <circle cx="18" cy="6" r="3"></circle>
          <circle cx="6" cy="18" r="3"></circle>
          <path d="M18 9a9 9 0 0 1-9 9"></path>
        </svg>
        <span class="worktree-label">WT</span>
        {#if $gitStore.repoInfo?.branch}
          <span class="worktree-branch">{$gitStore.repoInfo.branch}</span>
        {:else}
          <span>Worktree</span>
        {/if}
      </div>
    {/if}
    <AppLayout />
  </div>

  {#if $isQuickOpenVisible}
    <QuickOpen onSelect={handleFileSelect} />
  {/if}

  {#if $peekStore.isOpen && $peekStore.filePath}
    <PeekEditor
      filePath={$peekStore.filePath}
      lineNumber={$peekStore.lineNumber}
      onClose={() => peekStore.close()}
    />
  {/if}

  {#if $diffViewStore.isOpen && $diffViewStore.projectPath}
    <DiffViewModal projectPath={$diffViewStore.projectPath} onClose={() => diffViewStore.close()} />
  {/if}

  {#if $commitHistoryStore.isOpen && $commitHistoryStore.projectPath}
    <CommitHistoryModal
      projectPath={$commitHistoryStore.projectPath}
      onClose={() => commitHistoryStore.close()}
    />
  {/if}

  {#if $editorModalStore.isOpen && $editorModalStore.filePath}
    <EditorModal filePath={$editorModalStore.filePath} onClose={() => editorModalStore.close()} />
  {/if}

  {#if $worktreeViewStore.isOpen && $worktreeViewStore.projectPath}
    <WorktreePanel
      projectPath={$worktreeViewStore.projectPath}
      onClose={() => worktreeViewStore.close()}
    />
  {/if}

  {#if $isContentSearchOpen && projectStore.getCurrentPath()}
    <ContentSearchModal onOpenFile={handleFileSelect} onClose={() => contentSearchStore.close()} />
  {/if}
{:else}
  <StartScreen />
  <KeyboardShortcuts isOpen={showShortcuts} onClose={() => (showShortcuts = false)} />
{/if}

<!-- Global Toast notifications -->
<ToastContainer />

<style>
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    overflow: hidden;
  }

  .app-container > :global(.app-layout) {
    flex: 1;
    min-height: 0;
  }

  .worktree-banner {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    height: 24px;
    background: linear-gradient(90deg, rgba(251, 191, 36, 0.18) 0%, rgba(251, 191, 36, 0.08) 100%);
    border-bottom: 1px solid rgba(251, 191, 36, 0.4);
    color: var(--git-modified);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.03em;
    flex-shrink: 0;
  }

  .worktree-banner svg {
    opacity: 0.9;
  }

  .worktree-banner .worktree-label {
    font-size: 10px;
    font-weight: 700;
    padding: 1px 6px;
    background: rgba(251, 191, 36, 0.3);
    border-radius: 3px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .worktree-banner .worktree-branch {
    font-family: var(--font-mono);
    font-weight: 600;
    text-transform: none;
  }
</style>
