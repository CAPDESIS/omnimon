export interface Theme {
  id: string;
  name: string;
  colors: {
    // Backgrounds
    bgPrimary: string;
    bgSecondary: string;
    bgTertiary: string;
    bgCard: string;
    bgHover: string;

    // Text
    textPrimary: string;
    textSecondary: string;
    textMuted: string;

    // Accent
    accent: string;
    accentHover: string;
    accentMuted: string;

    // Semantic
    success: string;
    warning: string;
    danger: string;
    info: string;

    // Borders
    border: string;
    borderHover: string;

    // Charts
    chartLine: string;
    chartFill: string;
    chartGrid: string;
  };
}

export const themes: Record<string, Theme> = {
  dark: {
    id: 'dark',
    name: 'Dark',
    colors: {
      bgPrimary: '#0d1117',
      bgSecondary: '#161b22',
      bgTertiary: '#21262d',
      bgCard: '#1c2128',
      bgHover: '#292e36',
      textPrimary: '#e6edf3',
      textSecondary: '#8b949e',
      textMuted: '#484f58',
      accent: '#58a6ff',
      accentHover: '#79c0ff',
      accentMuted: '#1f3a5f',
      success: '#3fb950',
      warning: '#d29922',
      danger: '#f85149',
      info: '#58a6ff',
      border: '#30363d',
      borderHover: '#484f58',
      chartLine: '#58a6ff',
      chartFill: 'rgba(88,166,255,0.1)',
      chartGrid: '#21262d',
    },
  },
  light: {
    id: 'light',
    name: 'Light',
    colors: {
      bgPrimary: '#ffffff',
      bgSecondary: '#f6f8fa',
      bgTertiary: '#eaeef2',
      bgCard: '#ffffff',
      bgHover: '#f3f4f6',
      textPrimary: '#1f2328',
      textSecondary: '#656d76',
      textMuted: '#8b949e',
      accent: '#0969da',
      accentHover: '#0550ae',
      accentMuted: '#ddf4ff',
      success: '#1a7f37',
      warning: '#9a6700',
      danger: '#cf222e',
      info: '#0969da',
      border: '#d0d7de',
      borderHover: '#8b949e',
      chartLine: '#0969da',
      chartFill: 'rgba(9,105,218,0.1)',
      chartGrid: '#eaeef2',
    },
  },
  cyberpunk: {
    id: 'cyberpunk',
    name: 'Cyberpunk',
    colors: {
      bgPrimary: '#0a0a1a',
      bgSecondary: '#12122a',
      bgTertiary: '#1a1a3a',
      bgCard: '#15152b',
      bgHover: '#1f1f3f',
      textPrimary: '#e0e0ff',
      textSecondary: '#a0a0d0',
      textMuted: '#606090',
      accent: '#ff00ff',
      accentHover: '#ff44ff',
      accentMuted: '#3a0a3a',
      success: '#00ff88',
      warning: '#ffaa00',
      danger: '#ff3366',
      info: '#00ccff',
      border: '#2a2a4a',
      borderHover: '#4a4a6a',
      chartLine: '#00ccff',
      chartFill: 'rgba(0,204,255,0.1)',
      chartGrid: '#1a1a3a',
    },
  },
};

export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(theme.colors)) {
    const cssVar = '--' + key.replace(/([A-Z])/g, '-$1').toLowerCase();
    root.style.setProperty(cssVar, value);
  }
}

export function getTheme(id: string): Theme {
  return themes[id] ?? themes.dark;
}
