import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';
import { windowService } from '@/lib/services/windowService';
import { eventService } from '@/lib/services/eventService';

const MAX_RECENT_PROJECTS = 10;
export const MAX_RECENT_MENU_ITEMS = 5;
const STORE_PATH = 'kiri-settings.json';

export interface RecentProject {
  path: string;
  name: string;
  lastOpened: number;
  gitBranch?: string | null;
}

interface ProjectState {
  currentPath: string | null;
  recentProjects: RecentProject[];
  isLoading: boolean;
}

interface GitRepoInfo {
  root: string;
  branch: string | null;
  statuses: unknown[];
}

function createProjectStore() {
  const { subscribe, update } = writable<ProjectState>({
    currentPath: null,
    recentProjects: [],
    isLoading: true,
  });

  let store: Store | null = null;

  async function getStore(): Promise<Store> {
    if (!store) {
      store = await Store.load(STORE_PATH);
    }
    return store;
  }

  async function loadRecentProjects(): Promise<RecentProject[]> {
    try {
      // Always reload store from disk to get fresh data (for multi-window support)
      const s = await getStore();
      await s.reload();
      const projects = await s.get<RecentProject[]>('recentProjects');
      return projects ?? [];
    } catch (error) {
      console.error('Failed to load recent projects:', error);
      return [];
    }
  }

  async function saveRecentProjects(projects: RecentProject[]): Promise<void> {
    try {
      const s = await getStore();
      await s.set('recentProjects', projects);
      await s.save();
    } catch (error) {
      console.error('Failed to save recent projects:', error);
    }
  }

  async function getGitBranch(path: string): Promise<string | null> {
    try {
      const info = await invoke<GitRepoInfo>('get_git_status', { path });
      return info.branch;
    } catch {
      return null;
    }
  }

  async function emitRecentMenuUpdate(projects: RecentProject[]) {
    const menuItems = projects.slice(0, MAX_RECENT_MENU_ITEMS);
    await eventService.emit('update-recent-menu', menuItems);
  }

  return {
    subscribe,

    async init() {
      const recentProjects = await loadRecentProjects();
      update((state) => ({
        ...state,
        recentProjects,
        isLoading: false,
      }));
      await emitRecentMenuUpdate(recentProjects);
    },

    async openProject(path: string) {
      const name = path.split('/').pop() || path;
      const gitBranch = await getGitBranch(path);

      const newProject: RecentProject = {
        path,
        name,
        lastOpened: Date.now(),
        gitBranch,
      };

      // Re-read from disk before computing the new list so concurrent updates
      // from other windows aren't clobbered by this window's stale in-memory
      // copy.
      const latestProjects = await loadRecentProjects();
      const existingIndex = latestProjects.findIndex((p) => p.path === path);
      const merged =
        existingIndex >= 0
          ? [
              newProject,
              ...latestProjects.slice(0, existingIndex),
              ...latestProjects.slice(existingIndex + 1),
            ]
          : [newProject, ...latestProjects];
      const updatedProjects = merged.slice(0, MAX_RECENT_PROJECTS);

      update((state) => ({
        ...state,
        currentPath: path,
        recentProjects: updatedProjects,
      }));

      // Await the save so silent failures surface in logs and so the next
      // operation observes the disk state we just wrote.
      await saveRecentProjects(updatedProjects);
      await emitRecentMenuUpdate(updatedProjects);

      // Resize window to main editor size when opening a project
      try {
        await windowService.setSizeAndCenter(1200, 800);
      } catch (error) {
        console.error('Failed to resize window:', error);
      }
    },

    /**
     * Update the lastOpened timestamp for a project without changing the
     * current window's currentPath. Used when the user opens a project via
     * the "Open Recent" menu and the target window already exists (so
     * openProject() is never called by that window).
     */
    async bumpRecentTimestamp(path: string) {
      const latestProjects = await loadRecentProjects();
      const existingIndex = latestProjects.findIndex((p) => p.path === path);
      if (existingIndex < 0) return;

      const bumped: RecentProject = {
        ...latestProjects[existingIndex],
        lastOpened: Date.now(),
      };
      const updatedProjects = [
        bumped,
        ...latestProjects.slice(0, existingIndex),
        ...latestProjects.slice(existingIndex + 1),
      ].slice(0, MAX_RECENT_PROJECTS);

      update((state) => ({
        ...state,
        recentProjects: updatedProjects,
      }));

      await saveRecentProjects(updatedProjects);
      await emitRecentMenuUpdate(updatedProjects);
    },

    async closeProject() {
      update((state) => ({
        ...state,
        currentPath: null,
      }));

      // Resize window to start screen size and center when closing a project
      try {
        await windowService.setSizeAndCenter(800, 600);
      } catch (error) {
        console.error('Failed to resize window:', error);
      }
    },

    /**
     * Set current path directly (for restoring from persistence)
     * Unlike openProject, this doesn't update recent projects
     */
    setCurrentPath(path: string | null) {
      update((state) => ({
        ...state,
        currentPath: path,
      }));
    },

    async refreshRecentProjectsGitInfo() {
      update((state) => ({ ...state }));

      const currentState = await new Promise<ProjectState>((resolve) => {
        const unsubscribe = subscribe((state) => {
          resolve(state);
          unsubscribe();
        });
      });

      const updatedProjects = await Promise.all(
        currentState.recentProjects.map(async (project) => {
          const gitBranch = await getGitBranch(project.path);
          return { ...project, gitBranch };
        })
      );

      update((state) => ({
        ...state,
        recentProjects: updatedProjects,
      }));

      await saveRecentProjects(updatedProjects);
    },

    async removeProject(path: string) {
      const latestProjects = await loadRecentProjects();
      const updatedProjects = latestProjects.filter((p) => p.path !== path);

      update((state) => ({
        ...state,
        recentProjects: updatedProjects,
      }));

      await saveRecentProjects(updatedProjects);
      await emitRecentMenuUpdate(updatedProjects);
    },

    async clearRecentProjects() {
      update((state) => ({
        ...state,
        recentProjects: [],
      }));

      await saveRecentProjects([]);
      await emitRecentMenuUpdate([]);
    },

    getCurrentPath(): string | null {
      let currentPath: string | null = null;
      subscribe((state) => {
        currentPath = state.currentPath;
      })();
      return currentPath;
    },
  };
}

export const projectStore = createProjectStore();

export const currentProjectPath = derived(projectStore, ($store) => $store.currentPath);

export const currentProjectName = derived(projectStore, ($store) =>
  $store.currentPath ? $store.currentPath.split('/').pop() || $store.currentPath : null
);

export const recentProjects = derived(projectStore, ($store) => $store.recentProjects);

export const isProjectOpen = derived(projectStore, ($store) => $store.currentPath !== null);
