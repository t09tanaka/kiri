// Canonical $state class for the editor modal (issue #42 phase 1).
// See `editorModalStore.ts` (the facade) for the legacy export.

export interface EditorModalStateShape {
  isOpen: boolean;
  filePath: string | null;
}

class EditorModalState {
  state = $state<EditorModalStateShape>({ isOpen: false, filePath: null });

  open(filePath: string): void {
    this.state = { isOpen: true, filePath };
  }

  close(): void {
    this.state = { isOpen: false, filePath: null };
  }
}

export const editorModalState = new EditorModalState();
