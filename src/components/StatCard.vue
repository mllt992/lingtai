<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  label: string
  value: string
  hint?: string
  percent?: number
  accent?: string
  icon?: string
}>()

const barWidth = computed(() =>
  props.percent != null ? `${Math.max(0, Math.min(100, props.percent))}%` : '0%'
)

const barColor = computed(() => props.accent || 'var(--accent)')
</script>

<template>
  <div class="stat-card">
    <div class="head">
      <span v-if="icon" :class="icon" class="ic" :style="{ color: barColor }" />
      <span class="lbl">{{ label }}</span>
    </div>
    <div class="value">{{ value }}</div>
    <div v-if="hint" class="hint">{{ hint }}</div>
    <div v-if="percent != null" class="bar">
      <div
        class="fill"
        :style="{ width: barWidth, background: barColor }"
      />
    </div>
  </div>
</template>

<style scoped>
.stat-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-height: 110px;
}
.head {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 12.5px;
}
.ic {
  font-size: 16px;
}
.lbl {
  letter-spacing: 0.4px;
}
.value {
  font-size: 24px;
  font-weight: 600;
  font-family: var(--font-mono);
  color: var(--text);
  letter-spacing: -0.5px;
}
.hint {
  font-size: 11.5px;
  color: var(--text-muted);
}
.bar {
  margin-top: auto;
  height: 6px;
  background: var(--bg-elev);
  border-radius: 3px;
  overflow: hidden;
}
.fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.4s ease;
}
</style>
