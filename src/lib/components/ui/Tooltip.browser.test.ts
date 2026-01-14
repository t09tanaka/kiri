import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/svelte';
import TooltipTestWrapper from '../../../test/helpers/TooltipTestWrapper.svelte';

describe('Tooltip Component (Browser)', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('renders tooltip wrapper with child content', () => {
    render(TooltipTestWrapper, { props: { text: 'Tooltip text', buttonText: 'Click me' } });

    expect(screen.getByText('Click me')).toBeInTheDocument();
  });

  it('does not show tooltip initially', () => {
    const { container } = render(TooltipTestWrapper, { props: { text: 'Tooltip text' } });

    const tooltip = container.querySelector('.tooltip');
    expect(tooltip).not.toBeInTheDocument();
  });

  it('shows tooltip after mouseenter and delay', async () => {
    const { container } = render(TooltipTestWrapper, {
      props: { text: 'Tooltip text', delay: 100 },
    });

    const wrapper = container.querySelector('.tooltip-wrapper');
    await fireEvent.mouseEnter(wrapper!);

    // Advance timers by delay amount
    vi.advanceTimersByTime(100);

    await waitFor(() => {
      const tooltip = container.querySelector('.tooltip');
      expect(tooltip).toBeInTheDocument();
    });
  });

  it('shows tooltip text content', async () => {
    const { container } = render(TooltipTestWrapper, {
      props: { text: 'My tooltip message', delay: 0 },
    });

    const wrapper = container.querySelector('.tooltip-wrapper');
    await fireEvent.mouseEnter(wrapper!);

    vi.advanceTimersByTime(0);

    await waitFor(() => {
      expect(screen.getByRole('tooltip')).toBeInTheDocument();
      expect(screen.getByText('My tooltip message')).toBeInTheDocument();
    });
  });

  it('hides tooltip on mouseleave', async () => {
    const { container } = render(TooltipTestWrapper, {
      props: { text: 'Tooltip text', delay: 0 },
    });

    const wrapper = container.querySelector('.tooltip-wrapper');

    // Show tooltip
    await fireEvent.mouseEnter(wrapper!);
    vi.advanceTimersByTime(0);

    await waitFor(() => {
      expect(container.querySelector('.tooltip')).toBeInTheDocument();
    });

    // Hide tooltip
    await fireEvent.mouseLeave(wrapper!);

    await waitFor(() => {
      expect(container.querySelector('.tooltip')).not.toBeInTheDocument();
    });
  });

  it('applies top position class by default', async () => {
    const { container } = render(TooltipTestWrapper, {
      props: { text: 'Tooltip text', delay: 0 },
    });

    const wrapper = container.querySelector('.tooltip-wrapper');
    await fireEvent.mouseEnter(wrapper!);
    vi.advanceTimersByTime(0);

    await waitFor(() => {
      const tooltip = container.querySelector('.tooltip');
      expect(tooltip).toHaveClass('top');
    });
  });

  it('applies bottom position class', async () => {
    const { container } = render(TooltipTestWrapper, {
      props: { text: 'Tooltip text', position: 'bottom', delay: 0 },
    });

    const wrapper = container.querySelector('.tooltip-wrapper');
    await fireEvent.mouseEnter(wrapper!);
    vi.advanceTimersByTime(0);

    await waitFor(() => {
      const tooltip = container.querySelector('.tooltip');
      expect(tooltip).toHaveClass('bottom');
    });
  });

  it('applies left position class', async () => {
    const { container } = render(TooltipTestWrapper, {
      props: { text: 'Tooltip text', position: 'left', delay: 0 },
    });

    const wrapper = container.querySelector('.tooltip-wrapper');
    await fireEvent.mouseEnter(wrapper!);
    vi.advanceTimersByTime(0);

    await waitFor(() => {
      const tooltip = container.querySelector('.tooltip');
      expect(tooltip).toHaveClass('left');
    });
  });

  it('applies right position class', async () => {
    const { container } = render(TooltipTestWrapper, {
      props: { text: 'Tooltip text', position: 'right', delay: 0 },
    });

    const wrapper = container.querySelector('.tooltip-wrapper');
    await fireEvent.mouseEnter(wrapper!);
    vi.advanceTimersByTime(0);

    await waitFor(() => {
      const tooltip = container.querySelector('.tooltip');
      expect(tooltip).toHaveClass('right');
    });
  });

  it('shows tooltip on focus', async () => {
    const { container } = render(TooltipTestWrapper, {
      props: { text: 'Tooltip text', delay: 0 },
    });

    const button = screen.getByRole('button');
    await fireEvent.focusIn(button);
    vi.advanceTimersByTime(0);

    await waitFor(() => {
      expect(container.querySelector('.tooltip')).toBeInTheDocument();
    });
  });

  it('hides tooltip on blur', async () => {
    const { container } = render(TooltipTestWrapper, {
      props: { text: 'Tooltip text', delay: 0 },
    });

    const button = screen.getByRole('button');

    // Show tooltip
    await fireEvent.focusIn(button);
    vi.advanceTimersByTime(0);

    await waitFor(() => {
      expect(container.querySelector('.tooltip')).toBeInTheDocument();
    });

    // Hide tooltip
    await fireEvent.focusOut(button);

    await waitFor(() => {
      expect(container.querySelector('.tooltip')).not.toBeInTheDocument();
    });
  });
});
