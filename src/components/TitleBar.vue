<script setup lang="ts">
import { inject } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { windowModeKey } from '@/composables/useWindowMode'

const win = (() => {
  try {
    return getCurrentWindow()
  } catch {
    return null
  }
})()
const wm = inject(windowModeKey)

async function minimize() {
  try {
    await win?.minimize()
  } catch (e) {
    console.error('[titlebar] minimize failed:', e)
  }
}

async function toggleMax() {
  try {
    if (!win) return
    const isMax = await win.isMaximized()
    if (isMax) await win.unmaximize()
    else await win.maximize()
  } catch (e) {
    console.error('[titlebar] toggle maximize failed:', e)
  }
}

async function close() {
  try {
    await win?.hide()
  } catch (e) {
    console.error('[titlebar] hide failed, falling back to close():', e)
    try {
      await win?.close()
    } catch (e2) {
      console.error('[titlebar] close also failed:', e2)
    }
  }
}
</script>

<template>
  <div class="titlebar drag-region">
    <div class="title">
      <img src="/loft-logo.png" alt="Loft" class="logo" draggable="false" />
      <span class="brand">凌台 · Loft</span>
      <span class="sub">桌面控制台</span>
    </div>
    <div class="actions no-drag">
      <button
        class="win-btn"
        :title="wm?.isMini.value ? '展开' : '折叠'"
        @click="wm?.toggleMode()"
      >
        <span :class="wm?.isMini.value ? 'i-carbon-fit-to-screen' : 'i-carbon-center-to-fit'" />
      </button>
      <button
        class="win-btn"
        :class="{ on: wm?.alwaysOnTop.value }"
        :title="wm?.alwaysOnTop.value ? '取消置顶' : '置顶'"
        @click="wm?.togglePin()"
      >
        <span :class="wm?.alwaysOnTop.value ? 'i-carbon-pin-filled' : 'i-carbon-pin'" />
      </button>
      <button class="win-btn" title="最小化" @click="minimize">
        <span class="i-carbon-subtract" />
      </button>
      <button v-if="!wm?.isMini.value" class="win-btn" title="最大化/还原" @click="toggleMax">
        <span class="i-carbon-maximize" />
      </button>
      <button class="win-btn danger" title="关闭" @click="close">
        <span class="i-carbon-close" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-left: 14px;
  background: var(--titlebar);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  min-width: 0;
}
.logo {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  object-fit: contain;
  display: block;
  user-select: none;
  -webkit-user-drag: none;
}
.brand {
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
}
.sub {
  color: var(--text-muted);
  font-size: 11.5px;
  white-space: nowrap;
}
.actions {
  display: flex;
  height: 100%;
}
.win-btn {
  width: 40px;
  height: 100%;
  display: grid;
  place-items: center;
  color: var(--text-muted);
  transition: background-color 0.15s, color 0.15s;
  font-size: 14px;
}
.win-btn:hover {
  background: var(--bg-elev);
  color: var(--text);
}
.win-btn.on {
  color: var(--accent);
}
.win-btn.danger:hover {
  background: var(--danger);
  color: #fff;
}

:global(html[data-ui-mode='mini'] .titlebar) {
  height: 32px;
  padding-left: 10px;
}
:global(html[data-ui-mode='mini'] .sub) {
  display: none;
}
:global(html[data-ui-mode='mini'] .win-btn) {
  width: 34px;
}
:global(html[data-ui-mode='mini'] .logo) {
  width: 18px;
  height: 18px;
}
</style>
