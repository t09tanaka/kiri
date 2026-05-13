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
      setPaneCollapsed: vi.fn(),
    });

    listeners.get('cli:pane-split')!({
      payload: { requestId: 'r1', paneId: 'p0', direction: 'horizontal' },
    });
    await Promise.resolve();

    expect(splitPane).toHaveBeenCalledWith('p0', 'horizontal', {
      name: undefined,
      color: undefined,
    });
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
      setPaneCollapsed: vi.fn(),
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
      setPaneCollapsed: vi.fn(),
    });

    listeners.get('cli:pane-split')!({
      payload: { requestId: 'r', paneId: 'focused', direction: 'vertical' },
    });
    await Promise.resolve();

    expect(splitPane).toHaveBeenCalledWith('fp', 'vertical', {
      name: undefined,
      color: undefined,
    });
  });

  it('on cli:pane-close with focused but no focused pane, replies with error', async () => {
    const closePane = vi.fn();
    await startCliBridge({
      label: 'window-1',
      splitPane: vi.fn(),
      closePane,
      indexOf: () => 0,
      resolveFocusedPaneId: () => null,
      setPaneCollapsed: vi.fn(),
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
      setPaneCollapsed: vi.fn(),
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

  it('on cli:pane-split with name/color, passes them to splitPane', async () => {
    const splitPane = vi.fn(() => 'new-pane-id');
    const indexOf = vi.fn(() => 2);

    await startCliBridge({
      label: 'window-1',
      splitPane,
      closePane: vi.fn(),
      indexOf,
      resolveFocusedPaneId: () => 'focused-id',
      setPaneCollapsed: vi.fn(),
    });

    listeners.get('cli:pane-split')!({
      payload: {
        requestId: 'r4',
        paneId: 'p0',
        direction: 'horizontal',
        name: 'build',
        color: 'jade',
      },
    });
    await Promise.resolve();

    expect(splitPane).toHaveBeenCalledWith('p0', 'horizontal', {
      name: 'build',
      color: 'jade',
    });
    expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
      label: 'window-1',
      requestId: 'r4',
      payload: { newPaneId: 'new-pane-id', newPaneIndex: 2 },
    });
  });

  it('on cli:pane-split with neither name nor color, omits opts cleanly', async () => {
    const splitPane = vi.fn(() => 'new-pane-id');

    await startCliBridge({
      label: 'window-1',
      splitPane,
      closePane: vi.fn(),
      indexOf: () => 0,
      resolveFocusedPaneId: () => 'focused-id',
      setPaneCollapsed: vi.fn(),
    });

    listeners.get('cli:pane-split')!({
      payload: { requestId: 'r5', paneId: 'p0', direction: 'vertical' },
    });
    await Promise.resolve();

    expect(splitPane).toHaveBeenCalledWith('p0', 'vertical', {
      name: undefined,
      color: undefined,
    });
  });

  it('on cli:pane-minimize, calls setPaneCollapsed and resolves', async () => {
    const setPaneCollapsed = vi.fn();
    await startCliBridge({
      label: 'main',
      splitPane: vi.fn().mockReturnValue('pane-2'),
      closePane: vi.fn(),
      indexOf: vi.fn().mockReturnValue(1),
      resolveFocusedPaneId: () => 'pane-1',
      setPaneCollapsed,
    });

    listeners.get('cli:pane-minimize')!({
      payload: { requestId: 'r1', paneId: 'pane-1', minimized: true },
    });

    expect(setPaneCollapsed).toHaveBeenCalledWith('pane-1', true);
    expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
      label: 'main',
      requestId: 'r1',
      payload: {},
    });
  });

  it('on cli:pane-minimize with focused but no focused pane, replies error', async () => {
    const setPaneCollapsed = vi.fn();
    await startCliBridge({
      label: 'main',
      splitPane: vi.fn(),
      closePane: vi.fn(),
      indexOf: vi.fn(),
      resolveFocusedPaneId: () => null,
      setPaneCollapsed,
    });

    listeners.get('cli:pane-minimize')!({
      payload: { requestId: 'r2', paneId: 'focused', minimized: false },
    });

    expect(setPaneCollapsed).not.toHaveBeenCalled();
    expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
      label: 'main',
      requestId: 'r2',
      payload: { error: 'no_focused_pane' },
    });
  });

  it('on cli:pane-split with minimized=true, sets new pane collapsed before resolving', async () => {
    const setPaneCollapsed = vi.fn();
    const splitPane = vi.fn().mockReturnValue('pane-new');
    await startCliBridge({
      label: 'main',
      splitPane,
      closePane: vi.fn(),
      indexOf: vi.fn().mockReturnValue(2),
      resolveFocusedPaneId: () => 'pane-1',
      setPaneCollapsed,
    });

    listeners.get('cli:pane-split')!({
      payload: { requestId: 'r3', paneId: 'pane-1', direction: 'horizontal', minimized: true },
    });

    expect(splitPane).toHaveBeenCalledWith('pane-1', 'horizontal', {
      name: undefined,
      color: undefined,
    });
    expect(setPaneCollapsed).toHaveBeenCalledWith('pane-new', true);
    expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
      label: 'main',
      requestId: 'r3',
      payload: { newPaneId: 'pane-new', newPaneIndex: 2 },
    });
  });

  it('on cli:pane-split without minimized, does not touch setPaneCollapsed', async () => {
    const setPaneCollapsed = vi.fn();
    await startCliBridge({
      label: 'main',
      splitPane: vi.fn().mockReturnValue('pane-new'),
      closePane: vi.fn(),
      indexOf: vi.fn().mockReturnValue(2),
      resolveFocusedPaneId: () => 'pane-1',
      setPaneCollapsed,
    });

    listeners.get('cli:pane-split')!({
      payload: { requestId: 'r4', paneId: 'pane-1', direction: 'horizontal' },
    });

    expect(setPaneCollapsed).not.toHaveBeenCalled();
  });
});
