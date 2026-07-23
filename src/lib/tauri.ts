import { invoke } from '@tauri-apps/api/core'

export interface AppInfo {
  name: string
  version: string
}

export const greet = (name: string): Promise<string> => invoke<string>('greet', { name })

export const getAppInfo = (): Promise<AppInfo> => invoke<AppInfo>('get_app_info')
