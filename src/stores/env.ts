import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { EnvScope, EnvVarRow } from '@/types'

export const CRITICAL_ENV_NAMES = [
  'Path',
  'PATHEXT',
  'ComSpec',
  'OS',
  'windir',
  'SystemRoot',
  'TEMP',
  'TMP',
  'USERNAME',
  'USERPROFILE',
  'SystemDrive'
]

export function noteKey(scope: EnvScope, name: string) {
  return `${scope}:${name}`
}

export function isCriticalName(name: string) {
  return CRITICAL_ENV_NAMES.some((n) => n.toLowerCase() === name.toLowerCase())
}

export function isPathName(name: string) {
  return name.toLowerCase() === 'path'
}

interface NotesFile {
  version: number
  notes: Record<string, string>
}

export const useEnvStore = defineStore('env', {
  state: () => ({
    rows: [] as EnvVarRow[],
    notes: {} as Record<string, string>,
    loading: false,
    saving: false,
    error: null as string | null,
    keyword: '',
    scopes: ['user', 'machine'] as EnvScope[],
    copiedKey: '' as string
  }),
  getters: {
    filtered(state): EnvVarRow[] {
      const k = state.keyword.trim().toLowerCase()
      if (!k) return state.rows
      return state.rows.filter((row) => {
        if (row.name.toLowerCase().includes(k)) return true
        const note = state.notes[noteKey(row.scope, row.name)] ?? ''
        return note.toLowerCase().includes(k)
      })
    }
  },
  actions: {
    async refresh() {
      this.loading = true
      this.error = null
      try {
        this.rows = await invoke<EnvVarRow[]>('list_env_vars', {
          scopes: this.scopes
        })
      } catch (e: unknown) {
        this.error = String(e)
      } finally {
        this.loading = false
      }
    },
    async loadNotes() {
      try {
        const file = await invoke<NotesFile>('load_env_notes')
        this.notes = file?.notes ?? {}
      } catch (e: unknown) {
        this.error = String(e)
      }
    },
    async persistNotes() {
      const payload: NotesFile = { version: 1, notes: this.notes }
      await invoke('save_env_notes', { notes: payload })
    },
    async setNote(scope: EnvScope, name: string, text: string) {
      const key = noteKey(scope, name)
      const clipped = text.slice(0, 200)
      if (clipped) this.notes[key] = clipped
      else delete this.notes[key]
      try {
        await this.persistNotes()
      } catch (e: unknown) {
        this.error = String(e)
      }
    },
    toggleScope(scope: EnvScope) {
      if (this.scopes.includes(scope)) {
        if (this.scopes.length === 1) return
        this.scopes = this.scopes.filter((s) => s !== scope)
      } else {
        this.scopes = [...this.scopes, scope]
      }
      void this.refresh()
    },
    async getValue(name: string, scope: EnvScope) {
      return invoke<string>('get_env_value', { name, scope })
    },
    async upsert(name: string, scope: EnvScope, value: string) {
      this.saving = true
      this.error = null
      try {
        await invoke('upsert_env_var', { name, scope, value })
        await this.refresh()
      } catch (e: unknown) {
        this.error = String(e)
        throw e
      } finally {
        this.saving = false
      }
    },
    async remove(name: string, scope: EnvScope) {
      this.saving = true
      this.error = null
      try {
        await invoke('delete_env_var', { name, scope })
        const key = noteKey(scope, name)
        if (key in this.notes) {
          delete this.notes[key]
          await this.persistNotes()
        }
        await this.refresh()
      } catch (e: unknown) {
        this.error = String(e)
        throw e
      } finally {
        this.saving = false
      }
    }
  }
})
