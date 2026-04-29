import { beforeEach, describe, expect, it, vi } from 'vitest';

const listeners = new Map<string, (event: { payload: unknown }) => void>();
const invokeMock = vi.fn();

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, fn: (event: { payload: unknown }) => void) => {
    listeners.set(name, fn);
    return () => listeners.delete(name);
  }),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args: unknown) => invokeMock(cmd, args),
}));

import { startCliBridge } from './cliBridge';

describe('cliBridge', () => {
  beforeEach(() => {
    listeners.clear();
    invokeMock.mockReset();
  });

  it('on cli:pane-split, dispatches to splitPane and resolves the request', async () => {
    const splitPane = vi.fn(() => 'new-pane-id');
    const indexOf = vi.fn(() => 1);

    await startCliBridge({
      label: 'window-1',
      splitPane,
      closePane: vi.fn(),
      indexOf,
      resolveFocusedPaneId: () => 'focused-id',
    });

    listeners.get('cli:pane-split')!({
      payload: { requestId: 'r1', paneId: 'p0', direction: 'horizontal' },
    });
    await Promise.resolve();

    expect(splitPane).toHaveBeenCalledWith('p0', 'horizontal');
    expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
      label: 'window-1',
      requestId: 'r1',
      payload: { newPaneId: 'new-pane-id', newPaneIndex: 1 },
    });
  });

  it('on cli:pane-close, dispatches to closePane and resolves', async () => {
    const closePane = vi.fn();
    await startCliBridge({
      label: 'window-1',
      splitPane: vi.fn(),
      closePane,
      indexOf: () => 0,
      resolveFocusedPaneId: () => null,
    });

    listeners.get('cli:pane-close')!({
      payload: { requestId: 'r2', paneId: 'p1' },
    });
    await Promise.resolve();

    expect(closePane).toHaveBeenCalledWith('p1');
    expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
      label: 'window-1',
      requestId: 'r2',
      payload: {},
    });
  });

  it('resolves "focused" via resolveFocusedPaneId', async () => {
    const splitPane = vi.fn(() => 'new');
    await startCliBridge({
      label: 'window-1',
      splitPane,
      closePane: vi.fn(),
      indexOf: () => 0,
      resolveFocusedPaneId: () => 'fp',
    });

    listeners.get('cli:pane-split')!({
      payload: { requestId: 'r', paneId: 'focused', direction: 'vertical' },
    });
    await Promise.resolve();

    expect(splitPane).toHaveBeenCalledWith('fp', 'vertical');
  });

  it('on cli:pane-close with focused but no focused pane, replies with error', async () => {
    const closePane = vi.fn();
    await startCliBridge({
      label: 'window-1',
      splitPane: vi.fn(),
      closePane,
      indexOf: () => 0,
      resolveFocusedPaneId: () => null,
    });

    listeners.get('cli:pane-close')!({
      payload: { requestId: 'rc', paneId: 'focused' },
    });
    await Promise.resolve();

    expect(closePane).not.toHaveBeenCalled();
    expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
      label: 'window-1',
      requestId: 'rc',
      payload: { error: 'no_focused_pane' },
    });
  });

  it('on cli:pane-split with focused but no focused pane, replies with error', async () => {
    const splitPane = vi.fn(() => 'never');
    await startCliBridge({
      label: 'window-1',
      splitPane,
      closePane: vi.fn(),
      indexOf: () => 0,
      resolveFocusedPaneId: () => null,
    });

    listeners.get('cli:pane-split')!({
      payload: { requestId: 'r3', paneId: 'focused', direction: 'horizontal' },
    });
    await Promise.resolve();

    expect(splitPane).not.toHaveBeenCalled();
    expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
      label: 'window-1',
      requestId: 'r3',
      payload: { error: 'no_focused_pane' },
    });
  });
});
