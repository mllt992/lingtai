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
  /** 相对「路径根目录」的子路径；不在任何根下时为 null */
  relPath?: string | null
  /** 计算 relPath 时使用的根，回退时优先 */
  rootHint?: string | null
  target?: string | null
  iconPath?: string | null
  iconData?: string | null   // data:image/png;base64,...  抽取后缓存于此
  order: number
  addedAt: number
}

export interface ResourceGroup {
  id: string
  name: string
  order: number
  collapsed?: boolean
}

export interface ResourceItem {
  id: string
  groupId: string
  name: string
  kind: ResourceKind
  path: string
  relPath?: string | null
  rootHint?: string | null
  order: number
  addedAt: number
}

export type PathHealthStatus = 'ok' | 'recoverable' | 'missing' | 'skipped'

export interface PathHealth {
  status: PathHealthStatus
  resolved?: string | null
}

export interface RelPathResult {
  rel_path: string | null
  root_hint: string | null
}

export type ResolveEffectivePathResult =
  | { status: 'ok'; path: string; used_relative: boolean }
  | { status: 'missing' }

export interface PathCheckItem {
  id: string
  kind: 'launcher' | 'file' | 'folder' | 'url'
  path: string
  rel_path?: string | null
  root_hint?: string | null
}

export interface PathCheckResult {
  id: string
  status: PathHealthStatus
  resolved?: string | null
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

export type WindowMode = 'mini' | 'expanded'

export interface WindowRect {
  x: number | null
  y: number | null
  w: number
  h: number
}

export interface UiSettings {
  windowMode: WindowMode
  alwaysOnTop: boolean
  mini: WindowRect
  expanded: WindowRect
}

export interface AppSettings {
  version: number
  theme: ThemeId
  accent: string
  ports: { refreshMs: number; includeSystem: boolean }
  monitor: { refreshMs: number; historyLen: number }
  launcher: {
    autoScan: boolean
    extraPaths: string[]
    /** 相对路径基准根目录列表 */
    pathRoots: string[]
    /** 相对可用、绝对失效时是否静默写回绝对路径 */
    autoRepairPaths: boolean
  }
  ui: UiSettings
}

export type EnvScope = 'user' | 'machine' | 'process'

export interface EnvVarRow {
  name: string
  scope: EnvScope
  kind: 'sz' | 'expand_sz' | 'process'
}
