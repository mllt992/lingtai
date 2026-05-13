<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

const win = getCurrentWindow()

async function minimize() {
  await win.minimize()
}
async function toggleMax() {
  const isMax = await win.isMaximized()
  if (isMax) await win.unmaximize()
  else await win.maximize()
}
async function close() {
  await win.close()
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
      <button class="win-btn" title="最小化" @click="minimize">
        <span class="i-carbon-subtract" />
      </button>
      <button class="win-btn" title="最大化/还原" @click="toggleMax">
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
}
.sub {
  color: var(--text-muted);
  font-size: 11.5px;
}
.actions {
  display: flex;
  height: 100%;
}
.win-btn {
  width: 46px;
  height: 100%;
  display: grid;
  place-items: center;
  color: var(--text-muted);
  transition: background-color 0.15s;
  font-size: 14px;
}
.win-btn:hover {
  background: var(--bg-elev);
  color: var(--text);
}
.win-btn.danger:hover {
  background: var(--danger);
  color: #fff;
}
</style>
