// Transient UI preferences (issue #43 — split from settingsStore).
//
// Responsibility: in-memory UI state that the user tunes during a session
// (font size today; theme, accent color, etc. as we add them). Persistence
// happens through the same kiri-settings.json file, but the store itself
// does not own that — the App wires up auto-persist at boot.
//
// Anything that needs *disk-backed* configuration the user expects to
// survive across windows / restarts (startup command, recent paths) lives
// in `persistedSettingsStore` instead. Keep them separate so a UI-only
// preference change does not trigger a persistence migration discussion,
// and vice versa.

const MIN_FONT_SIZE = 8;
const MAX_FONT_SIZE = 32;
const DEFAULT_FONT_SIZE = 13;
const ZOOM_STEP = 1;

export const FONT_SIZE_CONSTRAINTS = {
  MIN: MIN_FONT_SIZE,
  MAX: MAX_FONT_SIZE,
  DEFAULT: DEFAULT_FONT_SIZE,
  STEP: ZOOM_STEP,
};

class UiPreferencesStore {
  fontSize = $state<number>(DEFAULT_FONT_SIZE);

  zoomIn(): void {
    this.fontSize = Math.min(this.fontSize + ZOOM_STEP, MAX_FONT_SIZE);
  }

  zoomOut(): void {
    this.fontSize = Math.max(this.fontSize - ZOOM_STEP, MIN_FONT_SIZE);
  }

  resetZoom(): void {
    this.fontSize = DEFAULT_FONT_SIZE;
  }

  setFontSize(size: number): void {
    this.fontSize = Math.max(MIN_FONT_SIZE, Math.min(size, MAX_FONT_SIZE));
  }

  reset(): void {
    this.fontSize = DEFAULT_FONT_SIZE;
  }
}

export const uiPreferencesStore = new UiPreferencesStore();
