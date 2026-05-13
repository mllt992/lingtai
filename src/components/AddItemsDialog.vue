<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useLauncherStore } from '@/stores/launcher'
import AppIcon from '@/components/AppIcon.vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import type { ShortcutEntry } from '@/types'

const props = defineProps<{
  open: boolean
  defaultGroupId: string
}>()
const emit = defineEmits<{ close: []; added: [count: number] }>()

const launcher = useLauncherStore()

type Tab = 'installed' | 'manual'
const tab = ref<Tab>('installed')
const search = ref('')
const selected = ref(new Set<string>())  // path
const targetGroupId = ref(props.defaultGroupId)
const submitting = ref(false)

// manual form
const manualName = ref('')
const manualPath = ref('')

watch(
  () => props.open,
  (v) => {
    if (v) {
      search.value = ''
      selected.value = new Set()
      manualName.value = ''
      manualPath.value = ''
      targetGroupId.value = props.defaultGroupId
      tab.value = 'installed'
      // 没扫过就扫
      if (launcher.autoApps.length === 0) launcher.scanApps()
    }
  }
)

const candidates = computed<ShortcutEntry[]>(() => {
  const k = search.value.trim().toLowerCase()
  const all = launcher.autoApps
  const existing = launcher.existingPaths
  return all.filter((a) => {
    if (existing.has(a.path.toLowerCase())) return false
    if (k && !a.name.toLowerCase().includes(k) && !(a.target ?? '').toLowerCase().includes(k))
      return false
    return true
  })
})

function toggle(path: string) {
  const s = new Set(selected.value)
  if (s.has(path)) s.delete(path)
  else s.add(path)
  selected.value = s
}

function selectAllVisible() {
  const s = new Set(selected.value)
  candidates.value.forEach((c) => s.add(c.path))
  selected.value = s
}

function clearSelection() {
  selected.value = new Set()
}

async function browseFile() {
  const picked = await openDialog({
    multiple: false,
    directory: false,
    filters: [
      { name: '可执行 / 快捷方式', extensions: ['exe', 'lnk', 'bat', 'cmd', 'url'] },
      { name: '所有文件', extensions: ['*'] }
    ]
  })
  if (typeof picked === 'string') {
    manualPath.value = picked
    if (!manualName.value)
      manualName.value = picked.split(/[\\\/]/).pop()?.replace(/\.(exe|lnk|url|bat|cmd)$/i, '') || ''
  }
}

async function confirm() {
  if (submitting.value) return
  submitting.value = true
  try {
    let added = 0
    if (tab.value === 'installed') {
      const picks = candidates.value.filter((c) => selected.value.has(c.path))
      if (picks.length) {
        await launcher.addItemsFromShortcuts(targetGroupId.value, picks)
        added = picks.length
      }
    } else {
      if (manualPath.value.trim()) {
        await launcher.addItem(targetGroupId.value, {
          name: manualName.value,
          path: manualPath.value
        })
        added = 1
      }
    }
    if (added > 0) emit('added', added)
    emit('close')
  } finally {
    submitting.value = false
  }
}

const canConfirm = computed(() => {
  if (tab.value === 'installed') return selected.value.size > 0
  return manualPath.value.trim().length > 0
})
</script>

<template>
  <transition name="dlg">
    <div v-if="open" class="mask" @click.self="emit('close')">
      <div class="dialog">
        <header>
          <h2>添加到启动器</h2>
          <button class="x" @click="emit('close')">
            <span class="i-carbon-close" />
          </button>
        </header>

        <div class="tabs">
          <button class="tab" :class="{ active: tab === 'installed' }" @click="tab = 'installed'">
            <span class="i-carbon-application" /> 从已装应用
            <em v-if="launcher.autoApps.length">{{ candidates.length }}</em>
          </button>
          <button class="tab" :class="{ active: tab === 'manual' }" @click="tab = 'manual'">
            <span class="i-carbon-add" /> 手动添加
          </button>
          <div class="group-picker">
            添加到
            <select v-model="targetGroupId">
              <option v-for="g in launcher.sortedGroups" :key="g.id" :value="g.id">
                {{ g.name }}
              </option>
            </select>
          </div>
        </div>

        <!-- Installed tab -->
        <div v-if="tab === 'installed'" class="body">
          <div class="toolbar">
            <div class="search">
              <span class="i-carbon-search" />
              <input v-model="search" placeholder="搜索已安装应用…" />
            </div>
            <button class="btn-ghost" @click="launcher.scanApps()" :disabled="launcher.scanning">
              <span class="i-carbon-renew" :class="{ spin: launcher.scanning }" /> 重新扫描
            </button>
            <button class="btn-ghost" @click="selectAllVisible">全选可见</button>
            <button class="btn-ghost" @click="clearSelection">清空</button>
            <span class="count">已选 {{ selected.size }}</span>
          </div>
          <div class="grid scrollbar-thin">
            <button
              v-for="c in candidates"
              :key="c.path"
              class="picker-card"
              :class="{ selected: selected.has(c.path) }"
              :title="c.target || c.path"
              @click="toggle(c.path)"
            >
              <AppIcon :name="c.name" :size="36" :rounded="8" />
              <div class="info">
                <div class="name">{{ c.name }}</div>
                <div class="path">{{ c.target || c.path }}</div>
              </div>
              <span class="check i-carbon-checkmark-filled" />
            </button>
            <div v-if="launcher.scanning && candidates.length === 0" class="empty">
              <span class="i-carbon-renew spin" /> 正在扫描…
            </div>
            <div v-else-if="candidates.length === 0" class="empty">
              <span class="i-carbon-information" />
              <p v-if="search">没有匹配的应用</p>
              <p v-else>所有已装应用都已添加到启动器</p>
            </div>
          </div>
        </div>

        <!-- Manual tab -->
        <div v-else class="body form">
          <label class="field">
            <span>名称（可选）</span>
            <input v-model="manualName" placeholder="比如 VSCode / Notion" class="input-base" />
          </label>
          <label class="field">
            <span>路径或 URL</span>
            <div class="row">
              <input
                v-model="manualPath"
                placeholder="C:\Program Files\…\app.exe  或  https://…"
                class="input-base flex-1"
              />
              <button class="btn-ghost" @click="browseFile">
                <span class="i-carbon-folder-open" /> 浏览
              </button>
            </div>
            <div class="hint">支持 .exe / .lnk / .bat / .cmd / .url 以及任何 Windows 能 "打开" 的路径</div>
          </label>
        </div>

        <footer>
          <button class="btn-ghost" @click="emit('close')">取消</button>
          <button class="btn-primary" :disabled="!canConfirm || submitting" @click="confirm">
            <span v-if="submitting" class="i-carbon-renew spin" />
            <span v-else class="i-carbon-add" />
            {{ tab === 'installed' ? `添加 ${selected.size} 项` : '添加' }}
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
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(6px);
  display: grid;
  place-items: center;
  z-index: 200;
}
.dialog {
  width: min(720px, 92vw);
  height: min(560px, 86vh);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 18px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 16px 64px rgba(0, 0, 0, 0.45);
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
}
header h2 {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
}
.x {
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border-radius: 6px;
  color: var(--text-muted);
}
.x:hover {
  background: var(--bg-elev);
  color: var(--text);
}

.tabs {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-soft);
}
.tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 8px;
  font-size: 12.5px;
  color: var(--text-muted);
  background: transparent;
}
.tab.active {
  background: var(--accent-soft);
  color: var(--accent);
}
.tab em {
  font-style: normal;
  font-size: 10.5px;
  background: var(--bg-elev);
  padding: 1px 6px;
  border-radius: 99px;
}
.group-picker {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 6px;
}
.group-picker select {
  height: 28px;
  padding: 0 10px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text);
  font-size: 12.5px;
  outline: none;
}

.body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.body.form {
  padding: 18px;
  gap: 14px;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
}
.search {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--bg-elev);
  border-radius: 8px;
  padding: 0 10px;
  height: 30px;
  color: var(--text-muted);
}
.search input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text);
  font-size: 12.5px;
}
.btn-ghost {
  height: 30px;
  padding: 0 10px;
  background: var(--bg-elev);
  border: 1px solid transparent;
  border-radius: 8px;
  color: var(--text-muted);
  font-size: 12px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.btn-ghost:hover {
  color: var(--text);
  border-color: var(--border);
}
.count {
  font-size: 11.5px;
  color: var(--accent);
  margin-left: 4px;
}

.grid {
  flex: 1;
  overflow-y: auto;
  padding: 10px 12px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 6px;
  align-content: start;
}
.picker-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 8px;
  cursor: pointer;
  text-align: left;
  position: relative;
  transition: background 0.12s;
}
.picker-card:hover {
  background: var(--bg-elev);
}
.picker-card.selected {
  background: var(--accent-soft);
  border-color: var(--accent);
}
.info {
  flex: 1;
  min-width: 0;
}
.name {
  font-size: 12.5px;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 500;
}
.path {
  font-size: 10.5px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: var(--font-mono);
}
.check {
  font-size: 16px;
  color: var(--accent);
  opacity: 0;
}
.picker-card.selected .check {
  opacity: 1;
}

.empty {
  grid-column: 1 / -1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 40px 20px;
  color: var(--text-muted);
  font-size: 12.5px;
}
.empty > span:first-child {
  font-size: 28px;
  opacity: 0.5;
}
.spin {
  animation: spin 1s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12.5px;
  color: var(--text-muted);
}
.field .row {
  display: flex;
  gap: 8px;
}
.flex-1 {
  flex: 1;
}
.field .hint {
  font-size: 11px;
  color: var(--text-muted);
  opacity: 0.7;
}

footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 18px;
  border-top: 1px solid var(--border);
}
footer .btn-primary {
  height: 32px;
  padding: 0 16px;
  background: var(--accent);
  color: #fff;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
footer .btn-primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
footer .btn-primary:hover:not(:disabled) {
  opacity: 0.9;
}
footer .btn-ghost {
  height: 32px;
  padding: 0 14px;
}

.dlg-enter-active, .dlg-leave-active {
  transition: opacity 0.18s ease;
}
.dlg-enter-active .dialog, .dlg-leave-active .dialog {
  transition: transform 0.2s ease, opacity 0.18s ease;
}
.dlg-enter-from, .dlg-leave-to {
  opacity: 0;
}
.dlg-enter-from .dialog, .dlg-leave-to .dialog {
  transform: scale(0.97) translateY(8px);
  opacity: 0;
}
</style>
