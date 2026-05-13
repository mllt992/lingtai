import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type {
  LauncherGroup,
  LauncherItem,
  ResourceGroup,
  ResourceItem,
  ResourceKind,
  ShortcutEntry
} from '@/types'

interface PersistedShapeV3 {
  version: 3
  groups: LauncherGroup[]
  items: LauncherItem[]
  resourceGroups: ResourceGroup[]
  resources: ResourceItem[]
}

const DEFAULT_LAUNCHER_GID = 'default'
const DEFAULT_RES_GID = 'res-default'

function uid() {
  return Math.random().toString(36).slice(2, 10) + Date.now().toString(36)
}

/**
 * 把任意旧版本的 items.json 升级到 v3。
 * - v1: { launcherItems, resources }  —— resources 无 group
 * - v2: { version:2, groups, items, resources }  —— 启动器有 group，资源无
 * - v3: { version:3, ..., resourceGroups, resources(with groupId) }
 */
function migrate(raw: any): PersistedShapeV3 {
  if (raw && raw.version === 3 && Array.isArray(raw.resourceGroups)) {
    return raw as PersistedShapeV3
  }

  // ---------- launcher 部分 ----------
  let groups: LauncherGroup[] = []
  let items: LauncherItem[] = []
  if (raw?.version === 2 && Array.isArray(raw.groups)) {
    groups = raw.groups
    items = raw.items ?? []
  } else if (Array.isArray(raw?.launcherItems)) {
    // v1
    groups = [
      {
        id: DEFAULT_LAUNCHER_GID,
        name: raw.launcherItems.length ? '我的应用' : '快速启动',
        order: 0
      }
    ]
    items = raw.launcherItems.map((it: any, i: number) => ({
      id: it.id ?? uid(),
      groupId: DEFAULT_LAUNCHER_GID,
      name: it.name,
      path: it.path,
      target: it.target ?? null,
      iconPath: it.iconPath ?? null,
      iconData: it.iconData ?? null,
      order: i,
      addedAt: it.addedAt ?? Date.now()
    }))
  } else {
    groups = [{ id: DEFAULT_LAUNCHER_GID, name: '快速启动', order: 0 }]
  }

  // ---------- resources 部分 ----------
  const rawResources: any[] = raw?.resources ?? []
  let resourceGroups: ResourceGroup[] = []
  let resources: ResourceItem[] = []

  if (rawResources.length === 0) {
    resourceGroups = [{ id: DEFAULT_RES_GID, name: '我的收藏', order: 0 }]
  } else if (rawResources[0]?.groupId) {
    // 已是新形态（少数边界情况）—— 直接用，缺 groupId 的丢到默认组
    const existingGids = new Set(rawResources.map((r) => r.groupId).filter(Boolean))
    if (existingGids.size === 0) existingGids.add(DEFAULT_RES_GID)
    resourceGroups = raw?.resourceGroups ?? [...existingGids].map((g, i) => ({
      id: g as string,
      name: '收藏',
      order: i
    }))
    resources = rawResources.map((r: any, i: number) => ({
      id: r.id ?? uid(),
      groupId: r.groupId ?? DEFAULT_RES_GID,
      name: r.name,
      kind: r.kind,
      path: r.path,
      order: r.order ?? i,
      addedAt: r.addedAt ?? Date.now()
    }))
  } else {
    // v1/v2 资源：按 kind 自动建组，迁移到新结构
    const byKind: Record<string, any[]> = {}
    for (const r of rawResources) {
      const k = r.kind ?? 'file'
      if (!byKind[k]) byKind[k] = []
      byKind[k].push(r)
    }
    const kindMeta: Array<[ResourceKind, string]> = [
      ['folder', '文件夹'],
      ['file', '文件'],
      ['url', '网址']
    ]
    let gidx = 0
    for (const [k, name] of kindMeta) {
      const arr = byKind[k]
      if (!arr?.length) continue
      const gid = `res-${k}-` + uid().slice(0, 6)
      resourceGroups.push({ id: gid, name, order: gidx++ })
      arr.forEach((r, i) => {
        resources.push({
          id: r.id ?? uid(),
          groupId: gid,
          name: r.name,
          kind: r.kind,
          path: r.path,
          order: i,
          addedAt: r.addedAt ?? Date.now()
        })
      })
    }
    // 兜底：若所有 kind 都为空（不应发生）
    if (resourceGroups.length === 0) {
      resourceGroups = [{ id: DEFAULT_RES_GID, name: '我的收藏', order: 0 }]
    }
  }

  return { version: 3, groups, items, resourceGroups, resources }
}

export const useLauncherStore = defineStore('launcher', {
  state: () => ({
    autoApps: [] as ShortcutEntry[],
    groups: [] as LauncherGroup[],
    items: [] as LauncherItem[],
    resourceGroups: [] as ResourceGroup[],
    resources: [] as ResourceItem[],
    scanning: false,
    scanError: null as string | null,
    keyword: ''
  }),
  getters: {
    sortedGroups(state): LauncherGroup[] {
      return [...state.groups].sort((a, b) => a.order - b.order)
    },
    itemsByGroup(state): Record<string, LauncherItem[]> {
      const k = state.keyword.trim().toLowerCase()
      const map: Record<string, LauncherItem[]> = {}
      for (const g of state.groups) map[g.id] = []
      for (const it of state.items) {
        if (k && !it.name.toLowerCase().includes(k) && !it.path.toLowerCase().includes(k)) {
          continue
        }
        if (!map[it.groupId]) map[it.groupId] = []
        map[it.groupId].push(it)
      }
      for (const gid of Object.keys(map)) {
        map[gid].sort((a, b) => a.order - b.order)
      }
      return map
    },
    sortedResourceGroups(state): ResourceGroup[] {
      return [...state.resourceGroups].sort((a, b) => a.order - b.order)
    },
    resourcesByGroup(state): Record<string, ResourceItem[]> {
      const k = state.keyword.trim().toLowerCase()
      const map: Record<string, ResourceItem[]> = {}
      for (const g of state.resourceGroups) map[g.id] = []
      for (const r of state.resources) {
        if (k && !r.name.toLowerCase().includes(k) && !r.path.toLowerCase().includes(k)) {
          continue
        }
        if (!map[r.groupId]) map[r.groupId] = []
        map[r.groupId].push(r)
      }
      for (const gid of Object.keys(map)) {
        map[gid].sort((a, b) => a.order - b.order)
      }
      return map
    },
    filteredApps(state): ShortcutEntry[] {
      const k = state.keyword.trim().toLowerCase()
      if (!k) return state.autoApps
      return state.autoApps.filter((a) => a.name.toLowerCase().includes(k))
    },
    existingPaths(state): Set<string> {
      const s = new Set<string>()
      for (const it of state.items) s.add(it.path.toLowerCase())
      return s
    }
  },
  actions: {
    async load() {
      const data = await invoke<any>('load_items')
      const m = migrate(data)
      this.groups = m.groups
      this.items = m.items
      this.resourceGroups = m.resourceGroups
      this.resources = m.resources
      if (!data || data.version !== 3) await this.persist()
    },
    async persist() {
      const payload: PersistedShapeV3 = {
        version: 3,
        groups: this.groups,
        items: this.items,
        resourceGroups: this.resourceGroups,
        resources: this.resources
      }
      await invoke('save_items', { items: payload })
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

    // ============ Launcher 分组 ============
    async addGroup(name: string): Promise<LauncherGroup> {
      const trimmed = name.trim() || '新分组'
      const order = this.groups.length
        ? Math.max(...this.groups.map((g) => g.order)) + 1
        : 0
      const g: LauncherGroup = { id: uid(), name: trimmed, order }
      this.groups.push(g)
      await this.persist()
      return g
    },
    async renameGroup(id: string, name: string) {
      const g = this.groups.find((x) => x.id === id)
      if (!g) return
      g.name = name.trim() || g.name
      await this.persist()
    },
    async toggleGroupCollapsed(id: string) {
      const g = this.groups.find((x) => x.id === id)
      if (!g) return
      g.collapsed = !g.collapsed
      await this.persist()
    },
    async removeGroup(id: string, moveItemsTo?: string) {
      if (this.groups.length <= 1) throw new Error('至少保留一个分组')
      const affected = this.items.filter((i) => i.groupId === id)
      if (moveItemsTo) {
        const tail = this.items
          .filter((i) => i.groupId === moveItemsTo)
          .reduce((m, i) => Math.max(m, i.order), -1)
        affected.forEach((it, i) => {
          it.groupId = moveItemsTo
          it.order = tail + 1 + i
        })
      } else {
        this.items = this.items.filter((i) => i.groupId !== id)
      }
      this.groups = this.groups.filter((g) => g.id !== id)
      this.groups.forEach((g, i) => (g.order = i))
      await this.persist()
    },

    // ============ Launcher 条目 ============
    async addItem(
      groupId: string,
      input: { name: string; path: string; target?: string | null }
    ): Promise<LauncherItem> {
      const order = this._nextLauncherOrder(groupId)
      const item: LauncherItem = {
        id: uid(),
        groupId,
        name: input.name.trim() || input.path,
        path: input.path,
        target: input.target ?? null,
        iconPath: null,
        iconData: null,
        order,
        addedAt: Date.now()
      }
      this.items.push(item)
      await this.persist()
      this.refreshIcon(item.id).catch(() => {})
      return item
    },
    async addItemsFromShortcuts(groupId: string, entries: ShortcutEntry[]) {
      let order = this._nextLauncherOrder(groupId)
      const created: LauncherItem[] = []
      for (const e of entries) {
        const item: LauncherItem = {
          id: uid(),
          groupId,
          name: e.name,
          path: e.path,
          target: e.target ?? null,
          iconPath: e.icon_path ?? null,
          iconData: null,
          order: order++,
          addedAt: Date.now()
        }
        this.items.push(item)
        created.push(item)
      }
      await this.persist()
      void this._extractBatch(created.map((c) => c.id))
      return created
    },
    async renameItem(id: string, name: string) {
      const it = this.items.find((x) => x.id === id)
      if (!it) return
      it.name = name.trim() || it.name
      await this.persist()
    },
    async removeItem(id: string) {
      this.items = this.items.filter((x) => x.id !== id)
      await this.persist()
    },
    async moveItem(id: string, targetGroupId: string) {
      const it = this.items.find((x) => x.id === id)
      if (!it || it.groupId === targetGroupId) return
      it.groupId = targetGroupId
      it.order = this._nextLauncherOrder(targetGroupId)
      await this.persist()
    },
    async dropItem(sourceId: string, targetGroupId: string, targetId: string | null) {
      const src = this.items.find((x) => x.id === sourceId)
      if (!src) return
      const list = this.items
        .filter((x) => x.groupId === targetGroupId && x.id !== sourceId)
        .sort((a, b) => a.order - b.order)
      let insertAt = list.length
      if (targetId) {
        const idx = list.findIndex((x) => x.id === targetId)
        if (idx >= 0) insertAt = idx
      }
      src.groupId = targetGroupId
      list.splice(insertAt, 0, src)
      list.forEach((x, i) => (x.order = i))
      await this.persist()
    },
    async refreshIcon(id: string) {
      const it = this.items.find((x) => x.id === id)
      if (!it) return
      try {
        const data = await invoke<string | null>('extract_icon', { path: it.path })
        if (data) {
          it.iconData = data
          await this.persist()
        }
      } catch {
        // silent
      }
    },
    async _extractBatch(ids: string[]) {
      const queue = [...ids]
      const workers = Array.from({ length: 4 }, async () => {
        while (queue.length) {
          const id = queue.shift()
          if (id) await this.refreshIcon(id)
        }
      })
      await Promise.all(workers)
    },
    _nextLauncherOrder(groupId: string): number {
      return this.items
        .filter((i) => i.groupId === groupId)
        .reduce((m, i) => Math.max(m, i.order + 1), 0)
    },

    // ============ 资源 分组 ============
    async addResourceGroup(name: string): Promise<ResourceGroup> {
      const trimmed = name.trim() || '新分组'
      const order = this.resourceGroups.length
        ? Math.max(...this.resourceGroups.map((g) => g.order)) + 1
        : 0
      const g: ResourceGroup = { id: uid(), name: trimmed, order }
      this.resourceGroups.push(g)
      await this.persist()
      return g
    },
    async renameResourceGroup(id: string, name: string) {
      const g = this.resourceGroups.find((x) => x.id === id)
      if (!g) return
      g.name = name.trim() || g.name
      await this.persist()
    },
    async toggleResourceGroupCollapsed(id: string) {
      const g = this.resourceGroups.find((x) => x.id === id)
      if (!g) return
      g.collapsed = !g.collapsed
      await this.persist()
    },
    async removeResourceGroup(id: string, moveItemsTo?: string) {
      if (this.resourceGroups.length <= 1) throw new Error('至少保留一个分组')
      const affected = this.resources.filter((r) => r.groupId === id)
      if (moveItemsTo) {
        const tail = this.resources
          .filter((r) => r.groupId === moveItemsTo)
          .reduce((m, r) => Math.max(m, r.order), -1)
        affected.forEach((r, i) => {
          r.groupId = moveItemsTo
          r.order = tail + 1 + i
        })
      } else {
        this.resources = this.resources.filter((r) => r.groupId !== id)
      }
      this.resourceGroups = this.resourceGroups.filter((g) => g.id !== id)
      this.resourceGroups.forEach((g, i) => (g.order = i))
      await this.persist()
    },

    // ============ 资源 条目 ============
    async addResource(
      groupId: string,
      name: string,
      path: string,
      kind: ResourceKind
    ): Promise<ResourceItem> {
      const r: ResourceItem = {
        id: uid(),
        groupId,
        name: name.trim() || path,
        kind,
        path,
        order: this._nextResourceOrder(groupId),
        addedAt: Date.now()
      }
      this.resources.push(r)
      await this.persist()
      return r
    },
    async removeResource(id: string) {
      this.resources = this.resources.filter((r) => r.id !== id)
      await this.persist()
    },
    async renameResource(id: string, name: string) {
      const r = this.resources.find((x) => x.id === id)
      if (!r) return
      r.name = name.trim() || r.name
      await this.persist()
    },
    async moveResource(id: string, targetGroupId: string) {
      const r = this.resources.find((x) => x.id === id)
      if (!r || r.groupId === targetGroupId) return
      r.groupId = targetGroupId
      r.order = this._nextResourceOrder(targetGroupId)
      await this.persist()
    },
    async dropResource(sourceId: string, targetGroupId: string, targetId: string | null) {
      const src = this.resources.find((x) => x.id === sourceId)
      if (!src) return
      const list = this.resources
        .filter((x) => x.groupId === targetGroupId && x.id !== sourceId)
        .sort((a, b) => a.order - b.order)
      let insertAt = list.length
      if (targetId) {
        const idx = list.findIndex((x) => x.id === targetId)
        if (idx >= 0) insertAt = idx
      }
      src.groupId = targetGroupId
      list.splice(insertAt, 0, src)
      list.forEach((x, i) => (x.order = i))
      await this.persist()
    },
    _nextResourceOrder(groupId: string): number {
      return this.resources
        .filter((r) => r.groupId === groupId)
        .reduce((m, r) => Math.max(m, r.order + 1), 0)
    },

    // ============ 启动 / 打开 ============
    async launchItem(item: LauncherItem | ShortcutEntry) {
      await invoke('launch_path', { path: item.path })
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
