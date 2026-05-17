/**
 * Floating ghost element shown beside the cursor during an internal
 * (mouse-based) file move drag. The ghost lives directly on `document.body`
 * so it can render above the file tree and any panes without affecting
 * tree layout / scroll.
 */
export class DragGhost {
  private element: HTMLDivElement | null = null;

  create(sourcePath: string): void {
    const name = sourcePath.split('/').pop() || sourcePath;
    this.element = document.createElement('div');
    this.element.className = 'drag-ghost';
    this.element.textContent = name;
    document.body.appendChild(this.element);
  }

  move(x: number, y: number): void {
    if (!this.element) return;
    this.element.style.left = `${x + 12}px`;
    this.element.style.top = `${y - 10}px`;
  }

  setValid(valid: boolean): void {
    if (!this.element) return;
    this.element.classList.toggle('invalid', !valid);
  }

  remove(): void {
    if (this.element) {
      this.element.remove();
      this.element = null;
    }
  }
}
