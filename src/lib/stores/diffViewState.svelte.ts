// Canonical $state class for the diff view modal (issue #42 phase 1).
// See `diffViewStore.ts` (the facade) for the legacy export.

export interface DiffViewStateShape {
  isOpen: boolean;
  projectPath: string | null;
}

class DiffViewState {
  state = $state<DiffViewStateShape>({ isOpen: false, projectPath: null });

  open(projectPath: string): void {
    this.state = { isOpen: true, projectPath };
  }

  close(): void {
    this.state = { isOpen: false, projectPath: null };
  }
}

export const diffViewState = new DiffViewState();
