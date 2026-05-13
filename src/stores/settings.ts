import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { AppSettings, ThemeId, ThemePreset } from '@/types'

export const THEME_PRESETS: ThemePreset[] = [
  {
    id: 'aurora',
    name: 'Aurora 极光',
    description: '深蓝渐变，默认深色',
    mode: 'dark',
    accent: '#5b8cff'
  },
  {
    id: 'carbon',
    name: 'Carbon 碳灰',
    description: 'OLED 友好纯黑',
    mode: 'dark',
    accent: '#ff6b35'
  },
  {
    id: 'nord',
    name: 'Nord 北欧',
    description: '冷调灰蓝',
    mode: 'dark',
    accent: '#88c0d0'
  },
  {
    id: 'vanilla',
    name: 'Vanilla 米白',
    description: '暖白护眼浅色',
    mode: 'light',
    accent: '#b07b3a'
  },
  {
    id: 'solar',
    name: 'Solar 暖橙',
    description: '阳光阅读色',
    mode: 'light',
    accent: '#cb4b16'
  },
  {
    id: 'mint',
    name: 'Mint 薄荷',
    description: '清新薄荷绿',
    mode: 'light',
    accent: '#16a085'
  }
]

const DEFAULTS: AppSettings = {
  version: 1,
  theme: 'aurora',
  accent: '#5b8cff',
  ports: { refreshMs: 5000, includeSystem: false },
  monitor: { refreshMs: 1000, historyLen: 60 },
  launcher: { autoScan: true, extraPaths: [] }
}

function applyTheme(theme: ThemeId, accent: string) {
  const root = document.documentElement
  root.setAttribute('data-theme', theme)
  // Override --accent if user has customized
  const preset = THEME_PRESETS.find((t) => t.id === theme)
  if (preset && accent !== preset.accent) {
    root.style.setProperty('--accent', accent)
  } else {
    root.style.removeProperty('--accent')
  }
}

export const useSettingsStore = defineStore('settings', {
  state: (): AppSettings & { loaded: boolean } => ({
    ...DEFAULTS,
    loaded: false
  }),
  getters: {
    currentPreset(state): ThemePreset {
      return (
        THEME_PRESETS.find((t) => t.id === state.theme) ?? THEME_PRESETS[0]
      )
    }
  },
  actions: {
    async load() {
      try {
        const cfg = await invoke<AppSettings>('load_settings')
        Object.assign(this, { ...DEFAULTS, ...cfg, loaded: true })
      } catch (e) {
        console.warn('[settings] load failed, using defaults:', e)
        this.loaded = true
      }
      applyTheme(this.theme, this.accent)
    },
    async setTheme(theme: ThemeId) {
      this.theme = theme
      const preset = THEME_PRESETS.find((t) => t.id === theme)
      if (preset) this.accent = preset.accent
      applyTheme(this.theme, this.accent)
      await this.persist()
    },
    async setAccent(color: string) {
      this.accent = color
      applyTheme(this.theme, this.accent)
      await this.persist()
    },
    async patch(patch: Partial<AppSettings>) {
      Object.assign(this, patch)
      applyTheme(this.theme, this.accent)
      await this.persist()
    },
    async persist() {
      const { loaded, ...settings } = this.$state as AppSettings & {
        loaded: boolean
      }
      try {
        await invoke('save_settings', { settings })
      } catch (e) {
        console.error('[settings] save failed:', e)
      }
    }
  }
})
