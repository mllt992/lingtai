<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'

const props = defineProps<{
  open: boolean
  title: string
  label?: string
  placeholder?: string
  initial?: string
  confirmText?: string
  danger?: boolean
}>()
const emit = defineEmits<{ close: []; confirm: [value: string] }>()

const value = ref('')
const inputRef = ref<HTMLInputElement | null>(null)

watch(
  () => props.open,
  async (v) => {
    if (v) {
      value.value = props.initial ?? ''
      await nextTick()
      inputRef.value?.focus()
      inputRef.value?.select()
    }
  }
)

function submit() {
  emit('confirm', value.value.trim())
}
function onKey(e: KeyboardEvent) {
  if (e.key === 'Enter') submit()
  if (e.key === 'Escape') emit('close')
}
</script>

<template>
  <transition name="dlg">
    <div v-if="open" class="mask" @click.self="emit('close')">
      <div class="dialog">
        <header><h3>{{ title }}</h3></header>
        <div class="body">
          <label class="lbl" v-if="label">{{ label }}</label>
          <input
            ref="inputRef"
            v-model="value"
            :placeholder="placeholder"
            class="input-base"
            @keydown="onKey"
          />
        </div>
        <footer>
          <button class="btn-ghost" @click="emit('close')">取消</button>
          <button class="btn-primary" :class="{ danger }" @click="submit">
            {{ confirmText ?? '确定' }}
          </button>
        </footer>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(6px);
  display: grid;
  place-items: center;
  z-index: 300;
}
.dialog {
  width: 380px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
}
header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}
header h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}
.body {
  padding: 14px 16px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.lbl {
  font-size: 12px;
  color: var(--text-muted);
}
.input-base {
  height: 34px;
  padding: 0 10px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elev);
  color: var(--text);
  outline: none;
  font-size: 13px;
}
.input-base:focus {
  border-color: var(--accent);
}
footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 10px 16px;
  border-top: 1px solid var(--border);
}
.btn-primary {
  height: 30px;
  padding: 0 14px;
  background: var(--accent);
  color: #fff;
  border-radius: 7px;
  font-size: 12.5px;
  font-weight: 500;
}
.btn-primary.danger {
  background: var(--danger);
}
.btn-primary:hover {
  opacity: 0.9;
}
.btn-ghost {
  height: 30px;
  padding: 0 12px;
  background: var(--bg-elev);
  border-radius: 7px;
  color: var(--text-muted);
  font-size: 12.5px;
}
.btn-ghost:hover {
  color: var(--text);
}

.dlg-enter-active, .dlg-leave-active {
  transition: opacity 0.15s ease;
}
.dlg-enter-active .dialog, .dlg-leave-active .dialog {
  transition: transform 0.18s ease, opacity 0.15s ease;
}
.dlg-enter-from, .dlg-leave-to { opacity: 0; }
.dlg-enter-from .dialog, .dlg-leave-to .dialog {
  transform: scale(0.97) translateY(6px);
  opacity: 0;
}
</style>
