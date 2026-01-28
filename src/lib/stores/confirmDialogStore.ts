import { writable, derived } from 'svelte/store';

export interface ConfirmDialogOptions {
  title?: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  kind?: 'info' | 'warning' | 'error';
}

interface ConfirmDialogState {
  isOpen: boolean;
  options: ConfirmDialogOptions | null;
  resolve: ((value: boolean) => void) | null;
}

const initialState: ConfirmDialogState = {
  isOpen: false,
  options: null,
  resolve: null,
};

function createConfirmDialogStore() {
  const { subscribe, set, update } = writable<ConfirmDialogState>(initialState);

  return {
    subscribe,

    /**
     * Show a confirmation dialog and return a promise that resolves to the user's choice
     */
    confirm: (options: ConfirmDialogOptions): Promise<boolean> => {
      return new Promise((resolve) => {
        set({
          isOpen: true,
          options: {
            title: 'Confirm',
            confirmLabel: 'OK',
            cancelLabel: 'Cancel',
            kind: 'info',
            ...options,
          },
          resolve,
        });
      });
    },

    /**
     * Handle user confirmation (OK button clicked)
     */
    handleConfirm: () => {
      update((state) => {
        state.resolve?.(true);
        return initialState;
      });
    },

    /**
     * Handle user cancellation (Cancel button or Esc key)
     */
    handleCancel: () => {
      update((state) => {
        state.resolve?.(false);
        return initialState;
      });
    },
  };
}

export const confirmDialogStore = createConfirmDialogStore();

export const isConfirmDialogOpen = derived(confirmDialogStore, ($state) => $state.isOpen);
