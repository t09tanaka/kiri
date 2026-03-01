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
  import { gitStore } from '@/lib/stores/gitStore';
  import { projectStore, isProjectOpen } from '@/lib/stores/projectStore';
  import { settingsStore } from '@/lib/stores/settingsStore';
  import { performanceService } from '@/lib/services/performanceService';
  import { setupLongTaskObserver } from '@/lib/utils/performanceMarker';
  import {
    loadSettings,
    saveSettings,
    loadProjectSettings,
    saveProjectSettings,
  } from '@/lib/services/persistenceService';
  import { portIsolationService } from '@/lib/services/portIsolationService';
  import { terminalService } from '@/lib/services/terminalService';
  import { confirmDialogStore } from '@/lib/stores/confirmDialogStore';
  import { getAllTerminalIds } from '@/lib/stores/tabStore';
  import { terminalRegistry } from '@/lib/stores/terminalRegistry';
  import { get } from 'svelte/store';

  let showShortcuts = $state(false);
  let windowLabel = $state('');
  let isAppQuitting = $state(false);

  async function handleOpenDirectory() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Open Directory',
    });

    if (selected && typeof selected === 'string') {
      // Skip reset if opening the same project
      const currentPath = projectStore.getCurrentPath();
      if (currentPath && currentPath !== selected) {
        await resetTerminals();
      }

      await projectStore.openProject(selected);
      // Open a default terminal tab for the new project
      if (get(tabStore).tabs.length === 0) {
        tabStore.addTerminalTab();
      }
    }
  }

  /**
   * Close all terminal PTY sessions, dispose xterm instances, and reset tab store.
   * Used when switching to a different project in the same window.
   */
  async function resetTerminals() {
    // Collect terminal IDs from tabs before resetting
    const state = get(tabStore);
    const tabTerminalIds = state.tabs.flatMap((t) => getAllTerminalIds(t.rootPane));

    // Reset tab store first so Svelte unmounts Terminal components cleanly
    tabStore.reset();

    // Clear any remaining registry entries (disposes xterm instances, calls unlisten)
    const registryTerminalIds = terminalRegistry.clearAll();

    // Merge all terminal IDs and deduplicate
    const allIds = [...new Set([...registryTerminalIds, ...tabTerminalIds])];

    // Close all PTY processes on the backend
    await Promise.all(allIds.map((id) => terminalService.closeTerminal(id).catch(() => {})));
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
          await invoke('create_window', {});
        } else {
          // Delegate to main window to avoid duplicate handling
          await emit('menu-new-window', {});
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

    // Listen for app-quitting event (for non-main windows to skip worktree cleanup)
    let unlistenAppQuitting: (() => void) | null = null;
    if (!isMainWindow) {
      unlistenAppQuitting = await listen('app-quitting', () => {
        isAppQuitting = true;
      });
    }

    // Handle URL parameters
    const params = new URLSearchParams(window.location.search);

    // Handle ?project= URL parameter (for worktree windows and open-recent)
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
      try {
        await windowService.registerWindow(windowLabel, decodedPath);
      } catch (e) {
        console.error('Failed to register window:', e);
      }

      if (get(tabStore).tabs.length === 0) {
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

    // Refresh worktree info and update window title when project changes
    const unsubscribeProjectStore = projectStore.subscribe((state) => {
      if (state.currentPath) {
        worktreeStore.refresh(state.currentPath);
        const projectName = state.currentPath.split('/').pop() || 'kiri';
        windowService.setTitle(`${projectName} — kiri`);
      } else {
        windowService.setTitle('kiri');
      }
    });

    // Auto-save settings when they change (main window only)
    let settingsSaveReady = false;
    let unsubscribeSettingsStore: (() => void) | null = null;
    if (isMainWindow) {
      // Delay enabling settings save to avoid saving the initial restore
      setTimeout(() => {
        settingsSaveReady = true;
      }, 500);
      unsubscribeSettingsStore = settingsStore.subscribe((state) => {
        if (settingsSaveReady) {
          saveSettings(state);
        }
      });
    }

    // Handle window close
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

      // Main window: signal non-main windows that app is quitting
      if (isMainWindow) {
        await eventService.emit('app-quitting', {});
        // Brief delay to let non-main windows receive the signal
        await new Promise((resolve) => setTimeout(resolve, 100));
      }

      // For worktree windows, automatically delete the worktree (skip when app is quitting)
      if (!isAppQuitting) {
        const currentPath = projectStore.getCurrentPath();
        if (currentPath) {
          try {
            const worktreeContext = await worktreeService.getContext(currentPath);
            if (
              worktreeContext?.is_worktree &&
              worktreeContext?.main_repo_path &&
              worktreeContext?.worktree_name
            ) {
              await worktreeService.remove(
                worktreeContext.main_repo_path,
                worktreeContext.worktree_name
              );

              // Release port assignments for this worktree
              try {
                const projectSettings = await loadProjectSettings(worktreeContext.main_repo_path);
                if (projectSettings.portConfig) {
                  projectSettings.portConfig = portIsolationService.removeWorktreeAssignments(
                    projectSettings.portConfig,
                    worktreeContext.worktree_name
                  );
                  await saveProjectSettings(worktreeContext.main_repo_path, projectSettings);
                }
              } catch (portError) {
                console.error('Failed to release port assignments:', portError);
              }

              // Emit event to notify other windows
              await eventService.emit('worktree-removed', { path: currentPath });
            }
          } catch (error) {
            // Worktree removal may fail (e.g., locked files, git errors).
            // The orphaned worktree will be cleaned up by pruneOrphanedAssignments
            // when WorktreePanel is opened next time.
            console.error(`Failed to remove worktree at ${currentPath}:`, error);
          }
        }
      }

      // Unregister window from the registry
      try {
        await windowService.unregisterWindow(windowLabel);
      } catch (error) {
        console.error('Failed to unregister window:', error);
      }

      await currentWindow.destroy();
    });

    // Listen for menu events from Rust
    const unlistenMenu = await listen('menu-open', () => {
      handleOpenDirectory();
    });

    // Listen for menu-new-window event from Rust menu handler
    // Only the main window handles this to avoid creating duplicate windows
    let unlistenMenuNewWindow: (() => void) | null = null;
    if (isMainWindow) {
      unlistenMenuNewWindow = await listen('menu-new-window', async () => {
        try {
          await invoke('create_window', {});
        } catch (error) {
          console.error('Failed to create window from menu:', error);
        }
      });
    }

    // Listen for menu-open-recent event from Rust menu handler
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
      unsubscribeProjectStore();
      unsubscribeSettingsStore?.();
      unlistenAppQuitting?.();
      unlistenWorktreeRemoved();
      window.removeEventListener('keydown', handleKeyDown);
      unlistenCloseRequested();
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
