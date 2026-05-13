<script setup lang="ts">
import { onMounted } from 'vue'
import TitleBar from '@/components/TitleBar.vue'
import Sidebar from '@/components/Sidebar.vue'
import { useSettingsStore } from '@/stores/settings'

const settings = useSettingsStore()

onMounted(async () => {
  await settings.load()
})
</script>

<template>
  <div class="app-shell">
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
