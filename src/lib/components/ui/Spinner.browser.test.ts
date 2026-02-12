import { describe, it, expect, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import Spinner from './Spinner.svelte';

describe('Spinner Component (Browser)', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders spinner container', () => {
    const { container } = render(Spinner);

    const spinnerContainer = container.querySelector('.spinner-container');
    expect(spinnerContainer).toBeInTheDocument();
  });

  it('renders SVG element with correct size for md (default)', () => {
    const { container } = render(Spinner);

    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute('width', '24');
    expect(svg).toHaveAttribute('height', '24');
  });

  it('renders SVG element with correct size for xs', () => {
    const { container } = render(Spinner, { props: { size: 'xs' } });

    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute('width', '12');
    expect(svg).toHaveAttribute('height', '12');
  });

  it('renders SVG element with correct size for sm', () => {
    const { container } = render(Spinner, { props: { size: 'sm' } });

    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute('width', '16');
    expect(svg).toHaveAttribute('height', '16');
  });

  it('renders SVG element with correct size for lg', () => {
    const { container } = render(Spinner, { props: { size: 'lg' } });

    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute('width', '36');
    expect(svg).toHaveAttribute('height', '36');
  });

  it('applies correct size class', () => {
    const { container } = render(Spinner, { props: { size: 'lg' } });

    const spinnerContainer = container.querySelector('.spinner-container');
    expect(spinnerContainer).toHaveClass('lg');
  });

  it('has spinner glow element', () => {
    const { container } = render(Spinner);

    const glow = container.querySelector('.spinner-glow');
    expect(glow).toBeInTheDocument();
  });

  it('has spinner arc element', () => {
    const { container } = render(Spinner);

    const arc = container.querySelector('.spinner-arc');
    expect(arc).toBeInTheDocument();
  });

  it('applies custom color to spinner arc', () => {
    const customColor = '#ff0000';
    const { container } = render(Spinner, { props: { color: customColor } });

    const arc = container.querySelector('.spinner-arc');
    expect(arc).toHaveAttribute('stroke', customColor);
  });

  it('has rotating animation class on SVG', () => {
    const { container } = render(Spinner);

    const svg = container.querySelector('svg.spinner');
    expect(svg).toBeInTheDocument();
  });
});
