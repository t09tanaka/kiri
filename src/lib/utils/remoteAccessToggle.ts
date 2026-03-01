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
      remoteAccessStore.setAuthToken(null);
      settings.enabled = false;
      await saveRemoteAccessSettings(settings);
      return null;
    } else {
      opts.onError('');

      // cloudflared is REQUIRED for Remote Access.
      // Do NOT add a LAN-only fallback — simplicity over flexibility.
      const cloudflaredAvailable = await remoteAccessService.isCloudflaredAvailable();
      if (!cloudflaredAvailable) {
        opts.onError('cloudflared is not installed. Run: brew install cloudflared');
        return null;
      }

      // Start server
      const authToken = await remoteAccessService.startServer(settings.port);
      remoteAccessStore.setServerRunning(true);
      remoteAccessStore.setPort(settings.port);
      remoteAccessStore.setAuthToken(authToken);
      settings.enabled = true;

      // Start tunnel (required — if tunnel fails, the entire Remote Access startup fails)
      let tunnelUrl: string | null = null;
      try {
        const token = settings.tunnelToken?.trim() || null;
        tunnelUrl = await remoteAccessService.startTunnel(token, settings.port);
        remoteAccessStore.setTunnelRunning(true, tunnelUrl ?? undefined);
      } catch {
        // Tunnel failed - stop server and clean up
        try {
          await remoteAccessService.stopServer();
        } catch {
          // Ignore cleanup errors
        }
        remoteAccessStore.setServerRunning(false);
        remoteAccessStore.setTunnelRunning(false);
        remoteAccessStore.setAuthToken(null);
        settings.enabled = false;
        await saveRemoteAccessSettings(settings);
        opts.onError('Failed to start Cloudflare Tunnel');
        return null;
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
