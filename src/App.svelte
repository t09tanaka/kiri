<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, emit } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { dialogService } from '@/lib/services/dialogService';
  import { AppLayout, StartScreen } from '@/lib/components';
  import QuickOpen from '@/lib/components/search/QuickOpen.svelte';
  import ContentSearchModal from '@/lib/components/search/ContentSearchModal.svelte';
  import KeyboardShortcuts from '@/lib/components/ui/KeyboardShortcuts.svelte';
  import ToastContainer from '@/lib/components/ui/ToastContainer.svelte';
  import DiffViewModal from '@/lib/components/git/DiffViewModal.svelte';
  import CommitHistoryModal from '@/lib/components/git/CommitHistoryModal.svelte';
  import PrPanel from '@/lib/components/pr/PrPanel.svelte';
  import { prViewStore } from '@/lib/stores/prViewStore';
  import { prStore } from '@/lib/stores/prStore';
  import RemoteAccessSettings from '@/lib/components/settings/RemoteAccessSettings.svelte';
  import QrCodeModal from '@/lib/components/remote/QrCodeModal.svelte';
  import EditorModal from '@/lib/components/editor/EditorModal.svelte';
  import { searchStore, isQuickOpenVisible } from '@/lib/stores/searchStore';
  import { contentSearchStore, isContentSearchOpen } from '@/lib/stores/contentSearchStore';
  import { tabStore } from '@/lib/stores/tabStore';
  import { editorModalStore } from '@/lib/stores/editorModalStore';
  import { peekStore } from '@/lib/stores/peekStore';
  import { diffViewStore } from '@/lib/stores/diffViewStore';
  import { commitHistoryStore } from '@/lib/stores/commitHistoryStore';
  import { remoteAccessViewStore } from '@/lib/stores/remoteAccessViewStore';
  import { windowService } from '@/lib/services/windowService';
  import { PeekEditor } from '@/lib/components/peek';
  import { projectStore, isProjectOpen } from '@/lib/stores/projectStore';
  import { settingsStore, startupCommand } from '@/lib/stores/settingsStore';
  import { isRemoteActive } from '@/lib/stores/remoteAccessStore';
  import { toggleRemoteAccess } from '@/lib/utils/remoteAccessToggle';
  import { toastStore } from '@/lib/stores/toastStore';
  import type { StartupCommand } from '@/lib/services/persistenceService';
  import { performanceService } from '@/lib/services/performanceService';
  import { setupLongTaskObserver } from '@/lib/utils/performanceMarker';
  import { loadSettings, saveSettings } from '@/lib/services/persistenceService';
  import { terminalService } from '@/lib/services/terminalService';
  import { confirmDialogStore } from '@/lib/stores/confirmDialogStore';
  import { getAllTerminalIds } from '@/lib/stores/tabStore';
  import { terminalRegistry } from '@/lib/stores/terminalRegistry';
  import { get } from 'svelte/store';

  let showShortcuts = $state(false);
  let windowLabel = $state('');

  // Sync tools state to macOS menu bar
  $effect(() => {
    const remoteOn = $isRemoteActive;
    const cmd = $startupCommand;
    emit('update-tools-menu', {
      remoteAccessOn: remoteOn,
      startupCommand: cmd,
    });
  });

  async function handleOpenDirectory() {
    const selected = await dialogService.openDirectory();

    if (selected) {
      // Clean up existing terminals if switching projects or if orphaned terminals remain
      const currentPath = projectStore.getCurrentPath();
      const hasExistingTabs = get(tabStore).tabs.length > 0;
      if ((currentPath && currentPath !== selected) || hasExistingTabs) {
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
      await resetTerminals();
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

    // Cmd+Shift+P: Toggle PR panel (only when project is open)
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'p' && $isProjectOpen) {
      e.preventDefault();
      const path = projectStore.getCurrentPath();
      if (path) {
        if ($prViewStore.isOpen) {
          prViewStore.close();
        } else {
          prViewStore.open(path);
        }
      }
      return;
    }

    // Cmd+Shift+R: Toggle Remote Access Settings
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === 'r') {
      e.preventDefault();
      remoteAccessViewStore.toggleSettings();
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

    // Handle URL parameters
    const params = new URLSearchParams(window.location.search);

    // Handle ?project= URL parameter (for open-recent and new windows)
    const projectParam = params.get('project');
    if (projectParam) {
      const decodedPath = decodeURIComponent(projectParam);
      await projectStore.openProject(decodedPath);

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

    // Resize to start screen size when no project is open
    const currentPath = projectStore.getCurrentPath();
    if (!currentPath && isMainWindow) {
      try {
        await windowService.setSizeAndCenter(800, 600);
      } catch (error) {
        console.error('Failed to resize to start screen size:', error);
      }
    }

    window.addEventListener('keydown', handleKeyDown);

    // Update window title and refresh PR list when project changes
    const unsubscribeProjectStore = projectStore.subscribe((state) => {
      if (state.currentPath) {
        prStore.refresh(state.currentPath);
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

    // Listen for menu-toggle-remote event from Tools menu
    const unlistenMenuToggleRemote = await listen('menu-toggle-remote', async () => {
      // Open QR modal immediately for instant feedback (before any async work)
      if (!get(isRemoteActive)) {
        remoteAccessViewStore.openQrModal();
      }
      const result = await toggleRemoteAccess({
        onToggling: () => {},
        onError: (msg) => {
          if (msg) {
            toastStore.error(msg);
            remoteAccessViewStore.closeQrModal();
          }
        },
      });
      // Close QR modal if toggle failed
      if (!result && !get(isRemoteActive)) {
        remoteAccessViewStore.closeQrModal();
      }
    });

    // Listen for menu-set-startup-command event from Tools menu
    const unlistenMenuStartupCmd = await listen<string>(
      'menu-set-startup-command',
      async (event) => {
        const cmd = event.payload as StartupCommand;
        settingsStore.setStartupCommand(cmd);
      }
    );

    // Listen for menu-show-qr-code event from Tools menu
    const unlistenMenuShowQr = await listen('menu-show-qr-code', () => {
      remoteAccessViewStore.openQrModal();
    });

    performanceService.markStartupPhase('app-mount-complete');

    return () => {
      unsubscribeProjectStore();
      unsubscribeSettingsStore?.();
      window.removeEventListener('keydown', handleKeyDown);
      unlistenCloseRequested();
      unlistenMenu();
      unlistenMenuNewWindow?.();
      unlistenOpenRecent();
      unlistenClearRecent();
      unlistenMenuToggleRemote();
      unlistenMenuStartupCmd();
      unlistenMenuShowQr();
      cleanupLongTaskObserver();
    };
  });
</script>

{#if $isProjectOpen}
  <div class="app-container">
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

  {#if $prViewStore.isOpen && $prViewStore.projectPath}
    <PrPanel projectPath={$prViewStore.projectPath} onClose={() => prViewStore.close()} />
  {/if}

  {#if $isContentSearchOpen && projectStore.getCurrentPath()}
    <ContentSearchModal onOpenFile={handleFileSelect} onClose={() => contentSearchStore.close()} />
  {/if}
{:else}
  <StartScreen />
  <KeyboardShortcuts isOpen={showShortcuts} onClose={() => (showShortcuts = false)} />
{/if}

{#if $remoteAccessViewStore.isSettingsOpen}
  <RemoteAccessSettings onClose={() => remoteAccessViewStore.closeSettings()} />
{/if}

{#if $remoteAccessViewStore.isQrModalOpen}
  <QrCodeModal onClose={() => remoteAccessViewStore.closeQrModal()} />
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
</style>
