<script setup lang="ts">
import { computed, onMounted, provide } from 'vue'
import TitleBar from '@/components/TitleBar.vue'
import Sidebar from '@/components/Sidebar.vue'
import MiniNav from '@/components/MiniNav.vue'
import PathRepairDialog from '@/components/PathRepairDialog.vue'
import HudView from '@/views/HudView.vue'
import { useSettingsStore } from '@/stores/settings'
import { useWindowMode, windowModeKey } from '@/composables/useWindowMode'
import { getCurrentWindow } from '@tauri-apps/api/window'

const settings = useSettingsStore()
let detectedLabel = 'main'
try {
  detectedLabel = getCurrentWindow().label
} catch {
  // 浏览器 dev 环境，按主窗口处理
}
const isHud = computed(() => detectedLabel === 'hud')
const windowMode = useWindowMode()
const { mode, isMini, init } = windowMode
provide(windowModeKey, windowMode)

onMounted(async () => {
  if (isHud.value) return
  await settings.load()
  await init()
})
</script>

<template>
  <HudView v-if="isHud" />
  <div v-else class="app-shell" :data-ui-mode="mode">
    <TitleBar />
    <MiniNav v-if="isMini" />
    <div class="app-body">
      <Sidebar v-if="!isMini" />
      <main class="app-main">
        <router-view v-slot="{ Component, route }">
          <transition name="fade" mode="out-in">
            <component :is="Component" :key="route.path" />
          </transition>
        </router-view>
      </main>
    </div>
    <PathRepairDialog />
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg);
  color: var(--text);
}

.app-body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.app-main {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  position: relative;
  background:
    radial-gradient(circle at 0% 0%, var(--accent-soft) 0, transparent 35%),
    radial-gradient(circle at 100% 100%, var(--accent-soft) 0, transparent 40%),
    var(--bg);
}
</style>
