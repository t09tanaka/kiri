import { describe, it, expect, afterEach } from 'vitest';
import { render, screen, cleanup } from '@testing-library/svelte';
import BadgeTestWrapper from '../../../test/helpers/BadgeTestWrapper.svelte';

describe('Badge Component (Browser)', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders badge with text content', () => {
    render(BadgeTestWrapper, { props: { text: 'Test Badge' } });

    expect(screen.getByText('Test Badge')).toBeInTheDocument();
  });

  it('renders badge element with default class', () => {
    const { container } = render(BadgeTestWrapper);

    const badge = container.querySelector('.badge');
    expect(badge).toBeInTheDocument();
    expect(badge).toHaveClass('default');
  });

  it('applies success variant class', () => {
    const { container } = render(BadgeTestWrapper, { props: { variant: 'success' } });

    const badge = container.querySelector('.badge');
    expect(badge).toHaveClass('success');
  });

  it('applies warning variant class', () => {
    const { container } = render(BadgeTestWrapper, { props: { variant: 'warning' } });

    const badge = container.querySelector('.badge');
    expect(badge).toHaveClass('warning');
  });

  it('applies error variant class', () => {
    const { container } = render(BadgeTestWrapper, { props: { variant: 'error' } });

    const badge = container.querySelector('.badge');
    expect(badge).toHaveClass('error');
  });

  it('applies info variant class', () => {
    const { container } = render(BadgeTestWrapper, { props: { variant: 'info' } });

    const badge = container.querySelector('.badge');
    expect(badge).toHaveClass('info');
  });

  it('applies muted variant class', () => {
    const { container } = render(BadgeTestWrapper, { props: { variant: 'muted' } });

    const badge = container.querySelector('.badge');
    expect(badge).toHaveClass('muted');
  });

  it('applies sm size class', () => {
    const { container } = render(BadgeTestWrapper, { props: { size: 'sm' } });

    const badge = container.querySelector('.badge');
    expect(badge).toHaveClass('sm');
  });

  it('applies md size class by default', () => {
    const { container } = render(BadgeTestWrapper);

    const badge = container.querySelector('.badge');
    expect(badge).toHaveClass('md');
  });

  it('shows glow element when glow is true', () => {
    const { container } = render(BadgeTestWrapper, { props: { glow: true } });

    const badge = container.querySelector('.badge');
    const glow = container.querySelector('.badge-glow');
    expect(badge).toHaveClass('glow');
    expect(glow).toBeInTheDocument();
  });

  it('does not show glow element when glow is false', () => {
    const { container } = render(BadgeTestWrapper, { props: { glow: false } });

    const glow = container.querySelector('.badge-glow');
    expect(glow).not.toBeInTheDocument();
  });

  it('applies pulse class when pulse is true', () => {
    const { container } = render(BadgeTestWrapper, { props: { pulse: true } });

    const badge = container.querySelector('.badge');
    expect(badge).toHaveClass('pulse');
  });

  it('has badge-content element', () => {
    const { container } = render(BadgeTestWrapper, { props: { text: 'Content' } });

    const content = container.querySelector('.badge-content');
    expect(content).toBeInTheDocument();
    expect(content).toHaveTextContent('Content');
  });
});
