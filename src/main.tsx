import { RouterProvider } from '@tanstack/react-router';
import { StrictMode, useEffect } from 'react';
import { createRoot } from 'react-dom/client';

import { router } from './router';
import { useSettingsStore } from './stores/settings.store';
import './styles/globals.css';

function ThemeController(): null {
  const theme = useSettingsStore((state) => state.theme);

  useEffect(
    function syncThemeToHtmlClass() {
      const root = document.documentElement;
      const media = window.matchMedia('(prefers-color-scheme: dark)');

      const applyTheme = (): void => {
        const prefersDark = media.matches;
        const isDark = theme === 'dark' || (theme === 'system' && prefersDark);
        root.classList.toggle('dark', isDark);
      };

      applyTheme();

      media.addEventListener('change', applyTheme);
      return function cleanupThemeListener() {
        media.removeEventListener('change', applyTheme);
      };
    },
    [theme],
  );

  return null;
}

const container = document.getElementById('root');

if (!container) {
  throw new Error('Root container #root not found');
}

createRoot(container).render(
  <StrictMode>
    <ThemeController />
    <RouterProvider router={router} />
  </StrictMode>,
);
