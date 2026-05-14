import { useState, useEffect } from 'react';

export interface ThemeColors {
  bg: string;
  fg: string;
  accent: string;
  border: string;
}

const lightTheme: ThemeColors = {
  bg: '#ffffff',
  fg: '#1a1a1a',
  accent: '#3b82f6',
  border: '#e5e7eb',
};

const darkTheme: ThemeColors = {
  bg: '#1a1a1a',
  fg: '#f9fafb',
  accent: '#60a5fa',
  border: '#374151',
};

export function useTheme() {
  const [isDark, setIsDark] = useState(() => {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem('theme');
      if (stored) return stored === 'dark';
      return window.matchMedia('(prefers-color-scheme: dark)').matches;
    }
    return false;
  });

  useEffect(() => {
    const root = document.documentElement;
    if (isDark) {
      root.classList.add('dark');
      localStorage.setItem('theme', 'dark');
    } else {
      root.classList.remove('dark');
      localStorage.setItem('theme', 'light');
    }
  }, [isDark]);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (e: MediaQueryListEvent) => {
      if (!localStorage.getItem('theme')) {
        setIsDark(e.matches);
      }
    };
    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, []);

  const toggle = () => setIsDark(!isDark);

  const colors = isDark ? darkTheme : lightTheme;

  return { isDark, toggle, colors };
}

export function ThemeToggle() {
  const { isDark, toggle } = useTheme();

  return (
    <button
      onClick={toggle}
      className="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
      title={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
    >
      {isDark ? '☀️' : '🌙'}
    </button>
  );
}

export function applyThemeStyles(colors: ThemeColors) {
  return {
    '--bg-primary': colors.bg,
    '--fg-primary': colors.fg,
    '--accent': colors.accent,
    '--border': colors.border,
  } as React.CSSProperties;
}