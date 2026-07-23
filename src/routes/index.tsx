import { createFileRoute } from '@tanstack/react-router'
import { useCallback, useEffect, useState } from 'react'

import { Button } from '../components/ui/button'
import { getAppInfo, greet, type AppInfo } from '../lib/tauri'
import { useAppStore } from '../stores/app.store'

const unknownErrorToMessage = (error: unknown): string => {
  if (error instanceof Error) {
    return error.message
  }

  return String(error)
}

function HomeRoute() {
  const [name, setName] = useState('Tauri')
  const [greeting, setGreeting] = useState('')
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null)
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const addNotification = useAppStore((state) => state.addNotification)

  const runGreeting = useCallback(
    async (targetName: string): Promise<void> => {
      try {
        const message = await greet(targetName)
        setGreeting(message)
        setErrorMessage(null)
        addNotification(`Greeting updated for ${targetName}`)
      } catch (err: unknown) {
        setErrorMessage(unknownErrorToMessage(err))
      }
    },
    [addNotification],
  )

  useEffect(function loadInitialData() {
    let isCancelled = false

    const bootstrap = async (): Promise<void> => {
      try {
        const [info, initialGreeting] = await Promise.all([getAppInfo(), greet('Tauri')])

        if (isCancelled) {
          return
        }

        setAppInfo(info)
        setGreeting(initialGreeting)
        setErrorMessage(null)
      } catch (err: unknown) {
        if (!isCancelled) {
          setErrorMessage(unknownErrorToMessage(err))
        }
      }
    }

    void bootstrap()

    return function cleanupInitialDataLoad() {
      isCancelled = true
    }
  }, [])

  return (
    <section className="max-w-2xl space-y-4">
      <h1 className="text-2xl font-bold">Welcome</h1>
      <p className="text-sm opacity-80">Invoke Rust commands from your React route.</p>

      <div className="rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] p-4">
        <label htmlFor="name" className="mb-2 block text-sm font-medium">
          Name
        </label>
        <div className="flex flex-wrap items-center gap-2">
          <input
            id="name"
            value={name}
            onChange={(event) => {
              setName(event.target.value)
            }}
            className="w-full min-w-48 flex-1 rounded-md border border-[var(--color-border)] bg-[var(--color-background)] px-3 py-2 text-sm"
          />
          <Button onClick={() => void runGreeting(name)}>Run greet()</Button>
        </div>
      </div>

      <div className="space-y-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-panel)] p-4 text-sm">
        <p>
          <span className="font-semibold">Greeting:</span> {greeting || ' - '}
        </p>
        <p>
          <span className="font-semibold">App:</span>{' '}
          {appInfo ? `${appInfo.name} v${appInfo.version}` : 'Loading app info…'}
        </p>
        {errorMessage ? <p className="text-red-500">Error: {errorMessage}</p> : null}
      </div>
    </section>
  )
}

export const Route = createFileRoute('/')({
  component: HomeRoute,
})

export const indexRoute = Route
