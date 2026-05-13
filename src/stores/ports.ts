import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { PortEntry } from '@/types'

export const usePortsStore = defineStore('ports', {
  state: () => ({
    entries: [] as PortEntry[],
    loading: false,
    error: null as string | null,
    includeSystem: false,
    intervalMs: 5000,
    keyword: '',
    _timer: null as ReturnType<typeof setInterval> | null
  }),
  getters: {
    filtered(state): PortEntry[] {
      const k = state.keyword.trim().toLowerCase()
      if (!k) return state.entries
      return state.entries.filter(
        (e) =>
          String(e.local_port).includes(k) ||
          (e.process_name?.toLowerCase().includes(k) ?? false) ||
          e.protocol.toLowerCase().includes(k)
      )
    }
  },
  actions: {
    async refresh() {
      this.loading = true
      this.error = null
      try {
        this.entries = await invoke<PortEntry[]>('list_user_ports', {
          includeSystem: this.includeSystem
        })
      } catch (e: any) {
        this.error = String(e)
      } finally {
        this.loading = false
      }
    },
    async kill(pid: number) {
      try {
        await invoke<boolean>('kill_process', { pid })
        await this.refresh()
      } catch (e: any) {
        this.error = String(e)
      }
    },
    start(intervalMs?: number) {
      if (intervalMs) this.intervalMs = intervalMs
      if (this._timer) return
      this.refresh()
      this._timer = setInterval(() => this.refresh(), this.intervalMs)
    },
    stop() {
      if (this._timer) {
        clearInterval(this._timer)
        this._timer = null
      }
    },
    restart(intervalMs: number) {
      this.stop()
      this.start(intervalMs)
    },
    setIncludeSystem(v: boolean) {
      this.includeSystem = v
      this.refresh()
    }
  }
})
