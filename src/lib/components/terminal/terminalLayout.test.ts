import { describe, expect, test, vi } from 'vitest';
import type { Terminal as TerminalType } from '@xterm/xterm';
import type { FitAddon as FitAddonType } from '@xterm/addon-fit';
import {
  applyPtyRowMargin,
  distributeDividerDrag,
  fitTerminalToContainer,
  PTY_ROW_MARGIN,
} from './terminalLayout';
import { MIN_PANE_SIZE_PX } from './terminalConstants';

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

describe('distributeDividerDrag', () => {
  const CONTAINER = 1000; // px — MIN_PANE_SIZE_PX is 7.2% of this

  test('splits two panes at the divider position', () => {
    expect(distributeDividerDrag([50, 50], 0, 30, CONTAINER)).toEqual([30, 70]);
  });

  test('re-distributes three panes proportionally around the dragged divider', () => {
    const result = distributeDividerDrag([20, 40, 40], 1, 75, CONTAINER);
    expect(result).not.toBeNull();
    expect(result![0] + result![1]).toBeCloseTo(75);
    expect(result![2]).toBeCloseTo(25);
    // The two panes before the divider keep their 1:2 ratio.
    expect(result![1] / result![0]).toBeCloseTo(2);
  });

  test('clamps a two-pane drag to the minimum pane size', () => {
    const minPercent = (MIN_PANE_SIZE_PX / CONTAINER) * 100;

    const draggedLeft = distributeDividerDrag([50, 50], 0, 1, CONTAINER);
    expect(draggedLeft![0]).toBeCloseTo(minPercent);
    expect(draggedLeft![1]).toBeCloseTo(100 - minPercent);

    const draggedRight = distributeDividerDrag([50, 50], 0, 99, CONTAINER);
    expect(draggedRight![0]).toBeCloseTo(100 - minPercent);
    expect(draggedRight![1]).toBeCloseTo(minPercent);
  });

  test('rejects a drag that would push any pane below the minimum size', () => {
    // Dragging the first divider to 1% would collapse the leading pane.
    expect(distributeDividerDrag([33, 33, 34], 0, 1, CONTAINER)).toBeNull();
    // ...and to 99% would collapse the trailing one.
    expect(distributeDividerDrag([33, 33, 34], 0, 99, CONTAINER)).toBeNull();
  });

  test('drops the minimum once a split holds more panes than it can fit', () => {
    // 20 panes in a 1000px container: the 72px floor cannot be met, so the
    // divider must stay draggable rather than freeze.
    const sizes = Array.from({ length: 20 }, () => 5);
    const result = distributeDividerDrag(sizes, 0, 1, CONTAINER);
    expect(result).not.toBeNull();
    expect(result![0]).toBeCloseTo(1);
  });

  test('treats a zero-size container as having no minimum', () => {
    expect(distributeDividerDrag([50, 50], 0, 2, 0)).toEqual([2, 98]);
  });
});
