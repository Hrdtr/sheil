import type { ITheme } from '@xterm/xterm';

/** Cursor shape rendered by xterm.js. */
type CursorStyle = 'block' | 'underline' | 'bar';

/** Identifier for a bundled color scheme preset. */
type ColorSchemeId =
  | 'catppuccin-mocha'
  | 'catppuccin-latte'
  | 'dracula'
  | 'nord'
  | 'solarized-dark'
  | 'solarized-light'
  | 'github-dark'
  | 'one-dark'
  | 'tokyo-night';

/** A named color scheme that maps directly onto xterm.js's `ITheme`. */
interface ColorScheme {
  id: ColorSchemeId;
  name: string;
  theme: ITheme;
}

/**
 * Runtime-configurable terminal appearance. Persisted to SQLite in Phase 1.5.
 */
interface Appearance {
  /** Active color scheme id. */
  colorSchemeId: ColorSchemeId;
  /** Font size in pixels. */
  fontSize: number;
  /** CSS font-family stack. */
  fontFamily: string;
  /** Normal font weight (0 = inherit from `font-family`). */
  fontWeight: number;
  /** Bold font weight (0 = inherit from `font-family`). */
  fontWeightBold: number;
  /** Line height multiplier (1.0 = single). */
  lineHeight: number;
  /** Cursor shape. */
  cursorStyle: CursorStyle;
  /** Whether the cursor blinks. */
  cursorBlink: boolean;
  /** Scrollback buffer size in lines. */
  scrollback: number;
}

/**
 * Bundled color scheme presets. Each maps directly onto xterm.js `ITheme`.
 *
 * Sources: official palette repos (MIT/CC0). Hex values are the canonical
 * reference palettes — do not tweak ad-hoc.
 */
const colorSchemes: readonly ColorScheme[] = [
  {
    id: 'catppuccin-mocha',
    name: 'Catppuccin Mocha',
    theme: {
      background: '#1e1e2e',
      foreground: '#cdd6f4',
      cursor: '#f5e0dc',
      cursorAccent: '#1e1e2e',
      selectionBackground: '#585b70',
      selectionForeground: '#cdd6f4',
      black: '#45475a',
      red: '#f38ba8',
      green: '#a6e3a1',
      yellow: '#f9e2af',
      blue: '#89b4fa',
      magenta: '#f5c2e7',
      cyan: '#94e2d5',
      white: '#bac2de',
      brightBlack: '#585b70',
      brightRed: '#f38ba8',
      brightGreen: '#a6e3a1',
      brightYellow: '#f9e2af',
      brightBlue: '#89b4fa',
      brightMagenta: '#f5c2e7',
      brightCyan: '#94e2d5',
      brightWhite: '#a6adc8',
    },
  },
  {
    id: 'catppuccin-latte',
    name: 'Catppuccin Latte',
    theme: {
      background: '#eff1f5',
      foreground: '#4c4f69',
      cursor: '#dc8a78',
      cursorAccent: '#eff1f5',
      selectionBackground: '#acb0be',
      selectionForeground: '#4c4f69',
      black: '#5c5f77',
      red: '#d20f39',
      green: '#40a02b',
      yellow: '#df8e1d',
      blue: '#1e66f5',
      magenta: '#ea76cb',
      cyan: '#179299',
      white: '#acb0be',
      brightBlack: '#6c6f85',
      brightRed: '#d20f39',
      brightGreen: '#40a02b',
      brightYellow: '#df8e1d',
      brightBlue: '#1e66f5',
      brightMagenta: '#ea76cb',
      brightCyan: '#179299',
      brightWhite: '#bcc0cc',
    },
  },
  {
    id: 'dracula',
    name: 'Dracula',
    theme: {
      background: '#282a36',
      foreground: '#f8f8f2',
      cursor: '#f8f8f0',
      cursorAccent: '#282a36',
      selectionBackground: '#44475a',
      selectionForeground: '#f8f8f2',
      black: '#21222c',
      red: '#ff5555',
      green: '#50fa7b',
      yellow: '#f1fa8c',
      blue: '#bd93f9',
      magenta: '#ff79c6',
      cyan: '#8be9fd',
      white: '#f8f8f2',
      brightBlack: '#6272a4',
      brightRed: '#ff6e6e',
      brightGreen: '#69ff94',
      brightYellow: '#ffffa5',
      brightBlue: '#d6acff',
      brightMagenta: '#ff92df',
      brightCyan: '#a4ffff',
      brightWhite: '#ffffff',
    },
  },
  {
    id: 'nord',
    name: 'Nord',
    theme: {
      background: '#2e3440',
      foreground: '#d8dee9',
      cursor: '#d8dee9',
      cursorAccent: '#2e3440',
      selectionBackground: '#434c5e',
      selectionForeground: '#d8dee9',
      black: '#3b4252',
      red: '#bf616a',
      green: '#a3be8c',
      yellow: '#ebcb8b',
      blue: '#81a1c1',
      magenta: '#b48ead',
      cyan: '#88c0d0',
      white: '#e5e9f0',
      brightBlack: '#4c566a',
      brightRed: '#bf616a',
      brightGreen: '#a3be8c',
      brightYellow: '#ebcb8b',
      brightBlue: '#81a1c1',
      brightMagenta: '#b48ead',
      brightCyan: '#8fbcbb',
      brightWhite: '#eceff4',
    },
  },
  {
    id: 'solarized-dark',
    name: 'Solarized Dark',
    theme: {
      background: '#002b36',
      foreground: '#839496',
      cursor: '#93a1a1',
      cursorAccent: '#002b36',
      selectionBackground: '#073642',
      selectionForeground: '#93a1a1',
      black: '#073642',
      red: '#dc322f',
      green: '#859900',
      yellow: '#b58900',
      blue: '#268bd2',
      magenta: '#d33682',
      cyan: '#2aa198',
      white: '#eee8d5',
      brightBlack: '#586e75',
      brightRed: '#cb4b16',
      brightGreen: '#586e75',
      brightYellow: '#657b83',
      brightBlue: '#839496',
      brightMagenta: '#6c71c4',
      brightCyan: '#93a1a1',
      brightWhite: '#fdf6e3',
    },
  },
  {
    id: 'solarized-light',
    name: 'Solarized Light',
    theme: {
      background: '#fdf6e3',
      foreground: '#657b83',
      cursor: '#586e75',
      cursorAccent: '#fdf6e3',
      selectionBackground: '#eee8d5',
      selectionForeground: '#586e75',
      black: '#073642',
      red: '#dc322f',
      green: '#859900',
      yellow: '#b58900',
      blue: '#268bd2',
      magenta: '#d33682',
      cyan: '#2aa198',
      white: '#eee8d5',
      brightBlack: '#002b36',
      brightRed: '#cb4b16',
      brightGreen: '#586e75',
      brightYellow: '#657b83',
      brightBlue: '#839496',
      brightMagenta: '#6c71c4',
      brightCyan: '#93a1a1',
      brightWhite: '#fdf6e3',
    },
  },
  {
    id: 'github-dark',
    name: 'GitHub Dark',
    theme: {
      background: '#0d1117',
      foreground: '#c9d1d9',
      cursor: '#6495ed',
      cursorAccent: '#0d1117',
      selectionBackground: '#1f6feb55',
      selectionForeground: '#c9d1d9',
      black: '#484f58',
      red: '#ff7b72',
      green: '#3fb950',
      yellow: '#d29922',
      blue: '#58a6ff',
      magenta: '#bc8cff',
      cyan: '#39c5cf',
      white: '#b1bac4',
      brightBlack: '#6e7681',
      brightRed: '#ffa198',
      brightGreen: '#56d364',
      brightYellow: '#e3b341',
      brightBlue: '#79c0ff',
      brightMagenta: '#d2a8ff',
      brightCyan: '#56d4dd',
      brightWhite: '#f0f6fc',
    },
  },
  {
    id: 'one-dark',
    name: 'One Dark',
    theme: {
      background: '#282c34',
      foreground: '#abb2bf',
      cursor: '#abb2bf',
      cursorAccent: '#282c34',
      selectionBackground: '#3e4451',
      selectionForeground: '#abb2bf',
      black: '#282c34',
      red: '#e06c75',
      green: '#98c379',
      yellow: '#e5c07b',
      blue: '#61afef',
      magenta: '#c678dd',
      cyan: '#56b6c2',
      white: '#abb2bf',
      brightBlack: '#5c6370',
      brightRed: '#e06c75',
      brightGreen: '#98c379',
      brightYellow: '#e5c07b',
      brightBlue: '#61afef',
      brightMagenta: '#c678dd',
      brightCyan: '#56b6c2',
      brightWhite: '#ffffff',
    },
  },
  {
    id: 'tokyo-night',
    name: 'Tokyo Night',
    theme: {
      background: '#1a1b26',
      foreground: '#a9b1d6',
      cursor: '#c0caf5',
      cursorAccent: '#1a1b26',
      selectionBackground: '#33467c',
      selectionForeground: '#c0caf5',
      black: '#15161e',
      red: '#f7768e',
      green: '#9ece6a',
      yellow: '#e0af68',
      blue: '#7aa2f7',
      magenta: '#bb9af7',
      cyan: '#7dcfff',
      white: '#a9b1d6',
      brightBlack: '#414868',
      brightRed: '#f7768e',
      brightGreen: '#9ece6a',
      brightYellow: '#e0af68',
      brightBlue: '#7aa2f7',
      brightMagenta: '#bb9af7',
      brightCyan: '#7dcfff',
      brightWhite: '#c0caf5',
    },
  },
] as const;

/** Lookup map: {@link ColorSchemeId} → {@link ColorScheme}. Built from {@link colorSchemes}. */
const colorSchemeIndex: ReadonlyMap<ColorSchemeId, ColorScheme> = new Map(
  colorSchemes.map((scheme) => [scheme.id, scheme]),
);

/**
 * Look up a color scheme by id. Falls back to Catppuccin Mocha (the default)
 * if the id is unknown so the terminal always has a usable palette.
 */
function getColorScheme(id: ColorSchemeId): ColorScheme {
  return colorSchemeIndex.get(id) ?? colorSchemeIndex.get('catppuccin-mocha')!;
}

/**
 * Composable for terminal appearance settings.
 *
 * Exposes reactive state (backed by `useState`) for the terminal's
 * color scheme, font, cursor, and line-height settings, along with
 * bounds, options arrays for select inputs, and clamp helpers.
 *
 * Settings are stored in Nuxt state under the key
 * `"terminal-settings:appearance"` so they survive HMR and can be
 * hydrated cross-route. Persistence to SQLite is planned in Phase 1.5.
 */
export function useTerminalSettings() {
  /**
   * Default terminal appearance. Matches the values that were previously
   * hard-coded in `terminal.vue` so existing behavior is preserved.
   */
  const defaultAppearance: Appearance = {
    colorSchemeId: 'catppuccin-mocha',
    fontFamily: '"JetBrains Mono", "Fira Code", monospace',
    fontSize: 14,
    lineHeight: 1.2,
    cursorStyle: 'block',
    cursorBlink: true,
    fontWeight: 400,
    fontWeightBold: 700,
    scrollback: 1000,
  };

  const appearance = useLocalStorage('terminal-settings:appearance', () => defaultAppearance);

  // Merge defaults for any fields added after the user's last settings save.
  appearance.value = { ...defaultAppearance, ...appearance.value };
  const colorScheme = computed(() => getColorScheme(appearance.value.colorSchemeId));

  const colorSchemeId = computed({
    get: () => appearance.value.colorSchemeId,
    set: (value) => {
      appearance.value = { ...appearance.value, colorSchemeId: value };
    },
  });

  /** Minimum usable font size (px). */
  const fontSizeMin = 8;
  /** Maximum usable font size (px). */
  const fontSizeMax = 32;

  /**
   * Clamp a font-size value within `[fontSizeMin, fontSizeMax]`.
   * Rounds to the nearest integer. Falls back to the default font size
   * when the input is `NaN`.
   */
  function clampFontSize(size: number): number {
    if (Number.isNaN(size)) return defaultAppearance.fontSize;
    return Math.min(fontSizeMax, Math.max(fontSizeMin, Math.round(size)));
  }

  const fontSize = computed({
    get: () => appearance.value.fontSize,
    set: (value) => {
      appearance.value = { ...appearance.value, fontSize: clampFontSize(value) };
    },
  });

  /**
   * Font families offered in the settings picker. `value` is a complete CSS
   * `font-family` stack — the first installed font wins.
   */
  const fontFamilyOptions: readonly { label: string; value: string }[] = [
    { label: 'JetBrains Mono', value: '"JetBrains Mono", "Fira Code", monospace' },
    { label: 'Fira Code', value: '"Fira Code", "JetBrains Mono", monospace' },
    { label: 'Cascadia Code', value: '"Cascadia Code", "JetBrains Mono", monospace' },
    { label: 'Source Code Pro', value: '"Source Code Pro", monospace' },
    { label: 'Menlo', value: 'Menlo, Monaco, monospace' },
    { label: 'Monaco', value: 'Monaco, Menlo, monospace' },
    { label: 'Consolas', value: 'Consolas, "Cascadia Code", monospace' },
    { label: 'Courier New', value: '"Courier New", monospace' },
  ] as const;
  const fontFamily = computed({
    get: () => appearance.value.fontFamily,
    set: (value) => {
      appearance.value = { ...appearance.value, fontFamily: value };
    },
  });

  const fontWeight = computed(() => appearance.value.fontWeight);
  const fontWeightBold = computed(() => appearance.value.fontWeightBold);

  /** Minimum usable line-height multiplier. */
  const lineHeightMin = 1.0;
  /** Maximum usable line-height multiplier. */
  const lineHeightMax = 2.0;

  /**
   * Clamp a line-height value within `[lineHeightMin, lineHeightMax]`.
   * Falls back to the default line height when the input is `NaN`.
   */
  function clampLineHeight(height: number): number {
    if (Number.isNaN(height)) return defaultAppearance.lineHeight;
    return Math.min(lineHeightMax, Math.max(lineHeightMin, height));
  }
  const lineHeight = computed({
    get: () => appearance.value.lineHeight,
    set: (value) => {
      appearance.value = { ...appearance.value, lineHeight: clampLineHeight(value) };
    },
  });

  const cursorStyleOptions: readonly { label: string; value: CursorStyle }[] = [
    { label: 'Block', value: 'block' },
    { label: 'Underline', value: 'underline' },
    { label: 'Bar', value: 'bar' },
  ] as const;
  const cursorStyle = computed({
    get: () => appearance.value.cursorStyle,
    set: (value) => {
      appearance.value = { ...appearance.value, cursorStyle: value };
    },
  });

  const cursorBlink = computed({
    get: () => appearance.value.cursorBlink,
    set: (value) => {
      appearance.value = { ...appearance.value, cursorBlink: value };
    },
  });

  /** Minimum usable scrollback lines. */
  const scrollbackMin = 500;
  /** Maximum usable scrollback lines. */
  const scrollbackMax = 100000;

  function clampScrollback(n: number): number {
    if (Number.isNaN(n)) return defaultAppearance.scrollback;
    return Math.min(scrollbackMax, Math.max(scrollbackMin, Math.round(n)));
  }

  const scrollback = computed({
    get: () => appearance.value.scrollback,
    set: (value) => {
      appearance.value = { ...appearance.value, scrollback: clampScrollback(value) };
    },
  });

  return {
    defaultAppearance,
    appearance,
    colorScheme,
    colorSchemeId,
    fontSizeMin,
    fontSizeMax,
    fontSize,
    fontFamilyOptions,
    fontFamily,
    fontWeight,
    fontWeightBold,
    lineHeightMin,
    lineHeightMax,
    lineHeight,
    cursorStyleOptions,
    cursorStyle,
    cursorBlink,
    scrollbackMin,
    scrollbackMax,
    scrollback,
  };
}
