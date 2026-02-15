import { Link, Outlet, createRootRoute } from '@tanstack/react-router';

import { useAppStore } from '../stores/app.store';

function RootLayout() {
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const setSidebarOpen = useAppStore((state) => state.setSidebarOpen);

  return (
    <div className="min-h-screen bg-[var(--color-background)] text-[var(--color-foreground)]">
      <div className="mx-auto flex min-h-screen max-w-6xl">
        <aside
          className={`border-r border-[var(--color-border)] bg-[var(--color-panel)] transition-all ${
            sidebarOpen ? 'w-56' : 'w-16'
          }`}
        >
          <div className="flex items-center justify-between px-3 py-3">
            <span className="text-sm font-semibold">tauri-app</span>
            <button
              type="button"
              onClick={() => setSidebarOpen(!sidebarOpen)}
              className="rounded border border-[var(--color-border)] px-2 py-1 text-xs"
              aria-label="Toggle sidebar"
            >
              {sidebarOpen ? '−' : '+'}
            </button>
          </div>

          <nav className="flex flex-col gap-1 px-2">
            <Link
              to="/"
              className="rounded px-3 py-2 text-sm hover:bg-[var(--color-background)]"
              activeProps={{ className: 'bg-[var(--color-background)] font-semibold' }}
            >
              Home
            </Link>
            <Link
              to="/settings"
              className="rounded px-3 py-2 text-sm hover:bg-[var(--color-background)]"
              activeProps={{ className: 'bg-[var(--color-background)] font-semibold' }}
            >
              Settings
            </Link>
          </nav>
        </aside>

        <main className="flex-1 p-6">
          <Outlet />
        </main>
      </div>
    </div>
  );
}

export const Route = createRootRoute({
  component: RootLayout,
});

export const rootRoute = Route;
