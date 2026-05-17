import { getContext, setContext } from 'svelte';

export type FileTreeAction = 'rename' | 'delete' | 'new-file' | 'new-folder';

export interface FileTreeRegistry {
  registerAction(path: string, handler: (action: FileTreeAction) => void): () => void;
  registerAutoExpand(path: string, handler: () => void): () => void;
}

const FILETREE_REGISTRY_KEY = Symbol('fileTreeRegistry');

export function createFileTreeRegistry(): {
  registry: FileTreeRegistry;
  dispatchAction(path: string, action: FileTreeAction): void;
  dispatchAutoExpand(path: string): void;
} {
  const actionHandlers = new Map<string, (action: FileTreeAction) => void>();
  const autoExpandHandlers = new Map<string, () => void>();

  const registry: FileTreeRegistry = {
    registerAction(path, handler) {
      actionHandlers.set(path, handler);
      return () => {
        if (actionHandlers.get(path) === handler) {
          actionHandlers.delete(path);
        }
      };
    },
    registerAutoExpand(path, handler) {
      autoExpandHandlers.set(path, handler);
      return () => {
        if (autoExpandHandlers.get(path) === handler) {
          autoExpandHandlers.delete(path);
        }
      };
    },
  };

  return {
    registry,
    dispatchAction(path, action) {
      actionHandlers.get(path)?.(action);
    },
    dispatchAutoExpand(path) {
      autoExpandHandlers.get(path)?.();
    },
  };
}

export function provideFileTreeRegistry(registry: FileTreeRegistry): void {
  setContext(FILETREE_REGISTRY_KEY, registry);
}

export function useFileTreeRegistry(): FileTreeRegistry | null {
  return getContext<FileTreeRegistry | null>(FILETREE_REGISTRY_KEY) ?? null;
}
