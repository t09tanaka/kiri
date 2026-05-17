import type { Action } from 'svelte/action';

/**
 * Svelte action that fires `onIntersect(path)` the first time the node
 * enters the viewport (plus a 200px lookahead). Used by DiffView to
 * defer rendering of off-screen file sections.
 */
export function createLazyLoadAction(
  onIntersect: (path: string) => void
): Action<HTMLElement, string> {
  return (node, path) => {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            onIntersect(path);
            observer.unobserve(node);
          }
        }
      },
      { root: null, rootMargin: '200px', threshold: 0 }
    );

    observer.observe(node);

    return {
      destroy() {
        observer.disconnect();
      },
    };
  };
}

/**
 * Tracks each file header's viewport top while it intersects the top
 * 20% of the viewport (offset by 44px to account for the sticky tabbar).
 * The handler with the smallest `top` is the "currently visible" file,
 * which DiffView feeds back into gitStore.
 */
export class VisibleHeaderTracker {
  private headers = new Map<string, number>();

  constructor(private readonly onChange: (topPath: string | null) => void) {}

  createAction(): Action<HTMLElement, string> {
    return (node, path) => {
      const observer = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (entry.isIntersecting) {
              this.headers.set(path, entry.boundingClientRect.top);
            } else {
              this.headers.delete(path);
            }
          }
          this.emitTopFile();
        },
        { root: null, rootMargin: '-44px 0px -80% 0px', threshold: 0 }
      );

      observer.observe(node);

      return {
        destroy() {
          observer.disconnect();
        },
      };
    };
  }

  private emitTopFile(): void {
    if (this.headers.size === 0) {
      this.onChange(null);
      return;
    }
    let topFile: string | null = null;
    let minTop = Infinity;
    for (const [path, top] of this.headers) {
      if (top < minTop) {
        minTop = top;
        topFile = path;
      }
    }
    this.onChange(topFile);
  }
}
