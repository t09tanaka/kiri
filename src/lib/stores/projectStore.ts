// Backward-compatible facade over `projectState` (issue #42 phase 1).
//
// The canonical in-memory snapshot lives in `projectState.svelte.ts`.
// This file owns all the side effects (Tauri Store IO, multi-window
// event fan-out via eventService, window resizing via windowService)
// because we don't want to drag those imports into the canonical class.

import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';
import { windowService } from '@/lib/services/windowService';
import { eventService } from '@/lib/services/eventService';
import { projectState, type RecentProject, type ProjectStateShape } from './projectState.svelte';

export type { RecentProject };
export type ProjectState = ProjectStateShape;

const MAX_RECENT_PROJECTS = 10;
export const MAX_RECENT_MENU_ITEMS = 5;
const STORE_PATH = 'kiri-settings.json';

interface GitRepoInfo {
  root: string;
  branch: string | null;
  statuses: unknown[];
}

function snapshot(): ProjectStateShape {
  return { ...projectState.state };
}

function createProjectStore() {
  const mirror = writable<ProjectStateShape>(snapshot());
  const refresh = () => mirror.set(snapshot());

  const patch = (partial: Partial<ProjectStateShape>) => {
    projectState.patch(partial);
    refresh();
  };

  let store: Store | null = null;

  async function getStore(): Promise<Store> {
    if (!store) {
      store = await Store.load(STORE_PATH);
    }
    return store;
  }

  /**
   * Load recent projects from disk. Returns [] on any failure so the UI
   * still renders, but never use this result as the basis for a write — a
   * transient load failure would then overwrite the persisted list with
   * an empty / partial value. Writers must use `readLatestForWrite`.
   */
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

  /**
   * Strict variant used as the basis for a save. Propagates failures so
   * the caller can skip the write and avoid clobbering the on-disk list
   * with stale or empty data.
   */
  async function readLatestForWrite(): Promise<RecentProject[]> {
    const s = await getStore();
    await s.reload();
    const projects = await s.get<RecentProject[]>('recentProjects');
    return projects ?? [];
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
    subscribe: mirror.subscribe,

    async init() {
      const recentProjects = await loadRecentProjects();
      patch({ recentProjects, isLoading: false });
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
      // copy. If the strict read fails (genuine disk error), skip the save
      // entirely instead of clobbering the persisted list — but still flip
      // currentPath so the UI navigates into the project the user asked for.
      let updatedProjects: RecentProject[] | null = null;
      try {
        const latestProjects = await readLatestForWrite();
        const existingIndex = latestProjects.findIndex((p) => p.path === path);
        const merged =
          existingIndex >= 0
            ? [
                newProject,
                ...latestProjects.slice(0, existingIndex),
                ...latestProjects.slice(existingIndex + 1),
              ]
            : [newProject, ...latestProjects];
        updatedProjects = merged.slice(0, MAX_RECENT_PROJECTS);
      } catch (error) {
        console.error('Failed to read recentProjects for write; skipping save:', error);
      }

      patch({
        currentPath: path,
        ...(updatedProjects !== null ? { recentProjects: updatedProjects } : {}),
      });

      if (updatedProjects !== null) {
        await saveRecentProjects(updatedProjects);
        await emitRecentMenuUpdate(updatedProjects);
      }

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
      let latestProjects: RecentProject[];
      try {
        latestProjects = await readLatestForWrite();
      } catch (error) {
        console.error('Failed to read recentProjects for bump; skipping:', error);
        return;
      }
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

      patch({ recentProjects: updatedProjects });

      await saveRecentProjects(updatedProjects);
      await emitRecentMenuUpdate(updatedProjects);
    },

    async closeProject() {
      patch({ currentPath: null });

      // Resize window to start screen size and center when closing a project
      try {
        await windowService.setSizeAndCenter(800, 600);
      } catch (error) {
        console.error('Failed to resize window:', error);
      }
    },

    /**
     * Set current path directly (for restoring from persistence). Unlike
     * openProject, this doesn't update recent projects.
     */
    setCurrentPath(path: string | null) {
      patch({ currentPath: path });
    },

    async refreshRecentProjectsGitInfo() {
      const current = get(mirror);

      const updatedProjects = await Promise.all(
        current.recentProjects.map(async (project) => {
          const gitBranch = await getGitBranch(project.path);
          return { ...project, gitBranch };
        })
      );

      patch({ recentProjects: updatedProjects });

      await saveRecentProjects(updatedProjects);
    },

    async removeProject(path: string) {
      let latestProjects: RecentProject[];
      try {
        latestProjects = await readLatestForWrite();
      } catch (error) {
        console.error('Failed to read recentProjects for remove; skipping:', error);
        return;
      }
      const updatedProjects = latestProjects.filter((p) => p.path !== path);

      patch({ recentProjects: updatedProjects });

      await saveRecentProjects(updatedProjects);
      await emitRecentMenuUpdate(updatedProjects);
    },

    async clearRecentProjects() {
      patch({ recentProjects: [] });

      await saveRecentProjects([]);
      await emitRecentMenuUpdate([]);
    },

    getCurrentPath(): string | null {
      return projectState.state.currentPath;
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
