<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    name: string
    iconData?: string | null
    size?: number
    rounded?: number
  }>(),
  { size: 40, rounded: 10 }
)

const initials = computed(() => {
  const n = (props.name || '?').trim()
  if (!n) return '?'
  // 中文：取第一个字符
  if (/[一-龥]/.test(n[0])) return n[0]
  return n.charAt(0).toUpperCase()
})

/** 给没图标的项目按名字 hash 一个稳定的色 */
const hashColor = computed(() => {
  let h = 0
  for (let i = 0; i < props.name.length; i++) h = (h * 31 + props.name.charCodeAt(i)) >>> 0
  const palette = ['#5b8cff', '#ff6b35', '#16a085', '#88c0d0', '#cb4b16', '#a855f7', '#ec4899', '#22c55e']
  return palette[h % palette.length]
})

const style = computed(() => ({
  width: props.size + 'px',
  height: props.size + 'px',
  borderRadius: props.rounded + 'px'
}))

const lblStyle = computed(() => ({
  background: hashColor.value + '22',
  color: hashColor.value,
  fontSize: Math.round(props.size * 0.42) + 'px'
}))
</script>

<template>
  <div class="app-icon" :style="style">
    <img v-if="iconData" :src="iconData" :alt="name" class="img" :style="style" />
    <div v-else class="placeholder" :style="{ ...style, ...lblStyle }">
      {{ initials }}
    </div>
  </div>
</template>

<style scoped>
.app-icon {
  display: grid;
  place-items: center;
  flex-shrink: 0;
  overflow: hidden;
}
.img {
  object-fit: contain;
  display: block;
}
.placeholder {
  display: grid;
  place-items: center;
  font-weight: 600;
  letter-spacing: -0.5px;
}
</style>
