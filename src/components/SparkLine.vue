<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    data: number[]
    max?: number
    color?: string
    fill?: boolean
    height?: number
  }>(),
  {
    max: 100,
    color: 'var(--accent)',
    fill: true,
    height: 60
  }
)

const VIEW_W = 200
const PADDING = 2

const pathData = computed(() => {
  const data = props.data
  if (!data.length) return { line: '', area: '' }
  const w = VIEW_W
  const h = props.height
  const step = data.length > 1 ? (w - PADDING * 2) / (data.length - 1) : 0
  const scale = (v: number) =>
    h - PADDING - (Math.max(0, Math.min(props.max, v)) / props.max) * (h - PADDING * 2)
  const pts = data.map((v, i) => [PADDING + i * step, scale(v)] as [number, number])
  const line = pts
    .map((p, i) => (i === 0 ? `M ${p[0]},${p[1]}` : `L ${p[0]},${p[1]}`))
    .join(' ')
  const lastX = pts[pts.length - 1][0]
  const firstX = pts[0][0]
  const area = `${line} L ${lastX},${h} L ${firstX},${h} Z`
  return { line, area }
})
</script>

<template>
  <svg
    class="spark"
    :viewBox="`0 0 ${VIEW_W} ${height}`"
    preserveAspectRatio="none"
    role="img"
    aria-label="时间序列图"
  >
    <defs>
      <linearGradient :id="`grad-${color.replace(/[^a-z0-9]/gi, '')}`" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" :stop-color="color" stop-opacity="0.35" />
        <stop offset="100%" :stop-color="color" stop-opacity="0" />
      </linearGradient>
    </defs>
    <path
      v-if="fill && pathData.area"
      :d="pathData.area"
      :fill="`url(#grad-${color.replace(/[^a-z0-9]/gi, '')})`"
      stroke="none"
    />
    <path
      v-if="pathData.line"
      :d="pathData.line"
      :stroke="color"
      stroke-width="1.5"
      fill="none"
      stroke-linecap="round"
      stroke-linejoin="round"
      vector-effect="non-scaling-stroke"
    />
  </svg>
</template>

<style scoped>
.spark {
  width: 100%;
  height: 100%;
  display: block;
}
</style>
