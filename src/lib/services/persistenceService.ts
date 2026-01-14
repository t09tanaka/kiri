import { Store } from '@tauri-apps/plugin-store';
import { invoke } from '@tauri-apps/api/core';
import type { SidebarMode } from '@/lib/stores/appStore';

const STORE_PATH = 'kiri-settings.json';

// Global settings (shared across all windows)
export interface PersistedSettings {
  fontSize: number;
}

const DEFAULT_SETTINGS: PersistedSettings = {
  fontSize: 13,
};

// Persisted tab structure (minimal data needed for restoration)
export interface PersistedTab {
  id: string;
  type: 'editor' | 'terminal';
  filePath?: string; // editor only
  title?: string; // terminal only
}

// UI state to persist
export interface PersistedUI {
  sidebarWidth: number;
  showSidebar: boolean;
  sidebarMode: SidebarMode;
}

// Window geometry (position and size)
export interface PersistedWindowGeometry {
  x: number;
  y: number;
  width: number;
  height: number;
}

// Single window state
export interface PersistedWindowState {
  label: string;
  currentProject: string | null;
  tabs: PersistedTab[];
  activeTabId: string | null;
  ui: PersistedUI;
  geometry?: PersistedWindowGeometry; // Window position and size
  closed?: boolean; // Mark window as closed (for filtering on restore)
}

// Complete persisted state (for backwards compatibility)
export interface PersistedState {
  currentProject: string | null;
  tabs: PersistedTab[];
  activeTabId: string | null;
  ui: PersistedUI;
}

// Multi-window session state (simplified structure)
export interface PersistedSession {
  mainWindow: PersistedWindowState | null;
  otherWindows: Omit<PersistedWindowState, 'label'>[]; // Array-based, no labels
}

const DEFAULT_UI: PersistedUI = {
  sidebarWidth: 220,
  showSidebar: true,
  sidebarMode: 'explorer',
};

let store: Store | null = null;

async function getStore(): Promise<Store> {
  if (!store) {
    store = await Store.load(STORE_PATH);
  }
  return store;
}

/**
 * Check if a file exists by trying to read it
 */
async function fileExists(path: string): Promise<boolean> {
  try {
    await invoke('read_file', { path });
    return true;
  } catch {
    return false;
  }
}

/**
 * Load persisted session state
 */
export async function loadSessionState(): Promise<PersistedState | null> {
  try {
    const s = await getStore();
    await s.reload(); // Ensure fresh data for multi-window support

    const state = await s.get<PersistedState>('sessionState');
    if (!state) {
      return null;
    }

    // Validate editor tab file paths exist
    const validatedTabs: PersistedTab[] = [];
    for (const tab of state.tabs) {
      if (tab.type === 'editor' && tab.filePath) {
        const exists = await fileExists(tab.filePath);
        if (exists) {
          validatedTabs.push(tab);
        }
      } else if (tab.type === 'terminal') {
        validatedTabs.push(tab);
      }
    }

    // If active tab was removed, pick first available
    let activeTabId = state.activeTabId;
    if (activeTabId && !validatedTabs.find((t) => t.id === activeTabId)) {
      activeTabId = validatedTabs.length > 0 ? validatedTabs[0].id : null;
    }

    return {
      ...state,
      tabs: validatedTabs,
      activeTabId,
    };
  } catch (error) {
    console.error('Failed to load session state:', error);
    return null;
  }
}

/**
 * Save session state to disk
 */
export async function saveSessionState(state: PersistedState): Promise<void> {
  try {
    const s = await getStore();
    await s.set('sessionState', state);
    await s.save();
  } catch (error) {
    console.error('Failed to save session state:', error);
  }
}

/**
 * Clear session state (for fresh start)
 */
export async function clearSessionState(): Promise<void> {
  try {
    const s = await getStore();
    await s.delete('sessionState');
    await s.save();
  } catch (error) {
    console.error('Failed to clear session state:', error);
  }
}

/**
 * Helper to extract UI state from current values
 */
export function createUIState(
  sidebarWidth: number,
  showSidebar: boolean,
  sidebarMode: SidebarMode
): PersistedUI {
  return { sidebarWidth, showSidebar, sidebarMode };
}

/**
 * Get default UI state
 */
export function getDefaultUI(): PersistedUI {
  return { ...DEFAULT_UI };
}

/**
 * Validate tabs for a window state
 */
async function validateWindowTabs(
  tabs: PersistedTab[]
): Promise<{ tabs: PersistedTab[]; activeTabId: string | null }> {
  const validatedTabs: PersistedTab[] = [];
  for (const tab of tabs) {
    if (tab.type === 'editor' && tab.filePath) {
      const exists = await fileExists(tab.filePath);
      if (exists) {
        validatedTabs.push(tab);
      }
    } else if (tab.type === 'terminal') {
      validatedTabs.push(tab);
    }
  }
  return {
    tabs: validatedTabs,
    activeTabId: validatedTabs.length > 0 ? validatedTabs[0].id : null,
  };
}

/**
 * Check if a window state has meaningful data (content or geometry)
 */
function hasWindowData(
  win: Omit<PersistedWindowState, 'label'> | null,
  includeGeometry = false
): boolean {
  if (!win) return false;
  // Content-based check
  if (win.tabs.length > 0 || win.currentProject !== null) return true;
  // Geometry-based check (for main window)
  if (includeGeometry && win.geometry) return true;
  return false;
}

/**
 * Load multi-window session state
 */
export async function loadMultiWindowSession(): Promise<PersistedSession | null> {
  try {
    const s = await getStore();
    await s.reload();

    const session = await s.get<PersistedSession>('multiWindowSession');

    // Check if session has new structure
    if (session && (session.mainWindow || session.otherWindows)) {
      // Validate main window tabs
      // Include geometry for main window (always restore main window geometry)
      let mainWindow: PersistedWindowState | null = null;
      if (session.mainWindow && hasWindowData(session.mainWindow, true)) {
        const { tabs, activeTabId } = await validateWindowTabs(session.mainWindow.tabs || []);
        mainWindow = {
          ...session.mainWindow,
          label: 'main',
          tabs,
          activeTabId:
            session.mainWindow.activeTabId &&
            tabs.find((t) => t.id === session.mainWindow!.activeTabId)
              ? session.mainWindow.activeTabId
              : activeTabId,
        };
      }

      // Validate other windows tabs
      const otherWindows: Omit<PersistedWindowState, 'label'>[] = [];
      for (const win of session.otherWindows || []) {
        if (hasWindowData(win)) {
          const { tabs, activeTabId } = await validateWindowTabs(win.tabs || []);
          otherWindows.push({
            ...win,
            tabs,
            activeTabId:
              win.activeTabId && tabs.find((t) => t.id === win.activeTabId)
                ? win.activeTabId
                : activeTabId,
          });
        }
      }

      if (mainWindow || otherWindows.length > 0) {
        return { mainWindow, otherWindows };
      }
    }

    // Fallback: try to load old format and migrate
    // Check old array-based format
    const oldSession = await s.get<{ windows: PersistedWindowState[] }>('multiWindowSession');
    if (oldSession && oldSession.windows && oldSession.windows.length > 0) {
      const mainWin = oldSession.windows.find((w) => w.label === 'main');
      const otherWins = oldSession.windows.filter((w) => w.label !== 'main');

      if (mainWin || otherWins.length > 0) {
        const migratedSession: PersistedSession = {
          mainWindow: mainWin ? { ...mainWin } : null,
          otherWindows: otherWins.map(({ label: _label, ...rest }) => rest),
        };
        await s.set('multiWindowSession', migratedSession);
        await s.save();
        return migratedSession;
      }
    }

    // Check even older single-window format
    const oldState = await s.get<PersistedState>('sessionState');
    if (oldState && (oldState.tabs.length > 0 || oldState.currentProject)) {
      const migratedSession: PersistedSession = {
        mainWindow: {
          label: 'main',
          currentProject: oldState.currentProject,
          tabs: oldState.tabs,
          activeTabId: oldState.activeTabId,
          ui: oldState.ui || DEFAULT_UI,
        },
        otherWindows: [],
      };
      await s.set('multiWindowSession', migratedSession);
      await s.delete('sessionState');
      await s.save();
      return migratedSession;
    }

    return null;
  } catch (error) {
    console.error('Failed to load multi-window session:', error);
    return null;
  }
}

/**
 * Save main window state
 */
export async function saveMainWindowState(
  windowState: Omit<PersistedWindowState, 'label'>
): Promise<void> {
  try {
    const s = await getStore();
    await s.reload();

    const session = (await s.get<PersistedSession>('multiWindowSession')) || {
      mainWindow: null,
      otherWindows: [],
    };

    session.mainWindow = { ...windowState, label: 'main' };

    await s.set('multiWindowSession', session);
    await s.save();
  } catch (error) {
    console.error('Failed to save main window state:', error);
  }
}

/**
 * Save a non-main window's state by index
 */
export async function saveOtherWindowState(
  index: number,
  windowState: Omit<PersistedWindowState, 'label'>
): Promise<void> {
  try {
    const s = await getStore();
    await s.reload();

    const session = (await s.get<PersistedSession>('multiWindowSession')) || {
      mainWindow: null,
      otherWindows: [],
    };

    // Ensure array is large enough
    while (session.otherWindows.length <= index) {
      session.otherWindows.push({
        currentProject: null,
        tabs: [],
        activeTabId: null,
        ui: DEFAULT_UI,
      });
    }

    session.otherWindows[index] = windowState;

    await s.set('multiWindowSession', session);
    await s.save();
  } catch (error) {
    console.error('Failed to save other window state:', error);
  }
}

/**
 * Mark a non-main window as closed (doesn't shift indices)
 */
export async function removeOtherWindow(index: number): Promise<void> {
  try {
    const s = await getStore();
    await s.reload();

    const session = await s.get<PersistedSession>('multiWindowSession');
    if (!session || !session.otherWindows) return;

    if (index >= 0 && index < session.otherWindows.length) {
      // Mark as closed instead of removing (to preserve indices)
      session.otherWindows[index] = {
        ...session.otherWindows[index],
        closed: true,
      };
    }

    await s.set('multiWindowSession', session);
    await s.save();
  } catch (error) {
    console.error('Failed to remove other window:', error);
  }
}

/**
 * Clear all other windows (called on startup before restoring)
 */
export async function clearOtherWindows(): Promise<void> {
  try {
    const s = await getStore();
    await s.reload();

    const session = await s.get<PersistedSession>('multiWindowSession');
    if (!session) return;

    session.otherWindows = [];

    await s.set('multiWindowSession', session);
    await s.save();
  } catch (error) {
    console.error('Failed to clear other windows:', error);
  }
}

/**
 * Load global settings (font size, etc.)
 */
export async function loadSettings(): Promise<PersistedSettings> {
  try {
    const s = await getStore();
    await s.reload();

    const settings = await s.get<PersistedSettings>('globalSettings');
    if (!settings) {
      return { ...DEFAULT_SETTINGS };
    }

    return {
      fontSize: settings.fontSize ?? DEFAULT_SETTINGS.fontSize,
    };
  } catch (error) {
    console.error('Failed to load settings:', error);
    return { ...DEFAULT_SETTINGS };
  }
}

/**
 * Save global settings
 */
export async function saveSettings(settings: PersistedSettings): Promise<void> {
  try {
    const s = await getStore();
    await s.set('globalSettings', settings);
    await s.save();
  } catch (error) {
    console.error('Failed to save settings:', error);
  }
}

/**
 * Get default settings
 */
export function getDefaultSettings(): PersistedSettings {
  return { ...DEFAULT_SETTINGS };
}
