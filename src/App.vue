<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import TitleBar from '@/components/TitleBar.vue'
import Sidebar from '@/components/Sidebar.vue'
import HudView from '@/views/HudView.vue'
import { useSettingsStore } from '@/stores/settings'
import { getCurrentWindow } from '@tauri-apps/api/window'

const settings = useSettingsStore()
const winLabel = ref<string>('main')

// 判定当前窗口是 main 还是 hud
// label === 'hud' 或 URL 含 hud=1 都视为 HUD
const isHud = computed(() => {
  if (winLabel.value === 'hud') return true
  if (typeof window !== 'undefined') {
    return window.location.search.includes('hud=1')
  }
  return false
})

onMounted(async () => {
  try {
    winLabel.value = getCurrentWindow().label
  } catch {
    winLabel.value = 'main'
  }
  // 只有主窗口加载用户设置（HUD 不需要）
  if (!isHud.value) {
    await settings.load()
  }
})
</script>

<template>
  <HudView v-if="isHud" />
  <div v-else class="app-shell">
    <TitleBar />
    <div class="app-body">
      <Sidebar />
      <main class="app-main">
        <router-view v-slot="{ Component, route }">
          <transition name="fade" mode="out-in">
            <component :is="Component" :key="route.path" />
          </transition>
        </router-view>
      </main>
    </div>
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
