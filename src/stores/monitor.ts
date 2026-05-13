import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { DriveInfo, SystemSnapshot } from '@/types'

interface SeriesPoint {
  t: number
  cpu: number
  mem: number
}

const HISTORY_LEN = 60

export const useMonitorStore = defineStore('monitor', {
  state: () => ({
    snapshot: null as SystemSnapshot | null,
    drives: [] as DriveInfo[],
    series: [] as SeriesPoint[],
    polling: false,
    intervalMs: 1000,
    error: null as string | null,
    _timer: null as ReturnType<typeof setInterval> | null
  }),
  getters: {
    cpuSeries(state): number[] {
      return state.series.map((p) => p.cpu)
    },
    memSeries(state): number[] {
      return state.series.map((p) => p.mem)
    }
  },
  actions: {
    async refreshOnce() {
      try {
        const snap = await invoke<SystemSnapshot>('get_system_snapshot')
        this.snapshot = snap
        const memPct = snap.mem.total
          ? (snap.mem.used / snap.mem.total) * 100
          : 0
        this.series.push({ t: Date.now(), cpu: snap.cpu.total, mem: memPct })
        if (this.series.length > HISTORY_LEN) {
          this.series.splice(0, this.series.length - HISTORY_LEN)
        }
        this.error = null
      } catch (e: any) {
        this.error = String(e)
      }
    },
    async refreshDrives() {
      try {
        this.drives = await invoke<DriveInfo[]>('list_drives')
      } catch (e: any) {
        this.error = String(e)
      }
    },
    start(intervalMs?: number) {
      if (intervalMs) this.intervalMs = intervalMs
      if (this._timer) return
      this.polling = true
      this.refreshOnce()
      this.refreshDrives()
      this._timer = setInterval(() => {
        this.refreshOnce()
      }, this.intervalMs)
    },
    stop() {
      if (this._timer) {
        clearInterval(this._timer)
        this._timer = null
      }
      this.polling = false
    },
    restart(intervalMs: number) {
      this.stop()
      this.start(intervalMs)
    }
  }
})
