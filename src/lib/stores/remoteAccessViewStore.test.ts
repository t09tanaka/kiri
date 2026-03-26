import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { remoteAccessViewStore } from './remoteAccessViewStore';

describe('remoteAccessViewStore', () => {
  beforeEach(() => {
    // Reset to default state
    remoteAccessViewStore.closeSettings();
    remoteAccessViewStore.closeQrModal();
  });

  it('should have default state with both modals closed', () => {
    const state = get(remoteAccessViewStore);
    expect(state.isSettingsOpen).toBe(false);
    expect(state.isQrModalOpen).toBe(false);
  });

  it('should open settings modal', () => {
    remoteAccessViewStore.openSettings();
    const state = get(remoteAccessViewStore);
    expect(state.isSettingsOpen).toBe(true);
    expect(state.isQrModalOpen).toBe(false);
  });

  it('should close settings modal', () => {
    remoteAccessViewStore.openSettings();
    remoteAccessViewStore.closeSettings();
    const state = get(remoteAccessViewStore);
    expect(state.isSettingsOpen).toBe(false);
  });

  it('should toggle settings modal', () => {
    remoteAccessViewStore.toggleSettings();
    expect(get(remoteAccessViewStore).isSettingsOpen).toBe(true);
    remoteAccessViewStore.toggleSettings();
    expect(get(remoteAccessViewStore).isSettingsOpen).toBe(false);
  });

  it('should open QR modal', () => {
    remoteAccessViewStore.openQrModal();
    const state = get(remoteAccessViewStore);
    expect(state.isQrModalOpen).toBe(true);
    expect(state.isSettingsOpen).toBe(false);
  });

  it('should close QR modal', () => {
    remoteAccessViewStore.openQrModal();
    remoteAccessViewStore.closeQrModal();
    expect(get(remoteAccessViewStore).isQrModalOpen).toBe(false);
  });

  it('should allow both modals to be open simultaneously', () => {
    remoteAccessViewStore.openSettings();
    remoteAccessViewStore.openQrModal();
    const state = get(remoteAccessViewStore);
    expect(state.isSettingsOpen).toBe(true);
    expect(state.isQrModalOpen).toBe(true);
  });

  it('should close each modal independently', () => {
    remoteAccessViewStore.openSettings();
    remoteAccessViewStore.openQrModal();
    remoteAccessViewStore.closeSettings();
    const state = get(remoteAccessViewStore);
    expect(state.isSettingsOpen).toBe(false);
    expect(state.isQrModalOpen).toBe(true);
  });
});
