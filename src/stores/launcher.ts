import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { LauncherItem, ResourceItem, ShortcutEntry } from '@/types'

interface PersistedItems {
  launcherItems: LauncherItem[]
  resources: ResourceItem[]
}

function uid() {
  return Math.random().toString(36).slice(2, 10) + Date.now().toString(36)
}

export const useLauncherStore = defineStore('launcher', {
  state: () => ({
    autoApps: [] as ShortcutEntry[],
    manualItems: [] as LauncherItem[],
    resources: [] as ResourceItem[],
    scanning: false,
    scanError: null as string | null,
    keyword: ''
  }),
  getters: {
    filteredApps(state): ShortcutEntry[] {
      const k = state.keyword.trim().toLowerCase()
      if (!k) return state.autoApps
      return state.autoApps.filter((a) => a.name.toLowerCase().includes(k))
    },
    filteredManual(state): LauncherItem[] {
      const k = state.keyword.trim().toLowerCase()
      if (!k) return state.manualItems
      return state.manualItems.filter((a) => a.name.toLowerCase().includes(k))
    },
    filteredResources(state): ResourceItem[] {
      const k = state.keyword.trim().toLowerCase()
      if (!k) return state.resources
      return state.resources.filter(
        (r) =>
          r.name.toLowerCase().includes(k) ||
          r.path.toLowerCase().includes(k)
      )
    },
    resourceGroups(): Record<string, ResourceItem[]> {
      const groups: Record<string, ResourceItem[]> = {}
      for (const r of this.filteredResources) {
        const key = r.kind === 'folder' ? '文件夹' : r.kind === 'file' ? '文件' : '网址'
        if (!groups[key]) groups[key] = []
        groups[key].push(r)
      }
      return groups
    }
  },
  actions: {
    async load() {
      const data = await invoke<PersistedItems>('load_items')
      this.manualItems = data.launcherItems ?? []
      this.resources = data.resources ?? []
    },
    async persist() {
      await invoke('save_items', {
        items: {
          launcherItems: this.manualItems,
          resources: this.resources
        }
      })
    },
    async scanApps() {
      this.scanning = true
      this.scanError = null
      try {
        this.autoApps = await invoke<ShortcutEntry[]>('scan_start_menu')
      } catch (e: any) {
        this.scanError = String(e)
      } finally {
        this.scanning = false
      }
    },
    async addLauncherItem(name: string, path: string) {
      // Try to resolve target for better launching
      let entry: ShortcutEntry | null = null
      try {
        entry = await invoke<ShortcutEntry>('resolve_shortcut', { path })
      } catch {
        // ignore
      }
      const item: LauncherItem = {
        id: uid(),
        name: name.trim() || entry?.name || path,
        path,
        target: entry?.target ?? null,
        iconPath: entry?.icon_path ?? null,
        source: 'manual',
        addedAt: Date.now(),
      }
      this.manualItems.unshift(item)
      await this.persist()
    },
    async removeLauncherItem(id: string) {
      this.manualItems = this.manualItems.filter((i) => i.id !== id)
      await this.persist()
    },
    async addResource(name: string, path: string, kind: ResourceItem['kind']) {
      const item: ResourceItem = {
        id: uid(),
        name: name.trim() || path,
        kind,
        path,
        addedAt: Date.now()
      }
      this.resources.unshift(item)
      await this.persist()
    },
    async removeResource(id: string) {
      this.resources = this.resources.filter((r) => r.id !== id)
      await this.persist()
    },
    async launchApp(entry: ShortcutEntry | LauncherItem) {
      await invoke('launch_path', { path: entry.path })
    },
    async openResource(item: ResourceItem) {
      if (item.kind === 'url') {
        await invoke('open_url', { url: item.path })
      } else {
        await invoke('open_path', { path: item.path })
      }
    }
  }
})
