import { invoke } from '@tauri-apps/api/core';

export type AppInfo = {
  name: string;
  version: string;
};

export const greet = (name: string): Promise<string> => invoke<string>('greet', { name });

export const getAppInfo = (): Promise<AppInfo> => invoke<AppInfo>('get_app_info');
