import { createFileRoute } from '@tanstack/react-router'

import { Button } from '../components/ui/button'
import { UpdaterCard } from '../components/UpdaterCard'
import { type Theme, useSettingsStore } from '../stores/settings.store'

const themes: Theme[] = ['light', 'dark', 'system']

function SettingsRoute() {
  const theme = useSettingsStore((state) => state.theme)
  const setTheme = useSettingsStore((state) => state.setTheme)

  return (
    <section className="max-w-2xl space-y-4">
      <h1 className="text-2xl font-bold">Settings</h1>
      <p className="text-sm opacity-80">Choose your preferred theme mode.</p>

      <div className="rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] p-4">
        <p className="mb-3 text-sm">
          Current theme: <span className="font-semibold">{theme}</span>
        </p>
        <div className="flex flex-wrap gap-2">
          {themes.map((themeOption) => (
            <Button
              key={themeOption}
              variant={theme === themeOption ? 'primary' : 'secondary'}
              onClick={() => {
                setTheme(themeOption)
              }}
            >
              {themeOption}
            </Button>
          ))}
        </div>
      </div>

      <UpdaterCard />
    </section>
  )
}

export const Route = createFileRoute('/settings')({
  component: SettingsRoute,
})

export const settingsRoute = Route
