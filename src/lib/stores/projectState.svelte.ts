// Canonical $state class for project state (issue #42 phase 1).
//
// See `projectStore.ts` (the facade) for the legacy export. This class
// holds only the in-memory snapshot — all disk IO, multi-window event
// fan-out, and Tauri window resizing stays in the facade, so this file
// remains free of Tauri imports and unit-testable in isolation.

export interface RecentProject {
  path: string;
  name: string;
  lastOpened: number;
  gitBranch?: string | null;
}

export interface ProjectStateShape {
  currentPath: string | null;
  recentProjects: RecentProject[];
  isLoading: boolean;
}

function initial(): ProjectStateShape {
  return {
    currentPath: null,
    recentProjects: [],
    isLoading: true,
  };
}

class ProjectState {
  state = $state<ProjectStateShape>(initial());

  patch(partial: Partial<ProjectStateShape>): void {
    this.state = { ...this.state, ...partial };
  }
}

export const projectState = new ProjectState();
