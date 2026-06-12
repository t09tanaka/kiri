import { describe, expect, test, vi } from 'vitest';
import type { Terminal as TerminalType } from '@xterm/xterm';
import type { FitAddon as FitAddonType } from '@xterm/addon-fit';
import { applyPtyRowMargin, fitTerminalToContainer, PTY_ROW_MARGIN } from './terminalLayout';

function createContainer(width: number, height: number): HTMLElement {
  return {
    getBoundingClientRect: () => ({ width, height }) as DOMRect,
  } as unknown as HTMLElement;
}

function createTerminal(cols: number, rows: number) {
  return {
    cols,
    rows,
    resize: vi.fn(),
    scrollToBottom: vi.fn(),
  };
}

function createFitAddon(dimensions: { cols: number; rows: number } | undefined) {
  return {
    proposeDimensions: vi.fn().mockReturnValue(dimensions),
  } as unknown as FitAddonType;
}

describe('applyPtyRowMargin', () => {
  test('reserves one row for the PTY backend', () => {
    expect(applyPtyRowMargin(40)).toBe(40 - PTY_ROW_MARGIN);
  });

  test('clamps to a minimum of 10 rows', () => {
    expect(applyPtyRowMargin(5)).toBe(10);
    expect(applyPtyRowMargin(0)).toBe(10);
  });
});

describe('fitTerminalToContainer', () => {
  test('skips fitting when the container is collapsed', () => {
    const terminal = createTerminal(80, 24);
    const fitAddon = createFitAddon({ cols: 100, rows: 40 });

    const result = fitTerminalToContainer(
      terminal as unknown as TerminalType,
      fitAddon,
      createContainer(1, 1)
    );

    expect(result).toBe(false);
    expect(terminal.resize).not.toHaveBeenCalled();
  });

  test('returns false when no dimensions can be proposed', () => {
    const terminal = createTerminal(80, 24);
    const fitAddon = createFitAddon(undefined);

    const result = fitTerminalToContainer(
      terminal as unknown as TerminalType,
      fitAddon,
      createContainer(800, 600)
    );

    expect(result).toBe(false);
    expect(terminal.resize).not.toHaveBeenCalled();
  });

  test('resizes when the proposed dimensions differ', () => {
    const terminal = createTerminal(80, 24);
    const fitAddon = createFitAddon({ cols: 100, rows: 40 });

    const result = fitTerminalToContainer(
      terminal as unknown as TerminalType,
      fitAddon,
      createContainer(800, 600)
    );

    expect(result).toBe(true);
    expect(terminal.resize).toHaveBeenCalledWith(100, 40);
  });

  test('does not resize when dimensions are unchanged', () => {
    const terminal = createTerminal(100, 40);
    const fitAddon = createFitAddon({ cols: 100, rows: 40 });

    fitTerminalToContainer(
      terminal as unknown as TerminalType,
      fitAddon,
      createContainer(800, 600)
    );

    expect(terminal.resize).not.toHaveBeenCalled();
  });

  test('pins to the bottom after a resize when pinToBottom is set', () => {
    const terminal = createTerminal(80, 24);
    const fitAddon = createFitAddon({ cols: 100, rows: 40 });

    fitTerminalToContainer(
      terminal as unknown as TerminalType,
      fitAddon,
      createContainer(800, 600),
      { pinToBottom: true }
    );

    expect(terminal.resize).toHaveBeenCalledWith(100, 40);
    expect(terminal.scrollToBottom).toHaveBeenCalledTimes(1);
  });

  test('does not scroll when pinToBottom is set but no resize happened', () => {
    const terminal = createTerminal(100, 40);
    const fitAddon = createFitAddon({ cols: 100, rows: 40 });

    fitTerminalToContainer(
      terminal as unknown as TerminalType,
      fitAddon,
      createContainer(800, 600),
      { pinToBottom: true }
    );

    expect(terminal.resize).not.toHaveBeenCalled();
    expect(terminal.scrollToBottom).not.toHaveBeenCalled();
  });

  test('does not scroll when pinToBottom is omitted', () => {
    const terminal = createTerminal(80, 24);
    const fitAddon = createFitAddon({ cols: 100, rows: 40 });

    fitTerminalToContainer(
      terminal as unknown as TerminalType,
      fitAddon,
      createContainer(800, 600)
    );

    expect(terminal.resize).toHaveBeenCalledWith(100, 40);
    expect(terminal.scrollToBottom).not.toHaveBeenCalled();
  });
});
