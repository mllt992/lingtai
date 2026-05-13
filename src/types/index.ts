export interface ShortcutEntry {
  name: string
  path: string
  target?: string | null
  icon_path?: string | null
  working_dir?: string | null
}

export type ResourceKind = 'folder' | 'file' | 'url'

export interface LauncherGroup {
  id: string
  name: string
  order: number
  collapsed?: boolean
}

export interface LauncherItem {
  id: string
  groupId: string
  name: string
  path: string
  target?: string | null
  iconPath?: string | null
  iconData?: string | null   // data:image/png;base64,...  抽取后缓存于此
  order: number
  addedAt: number
}

export interface ResourceItem {
  id: string
  name: string
  kind: ResourceKind
  path: string
  group?: string
  pinned?: boolean
  addedAt: number
}

export interface CpuSnapshot {
  total: number
  per_core: number[]
  brand: string
  frequency_mhz: number
  cores: number
}

export interface MemSnapshot {
  used: number
  total: number
  swap_used: number
  swap_total: number
}

export interface GpuInfo {
  name: string
  utilization: number | null
  mem_used: number | null
  mem_total: number | null
  vendor: string
}

export interface SystemSnapshot {
  cpu: CpuSnapshot
  mem: MemSnapshot
  gpus: GpuInfo[]
  uptime_secs: number
}

export interface DriveInfo {
  name: string
  mount: string
  total: number
  available: number
  file_system: string
  kind: string
}

export interface PortEntry {
  protocol: string
  local_addr: string
  local_port: number
  remote_addr: string | null
  remote_port: number | null
  state: string
  pid: number | null
  process_name: string | null
  process_path: string | null
  is_system: boolean
}

export type ThemeId =
  | 'aurora'
  | 'carbon'
  | 'vanilla'
  | 'nord'
  | 'solar'
  | 'mint'

export interface ThemePreset {
  id: ThemeId
  name: string
  description: string
  mode: 'dark' | 'light'
  accent: string
}

export interface AppSettings {
  version: number
  theme: ThemeId
  accent: string
  ports: { refreshMs: number; includeSystem: boolean }
  monitor: { refreshMs: number; historyLen: number }
  launcher: { autoScan: boolean; extraPaths: string[] }
}
