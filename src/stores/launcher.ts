import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { useSettingsStore } from '@/stores/settings'
import type {
  LauncherGroup,
  LauncherItem,
  PathCheckItem,
  PathCheckResult,
  PathHealth,
  RelPathResult,
  ResolveEffectivePathResult,
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

export interface PendingPathRepair {
  targetKind: 'launcher' | 'resource'
  targetId: string
  name: string
  oldPath: string
  newPath: string
  /** 确认后是否继续启动/打开 */
  continueOpen: boolean
}

type RepairChoice = 'repair-open' | 'open-once' | 'cancel'

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
    items = (raw.items ?? []).map((it: any) => ({
      ...it,
      relPath: it.relPath ?? null,
      rootHint: it.rootHint ?? null
    }))
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
      relPath: it.relPath ?? null,
      rootHint: it.rootHint ?? null,
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
      relPath: r.relPath ?? null,
      rootHint: r.rootHint ?? null,
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
          relPath: r.relPath ?? null,
          rootHint: r.rootHint ?? null,
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
    keyword: '',
    pathHealth: {} as Record<string, PathHealth>,
    checkingPaths: false,
    pendingPathRepair: null as PendingPathRepair | null,
    pathRepairResolver: null as ((choice: RepairChoice) => void) | null
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
      if (!('__TAURI_INTERNALS__' in window)) {
        console.warn('[launcher] load skipped: no Tauri internals')
        return
      }
      // 父组件 settings.load 可能晚于本 store，先确保 pathRoots 就绪
      const settings = useSettingsStore()
      if (!settings.loaded) await settings.load()
      const data = await invoke<any>('load_items')
      const m = migrate(data)
      this.groups = m.groups
      this.items = m.items
      this.resourceGroups = m.resourceGroups
      this.resources = m.resources
      if (!data || data.version !== 3) await this.persist()
      void this.checkHealth()
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
    async _computeRel(path: string): Promise<{ relPath: string | null; rootHint: string | null }> {
      const roots = useSettingsStore().launcher.pathRoots ?? []
      if (!roots.length || !path) return { relPath: null, rootHint: null }
      try {
        const r = await invoke<RelPathResult>('compute_rel_path_cmd', { path, roots })
        return { relPath: r.rel_path ?? null, rootHint: r.root_hint ?? null }
      } catch {
        return { relPath: null, rootHint: null }
      }
    },
    async addItem(
      groupId: string,
      input: { name: string; path: string; target?: string | null }
    ): Promise<LauncherItem> {
      const order = this._nextLauncherOrder(groupId)
      const rel = await this._computeRel(input.path)
      const item: LauncherItem = {
        id: uid(),
        groupId,
        name: input.name.trim() || input.path,
        path: input.path,
        relPath: rel.relPath,
        rootHint: rel.rootHint,
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
        const rel = await this._computeRel(e.path)
        const item: LauncherItem = {
          id: uid(),
          groupId,
          name: e.name,
          path: e.path,
          relPath: rel.relPath,
          rootHint: rel.rootHint,
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
      const rel = kind === 'url' ? { relPath: null, rootHint: null } : await this._computeRel(path)
      const r: ResourceItem = {
        id: uid(),
        groupId,
        name: name.trim() || path,
        kind,
        path,
        relPath: rel.relPath,
        rootHint: rel.rootHint,
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
      const full = 'id' in item ? (item as LauncherItem) : null
      await this._openWithFallback({
        targetKind: 'launcher',
        targetId: full?.id ?? '',
        name: full?.name ?? item.name,
        path: item.path,
        relPath: full?.relPath ?? null,
        rootHint: full?.rootHint ?? null,
        open: (p) => invoke('launch_path', { path: p })
      })
    },
    async openResource(item: ResourceItem) {
      if (item.kind === 'url') {
        await invoke('open_url', { url: item.path })
        return
      }
      await this._openWithFallback({
        targetKind: 'resource',
        targetId: item.id,
        name: item.name,
        path: item.path,
        relPath: item.relPath ?? null,
        rootHint: item.rootHint ?? null,
        open: (p) => invoke('open_path', { path: p })
      })
    },
    async _openWithFallback(opts: {
      targetKind: 'launcher' | 'resource'
      targetId: string
      name: string
      path: string
      relPath: string | null
      rootHint: string | null
      open: (path: string) => Promise<unknown>
    }) {
      const settings = useSettingsStore()
      const roots = settings.launcher.pathRoots ?? []
      let resolved: ResolveEffectivePathResult
      try {
        resolved = await invoke<ResolveEffectivePathResult>('resolve_effective_path_cmd', {
          path: opts.path,
          relPath: opts.relPath,
          rootHint: opts.rootHint,
          roots
        })
      } catch (e) {
        throw new Error(String(e))
      }

      if (resolved.status === 'missing') {
        this.pathHealth[opts.targetId] = { status: 'missing', resolved: null }
        throw new Error('路径失效，且无法通过相对路径恢复')
      }

      if (!resolved.used_relative) {
        this.pathHealth[opts.targetId] = { status: 'ok', resolved: resolved.path }
        await opts.open(resolved.path)
        return
      }

      // 相对可用、绝对失效
      if (settings.launcher.autoRepairPaths) {
        await this.applyPathRepair(opts.targetKind, opts.targetId, resolved.path)
        await opts.open(resolved.path)
        return
      }

      if (!opts.targetId) {
        // 无持久化目标（扫描临时项）：仅本次启动
        await opts.open(resolved.path)
        return
      }

      const choice = await this._askPathRepair({
        targetKind: opts.targetKind,
        targetId: opts.targetId,
        name: opts.name,
        oldPath: opts.path,
        newPath: resolved.path,
        continueOpen: true
      })
      if (choice === 'cancel') return
      if (choice === 'repair-open') {
        await this.applyPathRepair(opts.targetKind, opts.targetId, resolved.path)
      }
      await opts.open(resolved.path)
    },
    _askPathRepair(req: PendingPathRepair): Promise<RepairChoice> {
      return new Promise((resolve) => {
        this.pathRepairResolver = resolve
        this.pendingPathRepair = req
      })
    },
    resolvePendingPathRepair(choice: RepairChoice) {
      const resolve = this.pathRepairResolver
      this.pathRepairResolver = null
      this.pendingPathRepair = null
      resolve?.(choice)
    },
    async applyPathRepair(
      targetKind: 'launcher' | 'resource',
      targetId: string,
      newPath: string
    ) {
      const rel = await this._computeRel(newPath)
      if (targetKind === 'launcher') {
        const it = this.items.find((x) => x.id === targetId)
        if (!it) return
        it.path = newPath
        it.relPath = rel.relPath
        it.rootHint = rel.rootHint
        // 仅重解析 .lnk 元数据；不改写 iconData
        if (/\.lnk$/i.test(newPath)) {
          try {
            const e = await invoke<ShortcutEntry>('resolve_shortcut', { path: newPath })
            it.target = e.target ?? it.target
            it.iconPath = e.icon_path ?? it.iconPath
          } catch {
            // silent
          }
        }
      } else {
        const r = this.resources.find((x) => x.id === targetId)
        if (!r) return
        r.path = newPath
        r.relPath = rel.relPath
        r.rootHint = rel.rootHint
      }
      this.pathHealth[targetId] = { status: 'ok', resolved: newPath }
      await this.persist()
    },
    async repairItemById(targetId: string) {
      const health = this.pathHealth[targetId]
      const resolved = health?.resolved
      if (!resolved) return
      const isLauncher = this.items.some((x) => x.id === targetId)
      await this.applyPathRepair(isLauncher ? 'launcher' : 'resource', targetId, resolved)
    },
    async checkHealth() {
      if (!('__TAURI_INTERNALS__' in window)) return
      const settings = useSettingsStore()
      const roots = settings.launcher.pathRoots ?? []
      const items: PathCheckItem[] = [
        ...this.items.map((it) => ({
          id: it.id,
          kind: 'launcher' as const,
          path: it.path,
          rel_path: it.relPath ?? null,
          root_hint: it.rootHint ?? null
        })),
        ...this.resources.map((r) => ({
          id: r.id,
          kind: r.kind,
          path: r.path,
          rel_path: r.relPath ?? null,
          root_hint: r.rootHint ?? null
        }))
      ]
      if (!items.length) {
        this.pathHealth = {}
        return
      }
      this.checkingPaths = true
      try {
        const chunkSize = 80
        const next: Record<string, PathHealth> = {}
        for (let i = 0; i < items.length; i += chunkSize) {
          const chunk = items.slice(i, i + chunkSize)
          const results = await invoke<PathCheckResult[]>('check_paths_cmd', { items: chunk, roots })
          for (const r of results) {
            next[r.id] = { status: r.status, resolved: r.resolved ?? null }
          }
        }
        this.pathHealth = next

        if (settings.launcher.autoRepairPaths) {
          for (const [id, h] of Object.entries(next)) {
            if (h.status === 'recoverable' && h.resolved) {
              const isLauncher = this.items.some((x) => x.id === id)
              await this.applyPathRepair(isLauncher ? 'launcher' : 'resource', id, h.resolved)
            }
          }
        }
      } catch (e) {
        console.warn('[launcher] checkHealth failed:', e)
      } finally {
        this.checkingPaths = false
      }
    },
    async backfillRelPaths(): Promise<number> {
      const roots = useSettingsStore().launcher.pathRoots ?? []
      if (!roots.length) return 0
      let changed = 0
      for (const it of this.items) {
        const rel = await this._computeRel(it.path)
        if (rel.relPath !== (it.relPath ?? null) || rel.rootHint !== (it.rootHint ?? null)) {
          it.relPath = rel.relPath
          it.rootHint = rel.rootHint
          changed++
        }
      }
      for (const r of this.resources) {
        if (r.kind === 'url') continue
        const rel = await this._computeRel(r.path)
        if (rel.relPath !== (r.relPath ?? null) || rel.rootHint !== (r.rootHint ?? null)) {
          r.relPath = rel.relPath
          r.rootHint = rel.rootHint
          changed++
        }
      }
      if (changed) await this.persist()
      return changed
    }
  }
})
