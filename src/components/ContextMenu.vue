<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from 'vue'

export interface MenuItem {
  label: string
  icon?: string
  danger?: boolean
  disabled?: boolean
  divider?: boolean
  /** 子菜单（可选，简单嵌套） */
  submenu?: MenuItem[]
  onClick?: () => unknown | Promise<unknown>
}

const props = defineProps<{
  open: boolean
  x: number
  y: number
  items: MenuItem[]
}>()
const emit = defineEmits<{ close: [] }>()

const root = ref<HTMLElement | null>(null)
const submenuFor = ref<number | null>(null)

const style = computed(() => {
  // 自动反向避免溢出
  const W = 200
  const H = props.items.length * 32 + 12
  const vw = window.innerWidth
  const vh = window.innerHeight
  const x = Math.min(props.x, vw - W - 8)
  const y = Math.min(props.y, vh - H - 8)
  return { left: x + 'px', top: y + 'px' }
})

function handleOutside(e: MouseEvent) {
  if (!props.open) return
  if (root.value && !root.value.contains(e.target as Node)) emit('close')
}
function handleKey(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('close')
}

onMounted(() => {
  document.addEventListener('mousedown', handleOutside, true)
  document.addEventListener('keydown', handleKey)
})
onBeforeUnmount(() => {
  document.removeEventListener('mousedown', handleOutside, true)
  document.removeEventListener('keydown', handleKey)
})

async function pick(item: MenuItem, _idx: number) {
  if (item.disabled || item.divider) return
  if (item.submenu && item.submenu.length) return
  await item.onClick?.()
  emit('close')
}
</script>

<template>
  <teleport to="body">
    <transition name="ctx">
      <div v-if="open" ref="root" class="ctx-menu" :style="style" @contextmenu.prevent>
        <template v-for="(item, idx) in items" :key="idx">
          <div v-if="item.divider" class="divider" />
          <button
            v-else
            class="ctx-item"
            :class="{ danger: item.danger, disabled: item.disabled, 'has-sub': !!item.submenu }"
            :disabled="item.disabled"
            @mouseenter="submenuFor = item.submenu ? idx : null"
            @click="pick(item, idx)"
          >
            <span v-if="item.icon" :class="item.icon" class="ic" />
            <span class="lbl">{{ item.label }}</span>
            <span v-if="item.submenu" class="i-carbon-chevron-right arrow" />
            <!-- 子菜单 -->
            <div v-if="item.submenu && submenuFor === idx" class="ctx-menu submenu">
              <button
                v-for="(sub, i) in item.submenu"
                :key="i"
                class="ctx-item"
                :class="{ danger: sub.danger, disabled: sub.disabled }"
                :disabled="sub.disabled"
                @click.stop="pick(sub, i)"
              >
                <span v-if="sub.icon" :class="sub.icon" class="ic" />
                <span class="lbl">{{ sub.label }}</span>
              </button>
            </div>
          </button>
        </template>
      </div>
    </transition>
  </teleport>
</template>

<style scoped>
.ctx-menu {
  position: fixed;
  min-width: 180px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 6px;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.32), 0 2px 6px rgba(0, 0, 0, 0.18);
  z-index: 1000;
  backdrop-filter: blur(14px);
  user-select: none;
}
.ctx-item {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 10px;
  border-radius: 6px;
  font-size: 12.5px;
  color: var(--text);
  text-align: left;
  background: transparent;
  border: none;
  cursor: pointer;
}
.ctx-item:hover:not(.disabled) {
  background: var(--bg-elev);
}
.ctx-item.danger {
  color: var(--danger);
}
.ctx-item.danger:hover:not(.disabled) {
  background: rgba(248, 113, 113, 0.14);
}
.ctx-item.disabled {
  color: var(--text-muted);
  cursor: not-allowed;
  opacity: 0.6;
}
.ic {
  font-size: 14px;
  flex-shrink: 0;
}
.lbl {
  flex: 1;
}
.arrow {
  font-size: 12px;
  color: var(--text-muted);
}
.has-sub:hover {
  background: var(--bg-elev);
}
.divider {
  height: 1px;
  background: var(--border);
  margin: 4px 6px;
}
.submenu {
  position: absolute;
  left: 100%;
  top: -6px;
  margin-left: 2px;
}

.ctx-enter-active, .ctx-leave-active {
  transition: opacity 0.1s ease, transform 0.12s ease;
}
.ctx-enter-from, .ctx-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.98);
}
</style>
