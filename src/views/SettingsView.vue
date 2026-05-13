<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useSettingsStore, THEME_PRESETS } from '@/stores/settings'
import { useMonitorStore } from '@/stores/monitor'
import { usePortsStore } from '@/stores/ports'
import PageHeader from '@/components/PageHeader.vue'

const settings = useSettingsStore()
const monitor = useMonitorStore()
const ports = usePortsStore()

const monitorMs = ref(settings.monitor.refreshMs)
const portsMs = ref(settings.ports.refreshMs)

watch(
  () => settings.loaded,
  (v) => {
    if (v) {
      monitorMs.value = settings.monitor.refreshMs
      portsMs.value = settings.ports.refreshMs
    }
  }
)

async function pickTheme(id: typeof THEME_PRESETS[number]['id']) {
  await settings.setTheme(id)
}

async function pickAccent(color: string) {
  await settings.setAccent(color)
}

async function applyMonitorInterval() {
  await settings.patch({
    monitor: { ...settings.monitor, refreshMs: monitorMs.value }
  })
  monitor.restart(monitorMs.value)
}

async function applyPortsInterval() {
  await settings.patch({
    ports: { ...settings.ports, refreshMs: portsMs.value }
  })
  ports.restart(portsMs.value)
}

async function toggleSystemPorts(v: boolean) {
  await settings.patch({
    ports: { ...settings.ports, includeSystem: v }
  })
  ports.setIncludeSystem(v)
}

const accentInput = ref(settings.accent)
watch(() => settings.accent, (v) => (accentInput.value = v))

async function onAccentInput(e: Event) {
  const v = (e.target as HTMLInputElement).value
  accentInput.value = v
  await pickAccent(v)
}

const swatchColors = ['#5b8cff', '#ff6b35', '#16a085', '#88c0d0', '#b07b3a', '#cb4b16', '#a855f7', '#ec4899']

const currentMode = computed(() => settings.currentPreset.mode)
</script>

<template>
  <div class="page">
    <PageHeader title="设置" subtitle="主题 · 监控间隔 · 端口策略 · 数据存放在 %APPDATA%\com.loft.app\" />

    <div class="body scrollbar-thin">
      <section class="block">
        <h3><span class="i-carbon-color-palette" /> 主题预设</h3>
        <p class="desc">六套预设，配色立即生效。当前模式：<strong>{{ currentMode === 'dark' ? '深色' : '浅色' }}</strong></p>
        <div class="themes">
          <button
            v-for="t in THEME_PRESETS"
            :key="t.id"
            class="theme-card"
            :class="{ active: settings.theme === t.id }"
            @click="pickTheme(t.id)"
          >
            <div class="swatch" :style="{ background: t.accent }">
              <div class="swatch-inner" :data-theme="t.id"></div>
            </div>
            <div class="theme-info">
              <div class="theme-name">{{ t.name }}</div>
              <div class="theme-desc">{{ t.description }}</div>
            </div>
            <span v-if="settings.theme === t.id" class="check i-carbon-checkmark-filled" />
          </button>
        </div>
      </section>

      <section class="block">
        <h3><span class="i-carbon-paint-brush" /> 自定义主色</h3>
        <p class="desc">覆盖当前主题的强调色。颜色选择器或快捷色板。</p>
        <div class="accent-row">
          <input
            type="color"
            :value="accentInput"
            @input="onAccentInput"
            class="color-input"
          />
          <div class="accent-hex">{{ accentInput }}</div>
          <div class="swatches">
            <button
              v-for="c in swatchColors"
              :key="c"
              class="dot-sw"
              :class="{ active: settings.accent.toLowerCase() === c.toLowerCase() }"
              :style="{ background: c }"
              :title="c"
              @click="pickAccent(c)"
            />
          </div>
          <button class="btn-ghost" @click="pickTheme(settings.theme)">还原默认</button>
        </div>
      </section>

      <section class="block">
        <h3><span class="i-carbon-chart-line" /> 性能监控</h3>
        <p class="desc">实时采样频率。频率越快，资源占用略增。</p>
        <div class="control-row">
          <label>刷新间隔（毫秒）</label>
          <input
            v-model.number="monitorMs"
            type="range"
            min="500"
            max="5000"
            step="100"
            class="range"
          />
          <span class="value mono">{{ monitorMs }} ms</span>
          <button class="btn-primary" @click="applyMonitorInterval">应用</button>
        </div>
      </section>

      <section class="block">
        <h3><span class="i-carbon-network-1" /> 端口监控</h3>
        <p class="desc">默认隐藏系统级进程，避免噪声。开启可观察全部 LISTEN 端口。</p>
        <div class="control-row">
          <label>刷新间隔（毫秒）</label>
          <input
            v-model.number="portsMs"
            type="range"
            min="2000"
            max="30000"
            step="1000"
            class="range"
          />
          <span class="value mono">{{ (portsMs / 1000).toFixed(0) }} s</span>
          <button class="btn-primary" @click="applyPortsInterval">应用</button>
        </div>
        <div class="control-row">
          <label>显示系统进程</label>
          <label class="switch">
            <input
              type="checkbox"
              :checked="settings.ports.includeSystem"
              @change="toggleSystemPorts(($event.target as HTMLInputElement).checked)"
            />
            <span class="track"><span class="thumb" /></span>
          </label>
          <span class="value">{{ settings.ports.includeSystem ? '已开启' : '已隐藏' }}</span>
        </div>
      </section>

      <section class="block about">
        <h3><span class="i-carbon-information" /> 关于</h3>
        <div class="grid-info">
          <div><span>产品</span><strong>凌台 · Loft</strong></div>
          <div><span>版本</span><strong>v0.1.0</strong></div>
          <div><span>引擎</span><strong>Tauri 2 + Vue 3 + Rust</strong></div>
          <div><span>主题</span><strong>{{ settings.currentPreset.name }}</strong></div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 24px 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.block {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 18px 20px;
}
.block h3 {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 4px;
  color: var(--text);
}
.desc {
  font-size: 12px;
  color: var(--text-muted);
  margin: 0 0 14px;
}
.desc strong {
  color: var(--text);
}

.themes {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 10px;
}
.theme-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: var(--bg-elev);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  text-align: left;
  position: relative;
  transition: all 0.18s;
}
.theme-card:hover {
  border-color: var(--accent);
}
.theme-card.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.swatch {
  width: 44px;
  height: 44px;
  border-radius: var(--radius-md);
  position: relative;
  overflow: hidden;
  flex-shrink: 0;
}
.swatch-inner {
  position: absolute;
  inset: 0;
  border-radius: inherit;
  border: 3px solid var(--bg);
}
.theme-info {
  flex: 1;
  min-width: 0;
}
.theme-name {
  font-weight: 500;
  font-size: 13px;
  color: var(--text);
}
.theme-desc {
  font-size: 11px;
  color: var(--text-muted);
}
.check {
  position: absolute;
  top: 8px;
  right: 8px;
  font-size: 16px;
  color: var(--accent);
}

.accent-row {
  display: flex;
  align-items: center;
  gap: 14px;
  flex-wrap: wrap;
}
.color-input {
  width: 44px;
  height: 44px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  cursor: pointer;
  background: transparent;
}
.accent-hex {
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text);
  min-width: 75px;
}
.swatches {
  display: flex;
  gap: 6px;
}
.dot-sw {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid var(--bg-elev);
  transition: transform 0.15s;
}
.dot-sw:hover {
  transform: scale(1.15);
}
.dot-sw.active {
  border-color: var(--text);
  transform: scale(1.15);
}

.control-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 10px;
}
.control-row label {
  font-size: 12.5px;
  color: var(--text-muted);
  min-width: 130px;
}
.range {
  flex: 1;
  max-width: 320px;
  accent-color: var(--accent);
}
.value {
  font-size: 12.5px;
  color: var(--text);
  min-width: 70px;
}
.value.mono {
  font-family: var(--font-mono);
}
.btn-primary {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 14px;
  background: var(--accent);
  color: #fff;
  border-radius: var(--radius-md);
  font-weight: 500;
  font-size: 12.5px;
}
.btn-primary:hover {
  opacity: 0.9;
}
.btn-ghost {
  height: 32px;
  padding: 0 12px;
  background: var(--bg-elev);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  color: var(--text-muted);
  font-size: 12.5px;
}
.btn-ghost:hover {
  color: var(--text);
  border-color: var(--accent);
}

.switch {
  position: relative;
  cursor: pointer;
}
.switch input {
  display: none;
}
.track {
  display: inline-block;
  width: 36px;
  height: 20px;
  border-radius: 12px;
  background: var(--bg-elev);
  border: 1px solid var(--border);
  position: relative;
  transition: background 0.18s;
}
.thumb {
  position: absolute;
  top: 1px;
  left: 1px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--text-muted);
  transition: all 0.18s;
}
.switch input:checked + .track {
  background: var(--accent);
  border-color: var(--accent);
}
.switch input:checked + .track .thumb {
  background: #fff;
  transform: translateX(16px);
}

.about .grid-info {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 10px;
}
.about .grid-info > div {
  display: flex;
  flex-direction: column;
  gap: 2px;
  background: var(--bg-elev);
  padding: 10px 12px;
  border-radius: var(--radius-md);
  font-size: 12.5px;
}
.about .grid-info span {
  color: var(--text-muted);
  font-size: 11px;
}
</style>
