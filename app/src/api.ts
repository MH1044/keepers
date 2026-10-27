// The only file that calls invoke: every command the window uses goes through here, so the
// boundary to the core stays typed and in one place.
import { invoke } from '@tauri-apps/api/core'

export interface AppInfo {
  version: string
}

export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>('app_info')
}
