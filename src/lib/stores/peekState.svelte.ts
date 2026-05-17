// Canonical $state class for the peek editor (issue #42 phase 1).
// See `peekStore.ts` (the facade) for the legacy export.

export interface PeekStateShape {
  isOpen: boolean;
  filePath: string | null;
  lineNumber?: number;
  columnNumber?: number;
}

function initial(): PeekStateShape {
  return {
    isOpen: false,
    filePath: null,
    lineNumber: undefined,
    columnNumber: undefined,
  };
}

class PeekState {
  state = $state<PeekStateShape>(initial());

  open(filePath: string, lineNumber?: number, columnNumber?: number): void {
    this.state = { isOpen: true, filePath, lineNumber, columnNumber };
  }

  close(): void {
    this.state = initial();
  }
}

export const peekState = new PeekState();
