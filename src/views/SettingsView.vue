<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useSettingsStore, THEME_PRESETS } from '@/stores/settings'
import { useMonitorStore } from '@/stores/monitor'
import { usePortsStore } from '@/stores/ports'
import { useLauncherStore } from '@/stores/launcher'
import PageHeader from '@/components/PageHeader.vue'

const settings = useSettingsStore()
const monitor = useMonitorStore()
const ports = usePortsStore()
const launcher = useLauncherStore()

const monitorMs = ref(settings.monitor.refreshMs)
const portsMs = ref(settings.ports.refreshMs)
const newRoot = ref('')
const backfillMsg = ref('')

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

async function toggleAutoRepair(v: boolean) {
  await settings.setAutoRepairPaths(v)
  if (v) void launcher.checkHealth()
}

async function pickRootDir() {
  const picked = await openDialog({
    directory: true,
    multiple: false,
    title: '选择路径根目录'
  })
  if (typeof picked === 'string' && picked.trim()) {
    newRoot.value = picked.trim()
  }
}

async function addPathRoot() {
  const v = newRoot.value.trim()
  if (!v) return
  const exists = settings.launcher.pathRoots.some(
    (r) => r.toLowerCase().replace(/\\/g, '/') === v.toLowerCase().replace(/\\/g, '/')
  )
  if (exists) {
    newRoot.value = ''
    return
  }
  await settings.setPathRoots([...settings.launcher.pathRoots, v])
  newRoot.value = ''
  void launcher.checkHealth()
}

async function removePathRoot(root: string) {
  await settings.setPathRoots(settings.launcher.pathRoots.filter((r) => r !== root))
  void launcher.checkHealth()
}

async function backfillRelPaths() {
  backfillMsg.value = '处理中…'
  try {
    if (!launcher.items.length && !launcher.resources.length) await launcher.load()
    const n = await launcher.backfillRelPaths()
    backfillMsg.value = n > 0 ? `已回填 ${n} 条` : '无需回填（无匹配根或已最新）'
  } catch (e) {
    backfillMsg.value = `回填失败：${String(e)}`
  }
  setTimeout(() => (backfillMsg.value = ''), 4000)
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

      <section class="block">
        <h3><span class="i-carbon-folder" /> 路径根目录</h3>
        <p class="desc">
          启动项/资源若位于根目录下，会额外记录相对路径。绝对路径失效时，用「根 + 相对路径」回退；
          <strong>只修复主路径</strong>，不改写已缓存图标。网址资源不参与。
        </p>
        <div class="roots">
          <div v-for="r in settings.launcher.pathRoots" :key="r" class="root-row">
            <code class="mono">{{ r }}</code>
            <button class="btn-ghost sm" title="移除" @click="removePathRoot(r)">
              <span class="i-carbon-trash-can" />
            </button>
          </div>
          <p v-if="!settings.launcher.pathRoots.length" class="desc">尚未配置根目录。可添加便携软件目录、同步盘目录等。</p>
        </div>
        <div class="control-row">
          <input
            v-model="newRoot"
            class="text-input"
            placeholder="例如 D:\Portable"
            @keydown.enter="addPathRoot"
          />
          <button class="btn-ghost" @click="pickRootDir">浏览…</button>
          <button class="btn-primary" :disabled="!newRoot.trim()" @click="addPathRoot">添加根目录</button>
        </div>
        <div class="control-row">
          <label>自动修复路径</label>
          <label class="switch">
            <input
              type="checkbox"
              :checked="settings.launcher.autoRepairPaths"
              @change="toggleAutoRepair(($event.target as HTMLInputElement).checked)"
            />
            <span class="track"><span class="thumb" /></span>
          </label>
          <span class="value">
            {{ settings.launcher.autoRepairPaths ? '开启：相对可用时静默写回' : '关闭：启动时询问是否修复' }}
          </span>
        </div>
        <div class="control-row">
          <button class="btn-ghost" @click="backfillRelPaths">回填相对路径</button>
          <span class="value">{{ backfillMsg }}</span>
        </div>
      </section>

      <section class="block about">
        <h3><span class="i-carbon-information" /> 关于</h3>
        <div class="grid-info">
          <div><span>产品</span><strong>凌台 · Loft</strong></div>
          <div><span>版本</span><strong>v0.1.0</strong></div>
          <div><span>引擎</span><strong>Tauri 2 + Vue 3 + Rust</strong></div>
          <div><span>主题</span><strong>{{ settings.currentPreset.name }}</strong></div>
          <div>
            <span>窗口</span>
            <strong>{{ settings.ui.windowMode === 'mini' ? '小窗' : '展开' }}{{ settings.ui.alwaysOnTop ? ' · 置顶' : '' }}</strong>
          </div>
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
.roots {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 4px;
}
.root-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 10px;
  background: var(--bg-elev);
  border: 1px solid var(--border);
  border-radius: 8px;
}
.root-row .mono {
  font-size: 12px;
  word-break: break-all;
}
.text-input {
  flex: 1;
  min-width: 0;
  height: 32px;
  padding: 0 10px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elev);
  color: var(--text);
  font-size: 12.5px;
  outline: none;
}
.text-input:focus {
  border-color: var(--accent);
}
.btn-ghost.sm {
  height: 28px;
  padding: 0 8px;
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

:global(html[data-ui-mode='mini'] .body) {
  padding: 8px 10px 16px;
  gap: 12px;
}
:global(html[data-ui-mode='mini'] .themes) {
  grid-template-columns: 1fr;
}
:global(html[data-ui-mode='mini'] .block) {
  padding: 12px 12px;
}
:global(html[data-ui-mode='mini'] .control-row) {
  flex-wrap: wrap;
}
:global(html[data-ui-mode='mini'] .control-row label) {
  min-width: 0;
}
:global(html[data-ui-mode='mini'] .range) {
  max-width: none;
  min-width: 120px;
}
</style>
