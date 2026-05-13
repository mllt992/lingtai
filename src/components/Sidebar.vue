<script setup lang="ts">
import { computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'

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
  <aside class="sidebar">
    <nav class="nav">
      <router-link
        v-for="item in items"
        :key="item.path"
        :to="item.path"
        class="nav-item"
        :class="{ active: isActive(item.path) }"
        :title="item.title"
      >
        <span :class="item.icon" class="icon" />
        <span class="label">{{ item.title }}</span>
      </router-link>
    </nav>
    <div class="footer">
      <div class="version">v0.1.0</div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 76px;
  background: var(--bg-soft);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  padding: 12px 0;
  flex-shrink: 0;
}
.nav {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 0 8px;
}
.nav-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 10px 4px;
  border-radius: var(--radius-md);
  color: var(--text-muted);
  font-size: 11px;
  text-decoration: none;
  transition: all 0.2s;
  position: relative;
}
.nav-item:hover {
  color: var(--text);
  background: var(--bg-elev);
}
.nav-item.active {
  color: var(--accent);
  background: var(--accent-soft);
}
.nav-item.active::before {
  content: '';
  position: absolute;
  left: -8px;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 22px;
  border-radius: 2px;
  background: var(--accent);
}
.icon {
  font-size: 20px;
}
.label {
  font-size: 10.5px;
  letter-spacing: 0.5px;
}
.footer {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 0;
}
.version {
  font-size: 10px;
  color: var(--text-muted);
  opacity: 0.6;
}
</style>
