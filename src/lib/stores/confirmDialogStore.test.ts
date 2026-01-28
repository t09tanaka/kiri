import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { confirmDialogStore, isConfirmDialogOpen } from './confirmDialogStore';

describe('confirmDialogStore', () => {
  beforeEach(() => {
    // Reset store state by canceling any pending dialogs
    confirmDialogStore.handleCancel();
  });

  describe('initial state', () => {
    it('should be closed by default', () => {
      expect(get(isConfirmDialogOpen)).toBe(false);
    });

    it('should have no options by default', () => {
      const state = get(confirmDialogStore);
      expect(state.options).toBeNull();
    });
  });

  describe('confirm', () => {
    it('should open the dialog with provided options', () => {
      confirmDialogStore.confirm({
        message: 'Test message',
        title: 'Test Title',
        confirmLabel: 'Yes',
        cancelLabel: 'No',
        kind: 'warning',
      });

      const state = get(confirmDialogStore);
      expect(state.isOpen).toBe(true);
      expect(state.options?.message).toBe('Test message');
      expect(state.options?.title).toBe('Test Title');
      expect(state.options?.confirmLabel).toBe('Yes');
      expect(state.options?.cancelLabel).toBe('No');
      expect(state.options?.kind).toBe('warning');
    });

    it('should apply default values for optional fields', () => {
      confirmDialogStore.confirm({
        message: 'Test message',
      });

      const state = get(confirmDialogStore);
      expect(state.options?.title).toBe('Confirm');
      expect(state.options?.confirmLabel).toBe('OK');
      expect(state.options?.cancelLabel).toBe('Cancel');
      expect(state.options?.kind).toBe('info');
    });

    it('should return a promise', () => {
      const result = confirmDialogStore.confirm({ message: 'Test' });
      expect(result).toBeInstanceOf(Promise);
    });
  });

  describe('handleConfirm', () => {
    it('should close the dialog and resolve with true', async () => {
      const promise = confirmDialogStore.confirm({ message: 'Test' });

      // Confirm the dialog
      confirmDialogStore.handleConfirm();

      const result = await promise;
      expect(result).toBe(true);
      expect(get(isConfirmDialogOpen)).toBe(false);
    });

    it('should reset options after confirming', async () => {
      confirmDialogStore.confirm({ message: 'Test' });
      confirmDialogStore.handleConfirm();

      const state = get(confirmDialogStore);
      expect(state.options).toBeNull();
    });
  });

  describe('handleCancel', () => {
    it('should close the dialog and resolve with false', async () => {
      const promise = confirmDialogStore.confirm({ message: 'Test' });

      // Cancel the dialog
      confirmDialogStore.handleCancel();

      const result = await promise;
      expect(result).toBe(false);
      expect(get(isConfirmDialogOpen)).toBe(false);
    });

    it('should reset options after canceling', async () => {
      confirmDialogStore.confirm({ message: 'Test' });
      confirmDialogStore.handleCancel();

      const state = get(confirmDialogStore);
      expect(state.options).toBeNull();
    });
  });

  describe('isConfirmDialogOpen derived store', () => {
    it('should return true when dialog is open', () => {
      confirmDialogStore.confirm({ message: 'Test' });
      expect(get(isConfirmDialogOpen)).toBe(true);
    });

    it('should return false when dialog is closed', () => {
      confirmDialogStore.confirm({ message: 'Test' });
      confirmDialogStore.handleCancel();
      expect(get(isConfirmDialogOpen)).toBe(false);
    });
  });

  describe('multiple dialogs', () => {
    it('should handle sequential dialogs independently', async () => {
      // First dialog - confirm
      const promise1 = confirmDialogStore.confirm({ message: 'First' });
      confirmDialogStore.handleConfirm();
      const result1 = await promise1;
      expect(result1).toBe(true);

      // Second dialog - cancel
      const promise2 = confirmDialogStore.confirm({ message: 'Second' });
      confirmDialogStore.handleCancel();
      const result2 = await promise2;
      expect(result2).toBe(false);
    });
  });
});
