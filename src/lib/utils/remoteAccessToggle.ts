import {
  loadRemoteAccessSettings,
  saveRemoteAccessSettings,
} from '@/lib/services/persistenceService';
import { remoteAccessService } from '@/lib/services/remoteAccessService';
import { remoteAccessStore, isRemoteActive } from '@/lib/stores/remoteAccessStore';
import { get } from 'svelte/store';

export interface ToggleRemoteOptions {
  onToggling: (toggling: boolean) => void;
  onError: (message: string) => void;
  onServerStarted?: () => void;
}

/**
 * Toggle remote access on/off.
 * Returns { tunnelUrl } on successful ON, null on OFF or error.
 */
export async function toggleRemoteAccess(
  opts: ToggleRemoteOptions
): Promise<{ tunnelUrl: string | null } | null> {
  opts.onToggling(true);
  try {
    const settings = await loadRemoteAccessSettings();
    const active = get(isRemoteActive);

    if (active) {
      // Stop tunnel first, then server
      opts.onError('');
      try {
        await remoteAccessService.stopTunnel();
      } catch {
        // Tunnel may not be running
      }
      remoteAccessStore.setTunnelRunning(false);
      await remoteAccessService.stopServer();
      remoteAccessStore.setServerRunning(false);
      settings.enabled = false;
      await saveRemoteAccessSettings(settings);
      return null;
    } else {
      // Check if cloudflared is available before starting
      const cloudflaredAvailable = await remoteAccessService.isCloudflaredAvailable();
      if (!cloudflaredAvailable) {
        opts.onError('cloudflared is not installed. Run: brew install cloudflared');
        return null;
      }
      opts.onError('');

      // Start server, then tunnel
      await remoteAccessService.startServer(settings.port);
      remoteAccessStore.setServerRunning(true);
      remoteAccessStore.setPort(settings.port);
      remoteAccessStore.setHasToken(true);
      settings.enabled = true;

      // Notify caller that server is ready (before tunnel starts)
      opts.onServerStarted?.();

      let tunnelUrl: string | null = null;
      try {
        const token = settings.tunnelToken?.trim() || null;
        tunnelUrl = await remoteAccessService.startTunnel(token, settings.port);
        remoteAccessStore.setTunnelRunning(true, tunnelUrl ?? undefined);
      } catch {
        // Tunnel failed - server stays running
        remoteAccessStore.setTunnelRunning(false);
      }

      await saveRemoteAccessSettings(settings);
      return { tunnelUrl };
    }
  } catch (error) {
    console.error('Failed to toggle remote access:', error);
    return null;
  } finally {
    opts.onToggling(false);
  }
}
