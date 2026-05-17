/**
 * KIRI Mist Theme - soft atmospheric terminal colors.
 *
 * Colors are chosen to read against the dark `--bg-primary` background
 * used by the rest of the app. ANSI mapping follows the standard xterm
 * positions; `*Bright` variants are tuned to the same hue family as
 * their base so applications that switch palettes stay coherent.
 */
export const mistTheme = {
  background: '#0a0c10',
  foreground: '#c8d3e0',
  cursor: '#7dd3fc',
  cursorAccent: '#0a0c10',
  selectionBackground: 'rgba(125, 211, 252, 0.2)',
  selectionForeground: '#f0f4f8',
  black: '#0e1218',
  red: '#f87171',
  green: '#4ade80',
  yellow: '#fbbf24',
  blue: '#7dd3fc',
  magenta: '#c4b5fd',
  cyan: '#67e8f9',
  white: '#c8d3e0',
  brightBlack: '#5c6b7a',
  brightRed: '#fca5a5',
  brightGreen: '#86efac',
  brightYellow: '#fcd34d',
  brightBlue: '#93c5fd',
  brightMagenta: '#d8b4fe',
  brightCyan: '#a5f3fc',
  brightWhite: '#f0f4f8',
} as const;
