import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';

const MAX_RECENT_PROJECTS = 10;
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

  return {
    subscribe,

    async init() {
      const recentProjects = await loadRecentProjects();
      update((state) => ({
        ...state,
        recentProjects,
        isLoading: false,
      }));
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

      update((state) => {
        const existingIndex = state.recentProjects.findIndex((p) => p.path === path);
        let updatedProjects: RecentProject[];

        if (existingIndex >= 0) {
          updatedProjects = [
            newProject,
            ...state.recentProjects.slice(0, existingIndex),
            ...state.recentProjects.slice(existingIndex + 1),
          ];
        } else {
          updatedProjects = [newProject, ...state.recentProjects];
        }

        updatedProjects = updatedProjects.slice(0, MAX_RECENT_PROJECTS);

        saveRecentProjects(updatedProjects);

        return {
          ...state,
          currentPath: path,
          recentProjects: updatedProjects,
        };
      });
    },

    closeProject() {
      update((state) => ({
        ...state,
        currentPath: null,
      }));
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
      update((state) => {
        const updatedProjects = state.recentProjects.filter((p) => p.path !== path);
        saveRecentProjects(updatedProjects);
        return {
          ...state,
          recentProjects: updatedProjects,
        };
      });
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
