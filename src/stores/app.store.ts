import { create } from 'zustand'

export interface Notification {
  id: string
  message: string
}

interface AppState {
  sidebarOpen: boolean
  notifications: Notification[]
  setSidebarOpen: (open: boolean) => void
  addNotification: (message: string) => void
  removeNotification: (id: string) => void
}

const createNotificationId = (): string => {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID()
  }

  return `${Date.now()}-${Math.random().toString(16).slice(2)}`
}

export const useAppStore = create<AppState>((set) => ({
  sidebarOpen: true,
  notifications: [],
  setSidebarOpen: (open) => {
    set({ sidebarOpen: open })
  },
  addNotification: (message) => {
    set((state) => ({
      notifications: [...state.notifications, { id: createNotificationId(), message }],
    }))
  },
  removeNotification: (id) => {
    set((state) => ({
      notifications: state.notifications.filter((notification) => notification.id !== id),
    }))
  },
}))
