import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type {
  LauncherGroup,
  LauncherItem,
  ResourceItem,
  ShortcutEntry
} from '@/types'

interface PersistedShapeV2 {
  version: 2
  groups: LauncherGroup[]
  items: LauncherItem[]
  resources: ResourceItem[]
}

// 旧版（v1）形状：直接是 {launcherItems, resources}，没有 groups/order/groupId
interface PersistedShapeV1 {
  launcherItems?: Array<Partial<LauncherItem> & { name: string; path: string }>
  resources?: ResourceItem[]
}

const DEFAULT_GROUP_ID = 'default'

function uid() {
  return Math.random().toString(36).slice(2, 10) + Date.now().toString(36)
}

function migrate(raw: any): PersistedShapeV2 {
  if (raw && raw.version === 2 && Array.isArray(raw.groups)) {
    return raw as PersistedShapeV2
  }
  const old = (raw ?? {}) as PersistedShapeV1
  const items: LauncherItem[] = (old.launcherItems ?? []).map((it, i) => ({
    id: it.id ?? uid(),
    groupId: DEFAULT_GROUP_ID,
    name: it.name,
    path: it.path,
    target: (it as any).target ?? null,
    iconPath: (it as any).iconPath ?? null,
    iconData: (it as any).iconData ?? null,
    order: i,
    addedAt: it.addedAt ?? Date.now()
  }))
  return {
    version: 2,
    groups: [
      {
        id: DEFAULT_GROUP_ID,
        name: items.length ? '我的应用' : '快速启动',
        order: 0
      }
    ],
    items,
    resources: old.resources ?? []
  }
}

export const useLauncherStore = defineStore('launcher', {
  state: () => ({
    autoApps: [] as ShortcutEntry[],
    groups: [] as LauncherGroup[],
    items: [] as LauncherItem[],
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
    filteredApps(state): ShortcutEntry[] {
      const k = state.keyword.trim().toLowerCase()
      if (!k) return state.autoApps
      return state.autoApps.filter((a) => a.name.toLowerCase().includes(k))
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
    },
    /** 已添加路径集合，供"添加已装"对话框去重 */
    existingPaths(state): Set<string> {
      const s = new Set<string>()
      for (const it of state.items) s.add(it.path.toLowerCase())
      return s
    }
  },
  actions: {
    async load() {
      const data = await invoke<any>('load_items')
      const migrated = migrate(data)
      this.groups = migrated.groups
      this.items = migrated.items
      this.resources = migrated.resources ?? []
      // 若迁移产生了空的默认分组，立即写回
      if (!data || data.version !== 2) await this.persist()
    },
    async persist() {
      const payload: PersistedShapeV2 = {
        version: 2,
        groups: this.groups,
        items: this.items,
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
    // ============ 分组操作 ============
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
      if (this.groups.length <= 1) {
        throw new Error('至少保留一个分组')
      }
      const affected = this.items.filter((i) => i.groupId === id)
      if (moveItemsTo) {
        const targetMaxOrder = this.items
          .filter((i) => i.groupId === moveItemsTo)
          .reduce((m, i) => Math.max(m, i.order), -1)
        affected.forEach((it, i) => {
          it.groupId = moveItemsTo
          it.order = targetMaxOrder + 1 + i
        })
      } else {
        this.items = this.items.filter((i) => i.groupId !== id)
      }
      this.groups = this.groups.filter((g) => g.id !== id)
      this.groups.forEach((g, i) => (g.order = i))
      await this.persist()
    },
    async reorderGroups(orderedIds: string[]) {
      orderedIds.forEach((id, i) => {
        const g = this.groups.find((x) => x.id === id)
        if (g) g.order = i
      })
      await this.persist()
    },
    // ============ 条目操作 ============
    async addItem(
      groupId: string,
      input: { name: string; path: string; target?: string | null }
    ): Promise<LauncherItem> {
      const order = this._nextOrder(groupId)
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
      // 异步抽取图标，不阻塞插入
      this.refreshIcon(item.id).catch(() => {})
      return item
    },
    async addItemsFromShortcuts(groupId: string, entries: ShortcutEntry[]) {
      let order = this._nextOrder(groupId)
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
      // 并发抽取图标（限 4 并发）
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
      it.order = this._nextOrder(targetGroupId)
      await this.persist()
    },
    async reorderItems(groupId: string, orderedIds: string[]) {
      orderedIds.forEach((id, i) => {
        const it = this.items.find((x) => x.id === id)
        if (it && it.groupId === groupId) it.order = i
      })
      await this.persist()
    },
    /** 把 sourceId 插入到目标 group 的某条目之前；targetId 为空则追加末尾 */
    async dropItem(sourceId: string, targetGroupId: string, targetId: string | null) {
      const src = this.items.find((x) => x.id === sourceId)
      if (!src) return
      // 把目标 group 现有条目按 order 排序，决定插入位置
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
        // 静默：保留旧图标 / 占位
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
    _nextOrder(groupId: string): number {
      return this.items
        .filter((i) => i.groupId === groupId)
        .reduce((m, i) => Math.max(m, i.order + 1), 0)
    },
    // ============ 资源（保持不变） ============
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
