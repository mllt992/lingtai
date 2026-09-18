<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

const router = useRouter()
const route = useRoute()

const items = computed(() =>
  router.options.routes
    .filter((r) => r.meta?.title)
    .map((r) => ({
      path: r.path,
      title: r.meta!.title as string,
      icon: r.meta!.icon as string
    }))
)

function isActive(path: string) {
  return route.path.startsWith(path)
}
</script>

<template>
  <nav class="mini-nav">
    <router-link
      v-for="item in items"
      :key="item.path"
      :to="item.path"
      class="tab"
      :class="{ active: isActive(item.path) }"
      :title="item.title"
    >
      <span :class="item.icon" class="icon" />
    </router-link>
  </nav>
</template>

<style scoped>
.mini-nav {
  display: flex;
  align-items: center;
  gap: 2px;
  height: 36px;
  padding: 0 8px;
  background: var(--bg-soft);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.tab {
  flex: 1;
  height: 28px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  color: var(--text-muted);
  text-decoration: none;
  transition: background 0.15s, color 0.15s;
}
.tab:hover {
  color: var(--text);
  background: var(--bg-elev);
}
.tab.active {
  color: var(--accent);
  background: var(--accent-soft);
}
.icon {
  font-size: 16px;
}
</style>
