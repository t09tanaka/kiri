import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { remoteAccessStore, isRemoteActive } from './remoteAccessStore';

describe('remoteAccessStore', () => {
  beforeEach(() => {
    remoteAccessStore.reset();
  });

  describe('initial state', () => {
    it('should have correct defaults', () => {
      const state = get(remoteAccessStore);
      expect(state).toEqual({
        serverRunning: false,
        tunnelRunning: false,
        tunnelUrl: null,
        port: 9876,
        hasToken: false,
      });
    });
  });

  describe('setServerRunning', () => {
    it('should update serverRunning to true', () => {
      remoteAccessStore.setServerRunning(true);
      const state = get(remoteAccessStore);
      expect(state.serverRunning).toBe(true);
    });

    it('should update serverRunning to false', () => {
      remoteAccessStore.setServerRunning(true);
      remoteAccessStore.setServerRunning(false);
      const state = get(remoteAccessStore);
      expect(state.serverRunning).toBe(false);
    });

    it('should not affect other state properties', () => {
      remoteAccessStore.setPort(8080);
      remoteAccessStore.setServerRunning(true);
      const state = get(remoteAccessStore);
      expect(state.port).toBe(8080);
      expect(state.tunnelRunning).toBe(false);
    });
  });

  describe('setTunnelRunning', () => {
    it('should update tunnelRunning and tunnelUrl', () => {
      remoteAccessStore.setTunnelRunning(true, 'https://tunnel.example.com');
      const state = get(remoteAccessStore);
      expect(state.tunnelRunning).toBe(true);
      expect(state.tunnelUrl).toBe('https://tunnel.example.com');
    });

    it('should set tunnelUrl to null when no URL provided', () => {
      remoteAccessStore.setTunnelRunning(true);
      const state = get(remoteAccessStore);
      expect(state.tunnelRunning).toBe(true);
      expect(state.tunnelUrl).toBeNull();
    });

    it('should clear tunnelUrl when stopped', () => {
      remoteAccessStore.setTunnelRunning(true, 'https://tunnel.example.com');
      remoteAccessStore.setTunnelRunning(false);
      const state = get(remoteAccessStore);
      expect(state.tunnelRunning).toBe(false);
      expect(state.tunnelUrl).toBeNull();
    });

    it('should not affect other state properties', () => {
      remoteAccessStore.setServerRunning(true);
      remoteAccessStore.setTunnelRunning(true, 'https://tunnel.example.com');
      const state = get(remoteAccessStore);
      expect(state.serverRunning).toBe(true);
    });
  });

  describe('setPort', () => {
    it('should update port', () => {
      remoteAccessStore.setPort(8080);
      const state = get(remoteAccessStore);
      expect(state.port).toBe(8080);
    });

    it('should not affect other state properties', () => {
      remoteAccessStore.setServerRunning(true);
      remoteAccessStore.setPort(3000);
      const state = get(remoteAccessStore);
      expect(state.serverRunning).toBe(true);
      expect(state.port).toBe(3000);
    });
  });

  describe('setHasToken', () => {
    it('should update hasToken to true', () => {
      remoteAccessStore.setHasToken(true);
      const state = get(remoteAccessStore);
      expect(state.hasToken).toBe(true);
    });

    it('should update hasToken to false', () => {
      remoteAccessStore.setHasToken(true);
      remoteAccessStore.setHasToken(false);
      const state = get(remoteAccessStore);
      expect(state.hasToken).toBe(false);
    });
  });

  describe('reset', () => {
    it('should restore initial state', () => {
      remoteAccessStore.setServerRunning(true);
      remoteAccessStore.setTunnelRunning(true, 'https://tunnel.example.com');
      remoteAccessStore.setPort(8080);
      remoteAccessStore.setHasToken(true);

      remoteAccessStore.reset();

      const state = get(remoteAccessStore);
      expect(state).toEqual({
        serverRunning: false,
        tunnelRunning: false,
        tunnelUrl: null,
        port: 9876,
        hasToken: false,
      });
    });
  });

  describe('derived store: isRemoteActive', () => {
    it('should be false when server is not running', () => {
      expect(get(isRemoteActive)).toBe(false);
    });

    it('should be true when server is running', () => {
      remoteAccessStore.setServerRunning(true);
      expect(get(isRemoteActive)).toBe(true);
    });

    it('should update when serverRunning changes', () => {
      remoteAccessStore.setServerRunning(true);
      expect(get(isRemoteActive)).toBe(true);

      remoteAccessStore.setServerRunning(false);
      expect(get(isRemoteActive)).toBe(false);
    });
  });

  describe('subscribe', () => {
    it('should allow subscribing to store updates', () => {
      let lastValue: unknown = null;
      const unsubscribe = remoteAccessStore.subscribe((value) => {
        lastValue = value;
      });

      remoteAccessStore.setServerRunning(true);

      expect(lastValue).toMatchObject({ serverRunning: true });

      unsubscribe();
    });
  });
});
