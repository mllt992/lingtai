import { computed, onBeforeUnmount, type ComputedRef, type InjectionKey } from 'vue'
import {
  availableMonitors,
  currentMonitor,
  getCurrentWindow,
  LogicalPosition,
  LogicalSize,
  primaryMonitor,
  type Monitor
} from '@tauri-apps/api/window'
import { useSettingsStore } from '@/stores/settings'
import type { UiSettings, WindowMode, WindowRect } from '@/types'

export const MINI_SIZE = { w: 400, h: 560, minW: 360, minH: 440 }
export const EXPANDED_SIZE = { w: 980, h: 680, minW: 760, minH: 520 }
const MARGIN = 12
const PERSIST_MS = 400

export interface WindowModeApi {
  mode: ComputedRef<WindowMode>
  isMini: ComputedRef<boolean>
  alwaysOnTop: ComputedRef<boolean>
  init: () => Promise<void>
  toggleMode: () => Promise<void>
  togglePin: () => Promise<void>
}

export const windowModeKey: InjectionKey<WindowModeApi> = Symbol('windowMode')

interface LogicalRect {
  x: number
  y: number
  w: number
  h: number
}

function isFiniteNumber(n: unknown): n is number {
  return typeof n === 'number' && Number.isFinite(n)
}

function rectIsPlaced(rect: WindowRect): boolean {
  return isFiniteNumber(rect.x) && isFiniteNumber(rect.y) && isFiniteNumber(rect.w) && isFiniteNumber(rect.h)
}

function workAreaLogical(monitor: Monitor): LogicalRect {
  const factor = monitor.scaleFactor
  const pos = monitor.workArea.position.toLogical(factor)
  const size = monitor.workArea.size.toLogical(factor)
  return { x: pos.x, y: pos.y, w: size.width, h: size.height }
}

function intersects(a: LogicalRect, b: LogicalRect): boolean {
  return a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

function clampToRect(x: number, y: number, w: number, h: number, area: LogicalRect): { x: number; y: number } {
  const maxX = area.x + Math.max(0, area.w - w)
  const maxY = area.y + Math.max(0, area.h - h)
  return {
    x: Math.min(Math.max(x, area.x), maxX),
    y: Math.min(Math.max(y, area.y), maxY)
  }
}

function bottomRightIn(area: LogicalRect, w: number, h: number): { x: number; y: number } {
  return clampToRect(area.x + area.w - w - MARGIN, area.y + area.h - h - MARGIN, w, h, area)
}

export function useWindowMode(): WindowModeApi {
  const settings = useSettingsStore()
  const mode = computed(() => settings.ui.windowMode)
  const isMini = computed(() => mode.value === 'mini')
  const alwaysOnTop = computed(() => settings.ui.alwaysOnTop)

  let applying = false
  let persistTimer: ReturnType<typeof setTimeout> | null = null
  const unlisteners: Array<() => void> = []

  function setModeAttr(next: WindowMode) {
    document.documentElement.setAttribute('data-ui-mode', next)
  }

  function getWin() {
    try {
      if (!('__TAURI_INTERNALS__' in window)) return null
      const win = getCurrentWindow()
      if (win.label === 'hud') return null
      return win
    } catch {
      return null
    }
  }

  function preset(m: WindowMode) {
    return m === 'mini' ? MINI_SIZE : EXPANDED_SIZE
  }

  function normalizeRect(m: WindowMode, rect: WindowRect): WindowRect {
    const p = preset(m)
    return {
      x: rect.x,
      y: rect.y,
      w: Math.max(p.minW, rect.w || p.w),
      h: Math.max(p.minH, rect.h || p.h)
    }
  }

  async function pickWorkArea(preferX?: number, preferY?: number): Promise<LogicalRect | null> {
    try {
      const monitors = await availableMonitors()
      if (!monitors.length) {
        const fallback = await primaryMonitor()
        return fallback ? workAreaLogical(fallback) : null
      }
      const areas = monitors.map(workAreaLogical)
      if (isFiniteNumber(preferX) && isFiniteNumber(preferY)) {
        const hit = areas.find((a) => preferX >= a.x && preferX < a.x + a.w && preferY >= a.y && preferY < a.y + a.h)
        if (hit) return hit
      }
      const current = await currentMonitor()
      if (current) return workAreaLogical(current)
      return areas[0]
    } catch (e) {
      console.warn('[window-mode] monitor lookup failed:', e)
      return null
    }
  }

  async function resolvePosition(m: WindowMode, rect: WindowRect): Promise<{ x: number; y: number; w: number; h: number }> {
    const sized = normalizeRect(m, rect)
    const area = await pickWorkArea(sized.x ?? undefined, sized.y ?? undefined)
    if (!area) {
      return { x: sized.x ?? 80, y: sized.y ?? 80, w: sized.w, h: sized.h }
    }
    if (rectIsPlaced(sized)) {
      const probe: LogicalRect = { x: sized.x!, y: sized.y!, w: sized.w, h: sized.h }
      const onScreen = (await availableMonitors()).some((mon) => intersects(probe, workAreaLogical(mon)))
      if (onScreen) {
        const placed = clampToRect(sized.x!, sized.y!, sized.w, sized.h, area)
        return { ...placed, w: sized.w, h: sized.h }
      }
    }
    const placed = bottomRightIn(area, sized.w, sized.h)
    return { ...placed, w: sized.w, h: sized.h }
  }

  async function applyConstraints(m: WindowMode) {
    const win = getWin()
    if (!win) return
    const p = preset(m)
    await win.setMinSize(new LogicalSize(p.minW, p.minH))
    await win.setMaximizable(m === 'expanded')
  }

  async function applyGeometry(m: WindowMode, rect: WindowRect) {
    const win = getWin()
    if (!win) {
      setModeAttr(m)
      return
    }
    applying = true
    try {
      if (await win.isMaximized()) await win.unmaximize()
      const next = await resolvePosition(m, rect)
      await applyConstraints(m)
      await win.setSize(new LogicalSize(next.w, next.h))
      await win.setPosition(new LogicalPosition(next.x, next.y))
      await win.setAlwaysOnTop(settings.ui.alwaysOnTop)
      setModeAttr(m)
    } catch (e) {
      console.warn('[window-mode] applyGeometry failed:', e)
      setModeAttr(m)
    } finally {
      applying = false
    }
  }

  async function snapshot(m: WindowMode): Promise<WindowRect> {
    const win = getWin()
    const fallback = normalizeRect(m, settings.ui[m])
    if (!win) return fallback
    try {
      const factor = await win.scaleFactor()
      const pos = (await win.outerPosition()).toLogical(factor)
      const size = (await win.outerSize()).toLogical(factor)
      return {
        x: pos.x,
        y: pos.y,
        w: Math.max(preset(m).minW, Math.round(size.width)),
        h: Math.max(preset(m).minH, Math.round(size.height))
      }
    } catch (e) {
      console.warn('[window-mode] snapshot failed:', e)
      return fallback
    }
  }

  async function persistUi(patch: Partial<UiSettings>) {
    const next: UiSettings = {
      ...settings.ui,
      ...patch,
      mini: { ...settings.ui.mini, ...(patch.mini ?? {}) },
      expanded: { ...settings.ui.expanded, ...(patch.expanded ?? {}) }
    }
    settings.ui = next
    await settings.persist()
  }

  function schedulePersist() {
    if (applying) return
    if (persistTimer) clearTimeout(persistTimer)
    persistTimer = setTimeout(() => {
      persistTimer = null
      void (async () => {
        const m = mode.value
        const rect = await snapshot(m)
        await persistUi({ [m]: rect } as Partial<UiSettings>)
      })()
    }, PERSIST_MS)
  }

  async function toggleMode() {
    const win = getWin()
    const from = mode.value
    const to: WindowMode = from === 'mini' ? 'expanded' : 'mini'
    const current = await snapshot(from)
    const target = normalizeRect(to, settings.ui[to])
    const anchored = {
      x: current.x! + current.w - target.w,
      y: current.y! + current.h - target.h,
      w: target.w,
      h: target.h
    }
    applying = true
    try {
      if (win && (await win.isMaximized())) await win.unmaximize()
      const next = await resolvePosition(to, anchored)
      if (win) {
        await applyConstraints(to)
        await win.setSize(new LogicalSize(next.w, next.h))
        await win.setPosition(new LogicalPosition(next.x, next.y))
      }
      setModeAttr(to)
      await persistUi({ windowMode: to, [from]: current, [to]: next })
    } catch (e) {
      console.warn('[window-mode] toggleMode failed:', e)
    } finally {
      applying = false
    }
  }

  async function togglePin() {
    const win = getWin()
    const next = !settings.ui.alwaysOnTop
    try {
      if (win) await win.setAlwaysOnTop(next)
      await persistUi({ alwaysOnTop: next })
    } catch (e) {
      console.warn('[window-mode] togglePin failed:', e)
    }
  }

  async function init() {
    setModeAttr(mode.value)
    const win = getWin()
    try {
      await applyGeometry(mode.value, settings.ui[mode.value])
      const placed = await snapshot(mode.value)
      await persistUi({ [mode.value]: placed } as Partial<UiSettings>)
    } catch (e) {
      console.warn('[window-mode] init geometry failed:', e)
    }
    if (win) {
      try {
        const visible = await win.isVisible()
        if (!visible) {
          await win.show()
          await win.setFocus()
        }
      } catch (e) {
        console.warn('[window-mode] show failed:', e)
        try {
          await win.show()
        } catch {
          /* ignore */
        }
      }
      try {
        unlisteners.push(await win.onResized(() => schedulePersist()))
        unlisteners.push(await win.onMoved(() => schedulePersist()))
      } catch (e) {
        console.warn('[window-mode] listen resize/move failed:', e)
      }
    }
  }

  onBeforeUnmount(() => {
    if (persistTimer) clearTimeout(persistTimer)
    for (const off of unlisteners) off()
    unlisteners.length = 0
  })

  return { mode, isMini, alwaysOnTop, init, toggleMode, togglePin }
}
