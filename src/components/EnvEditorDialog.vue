<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { isPathName } from '@/stores/env'
import type { EnvScope } from '@/types'

const props = defineProps<{
  open: boolean
  mode: 'create' | 'edit'
  name: string
  scope: EnvScope
  value: string
  note: string
  loading?: boolean
}>()

const emit = defineEmits<{
  close: []
  confirm: [payload: { name: string; scope: EnvScope; value: string; note: string }]
}>()

const name = ref('')
const scope = ref<EnvScope>('user')
const value = ref('')
const note = ref('')
const pathEntries = ref<string[]>([])
const nameError = ref('')

const pathMode = computed(() => isPathName(name.value.trim()))

watch(
  () => props.open,
  (open) => {
    if (!open) return
    name.value = props.name
    scope.value = props.scope
    value.value = props.value
    note.value = props.note
    pathEntries.value = splitPath(props.value)
    nameError.value = ''
  }
)

watch(
  () => props.value,
  (v) => {
    if (!props.open) return
    value.value = v
    pathEntries.value = splitPath(v)
  }
)

watch(pathMode, (on) => {
  if (on && pathEntries.value.length === 0) pathEntries.value = ['']
})

function splitPath(raw: string) {
  return raw
    .split(';')
    .map((s) => s.trim())
    .filter(Boolean)
}

function joinedPath() {
  return pathEntries.value.map((s) => s.trim()).filter(Boolean).join(';')
}

function addPathEntry() {
  pathEntries.value = [...pathEntries.value, '']
}

function removePathEntry(idx: number) {
  pathEntries.value = pathEntries.value.filter((_, i) => i !== idx)
}

function movePath(idx: number, dir: -1 | 1) {
  const next = idx + dir
  if (next < 0 || next >= pathEntries.value.length) return
  const copy = [...pathEntries.value]
  const tmp = copy[idx]
  copy[idx] = copy[next]
  copy[next] = tmp
  pathEntries.value = copy
}

function validateName(raw: string) {
  const n = raw.trim()
  if (!n) return '变量名不能为空'
  if (n.includes('=') || n.includes('\n')) return '变量名不能包含 = 或换行'
  if (n.length > 255) return '变量名过长'
  return ''
}

function submit() {
  const err = validateName(name.value)
  nameError.value = err
  if (err) return
  emit('confirm', {
    name: name.value.trim(),
    scope: scope.value,
    value: pathMode.value ? joinedPath() : value.value,
    note: note.value.slice(0, 200)
  })
}
</script>

<template>
  <transition name="dlg">
    <div v-if="open" class="mask" @click.self="emit('close')">
      <div class="dialog">
        <header>
          <h3>{{ mode === 'create' ? '新增环境变量' : '编辑环境变量' }}</h3>
        </header>
        <div class="body">
          <label class="lbl">名称</label>
          <input
            v-model="name"
            class="input-base"
            :disabled="mode === 'edit'"
            placeholder="例如 JAVA_HOME"
            @keydown.enter="!pathMode && submit()"
          />
          <p v-if="nameError" class="err">{{ nameError }}</p>

          <label class="lbl">范围</label>
          <div class="scopes">
            <label class="scope">
              <input v-model="scope" type="radio" value="user" :disabled="mode === 'edit'" />
              用户
            </label>
            <label class="scope">
              <input v-model="scope" type="radio" value="machine" :disabled="mode === 'edit'" />
              系统
            </label>
            <label class="scope">
              <input v-model="scope" type="radio" value="process" :disabled="mode === 'edit'" />
              当前进程
            </label>
          </div>
          <p v-if="scope === 'machine'" class="hint">
            写入系统级可能弹出 UAC。已打开的终端通常需要重开才会读到新值。
          </p>
          <p v-else-if="scope === 'process'" class="hint">
            只影响凌台自己，关闭后丢失，不会写入 Windows。
          </p>
          <p v-else class="hint">写入当前用户环境变量。已打开的终端通常需要重开。</p>

          <label class="lbl">{{ pathMode ? 'PATH 条目' : '值' }}</label>
          <div v-if="pathMode" class="path-list">
            <div v-for="(_, idx) in pathEntries" :key="idx" class="path-row">
              <input v-model="pathEntries[idx]" class="input-base grow" placeholder="目录路径" />
              <button class="icon-btn" title="上移" :disabled="idx === 0" @click="movePath(idx, -1)">
                <span class="i-carbon-arrow-up" />
              </button>
              <button
                class="icon-btn"
                title="下移"
                :disabled="idx === pathEntries.length - 1"
                @click="movePath(idx, 1)"
              >
                <span class="i-carbon-arrow-down" />
              </button>
              <button class="icon-btn danger" title="删除此条" @click="removePathEntry(idx)">
                <span class="i-carbon-close" />
              </button>
            </div>
            <button class="btn-ghost add" @click="addPathEntry">
              <span class="i-carbon-add" /> 添加条目
            </button>
          </div>
          <textarea
            v-else
            v-model="value"
            class="textarea"
            rows="4"
            placeholder="变量值，列表页不会显示"
          />

          <label class="lbl">备注 <span class="muted">{{ note.length }}/200</span></label>
          <input v-model="note" class="input-base" maxlength="200" placeholder="仅保存在凌台本地" />
        </div>
        <footer>
          <button class="btn-ghost" @click="emit('close')">取消</button>
          <button class="btn-primary" :disabled="loading" @click="submit">
            {{ loading ? '保存中…' : '保存' }}
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
  width: 560px;
  max-width: calc(100vw - 32px);
  max-height: calc(100vh - 48px);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
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
  gap: 8px;
  overflow: auto;
}
.lbl {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 4px;
}
.muted {
  font-weight: 400;
  opacity: 0.7;
}
.err {
  margin: 0;
  font-size: 12px;
  color: var(--danger);
}
.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-muted);
}
.scopes {
  display: flex;
  gap: 14px;
}
.scope {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  cursor: pointer;
}
.scope input {
  accent-color: var(--accent);
}
.textarea {
  min-height: 88px;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-elev);
  color: var(--text);
  outline: none;
  font-size: 13px;
  font-family: var(--font-mono);
  resize: vertical;
}
.textarea:focus,
.input-base:focus {
  border-color: var(--accent);
}
.path-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.path-row {
  display: flex;
  gap: 6px;
  align-items: center;
}
.grow {
  flex: 1;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: 12.5px;
}
.icon-btn {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border-radius: 6px;
  color: var(--text-muted);
  background: var(--bg-elev);
}
.icon-btn:hover:not(:disabled) {
  color: var(--text);
  border: 1px solid var(--accent);
}
.icon-btn.danger:hover:not(:disabled) {
  background: var(--danger);
  color: #fff;
}
.icon-btn:disabled {
  opacity: 0.35;
  cursor: default;
}
.add {
  align-self: flex-start;
  height: 30px;
  padding: 0 10px;
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
.btn-primary:disabled {
  opacity: 0.6;
  cursor: default;
}
.btn-ghost {
  height: 30px;
  padding: 0 12px;
  background: var(--bg-elev);
  border-radius: 7px;
  color: var(--text-muted);
  font-size: 12.5px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.btn-ghost:hover {
  color: var(--text);
}
.dlg-enter-active,
.dlg-leave-active {
  transition: opacity 0.15s ease;
}
.dlg-enter-active .dialog,
.dlg-leave-active .dialog {
  transition: transform 0.18s ease, opacity 0.15s ease;
}
.dlg-enter-from,
.dlg-leave-to {
  opacity: 0;
}
.dlg-enter-from .dialog,
.dlg-leave-to .dialog {
  transform: scale(0.97) translateY(6px);
  opacity: 0;
}
</style>
