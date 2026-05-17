// Canonical $state class for the focused-pane tracker (issue #42 phase 1).
// See `focusedPaneStore.ts` (the facade) for the legacy export.

class FocusedPaneState {
  paneId = $state<string | null>(null);

  set(id: string | null): void {
    this.paneId = id;
  }

  current(): string | null {
    return this.paneId;
  }
}

export const focusedPaneState = new FocusedPaneState();
