<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

interface Metrics {
  cpu_pct: number
  mem_pct: number
  temp: number | null
  country: string | null
  ip: string | null
  city: string | null
  is_china: boolean
}

const m = ref<Metrics | null>(null)
let unlisten: UnlistenFn | null = null
const win = getCurrentWindow()

onMounted(async () => {
  unlisten = await listen<Metrics>('metrics', (event) => {
    m.value = event.payload
  })
})
onBeforeUnmount(() => {
  if (unlisten) unlisten()
})

function cpuColor(p: number) {
  if (p < 30) return '#34d399' // emerald-400
  if (p < 70) return '#facc15' // yellow-400
  return '#f87171' // red-400
}
function memColor(p: number) {
  if (p < 50) return '#34d399'
  if (p < 80) return '#facc15'
  return '#f87171'
}
function tempColor(t: number) {
  if (t < 50) return '#34d399'
  if (t < 75) return '#fb923c'
  return '#f87171'
}

const cpu = computed(() => m.value?.cpu_pct ?? 0)
const mem = computed(() => m.value?.mem_pct ?? 0)
const temp = computed(() => m.value?.temp ?? null)
const showVpn = computed(
  () => !!m.value && !m.value.is_china && !!m.value.country
)

// 整窗左键按下触发原生拖动；没拖动而仅是点击会自然不触发移动
async function onMouseDown(e: MouseEvent) {
  if (e.button !== 0) return
  e.preventDefault()
  try {
    await win.startDragging()
  } catch {
    /* ignore */
  }
}

async function onContextMenu(e: MouseEvent) {
  e.preventDefault()
  // 右键 → 隐藏 HUD（用户可以从主托盘菜单再开）
  await win.hide()
}
</script>

<template>
  <div class="hud" @mousedown="onMouseDown" @contextmenu="onContextMenu">
    <div class="chip">
      <span class="dot" :style="{ background: cpuColor(cpu) }" />
      <span class="label">CPU</span>
      <span class="value">{{ cpu.toFixed(0) }}<span class="unit">%</span></span>
    </div>
    <div class="sep" />
    <div class="chip">
      <span class="dot" :style="{ background: memColor(mem) }" />
      <span class="label">MEM</span>
      <span class="value">{{ mem.toFixed(0) }}<span class="unit">%</span></span>
    </div>
    <template v-if="temp != null">
      <div class="sep" />
      <div class="chip">
        <span class="dot" :style="{ background: tempColor(temp) }" />
        <span class="label">TEMP</span>
        <span class="value">{{ temp.toFixed(0) }}<span class="unit">°</span></span>
      </div>
    </template>
    <template v-if="showVpn">
      <div class="sep" />
      <div class="chip vpn">
        <span class="dot" :style="{ background: '#a78bfa' }" />
        <span class="label">VPN</span>
        <span class="value">{{ m!.country }}</span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.hud {
  height: 40px;
  padding: 0 14px;
  display: flex;
  align-items: center;
  gap: 6px;
  background: rgba(15, 23, 42, 0.82);
  backdrop-filter: blur(12px) saturate(140%);
  -webkit-backdrop-filter: blur(12px) saturate(140%);
  border-radius: 14px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow:
    0 8px 24px rgba(0, 0, 0, 0.28),
    inset 0 1px 0 rgba(255, 255, 255, 0.06);
  user-select: none;
  -webkit-user-select: none;
  cursor: grab;
  font-family: 'Segoe UI', 'PingFang SC', system-ui, sans-serif;
  color: #f1f5f9;
}
.hud:active {
  cursor: grabbing;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
}
.chip.vpn .label {
  color: #c4b5fd;
}
.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
  box-shadow: 0 0 8px currentColor;
  opacity: 0.95;
}
.label {
  font-size: 10.5px;
  font-weight: 600;
  color: #94a3b8;
  letter-spacing: 0.6px;
}
.value {
  font-size: 14px;
  font-weight: 700;
  color: #f8fafc;
  font-feature-settings: 'tnum' 1; /* 等宽数字 */
  letter-spacing: -0.3px;
}
.unit {
  font-size: 11px;
  font-weight: 500;
  color: #94a3b8;
  margin-left: 1px;
}
.sep {
  width: 1px;
  height: 14px;
  background: rgba(255, 255, 255, 0.1);
  margin: 0 2px;
}
</style>
