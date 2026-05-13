<script setup lang="ts">
defineProps<{
  title: string
  subtitle?: string
}>()
</script>

<template>
  <header class="page-header">
    <div class="left">
      <h1>{{ title }}</h1>
      <p v-if="subtitle">{{ subtitle }}</p>
    </div>
    <div class="right">
      <slot name="actions" />
    </div>
  </header>
</template>

<style scoped>
.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 18px 24px 8px;
  flex-shrink: 0;
}
h1 {
  font-size: 20px;
  font-weight: 600;
  margin: 0 0 2px;
  color: var(--text);
  letter-spacing: -0.3px;
}
p {
  margin: 0;
  font-size: 12.5px;
  color: var(--text-muted);
}
.right {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
