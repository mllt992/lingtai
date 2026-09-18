<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import PageHeader from '@/components/PageHeader.vue'
import ContextMenu, { type MenuItem } from '@/components/ContextMenu.vue'
import EnvEditorDialog from '@/components/EnvEditorDialog.vue'
import { isCriticalName, isPathName, noteKey, useEnvStore } from '@/stores/env'
import type { EnvScope, EnvVarRow } from '@/types'

const env = useEnvStore()

const ctxOpen = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxItems = ref<MenuItem[]>([])

const editorOpen = ref(false)
const editorMode = ref<'create' | 'edit'>('create')
const editorName = ref('')
const editorScope = ref<EnvScope>('user')
const editorValue = ref('')
const editorNote = ref('')
const editorLoading = ref(false)

const confirmDelete = ref<EnvVarRow | null>(null)
const confirmEmptyPath = ref(false)
const confirmOverwrite = ref(false)
const pendingSave = ref<{
  name: string
  scope: EnvScope
  value: string
  note: string
} | null>(null)

const subtitle = computed(() => {
  const n = env.filtered.length
  return `列表不显示值 · 改用户/系统后已打开的终端通常要重开 · ${n} 项`
})

onMounted(async () => {
  await env.loadNotes()
  await env.refresh()
})

function scopeLabel(scope: EnvScope) {
  if (scope === 'user') return '用户'
  if (scope === 'machine') return '系统'
  return '进程'
}

function defaultCreateScope(): EnvScope {
  if (env.scopes.includes('user')) return 'user'
  if (env.scopes.includes('machine')) return 'machine'
  return 'process'
}

function openCreate() {
  editorMode.value = 'create'
  editorName.value = ''
  editorScope.value = defaultCreateScope()
  editorValue.value = ''
  editorNote.value = ''
  editorOpen.value = true
}

async function openEdit(row: EnvVarRow) {
  editorMode.value = 'edit'
  editorName.value = row.name
  editorScope.value = row.scope
  editorNote.value = env.notes[noteKey(row.scope, row.name)] ?? ''
  editorValue.value = ''
  editorOpen.value = true
  editorLoading.value = true
  try {
    editorValue.value = await env.getValue(row.name, row.scope)
  } catch (e: unknown) {
    env.error = String(e)
    editorOpen.value = false
  } finally {
    editorLoading.value = false
  }
}

async function copyValue(row: EnvVarRow) {
  try {
    const value = await env.getValue(row.name, row.scope)
    await navigator.clipboard.writeText(value)
    env.copiedKey = noteKey(row.scope, row.name)
    window.setTimeout(() => {
      if (env.copiedKey === noteKey(row.scope, row.name)) env.copiedKey = ''
    }, 1400)
  } catch (e: unknown) {
    env.error = String(e)
  }
}

async function copyName(row: EnvVarRow) {
  await navigator.clipboard.writeText(row.name)
}

function askDelete(row: EnvVarRow) {
  confirmDelete.value = row
}

async function doDelete() {
  const row = confirmDelete.value
  if (!row) return
  try {
    await env.remove(row.name, row.scope)
    confirmDelete.value = null
  } catch {
    /* error already on store */
  }
}

function existsSame(name: string, scope: EnvScope) {
  return env.rows.some(
    (row) => row.scope === scope && row.name.toLowerCase() === name.toLowerCase()
  )
}

async function onEditorConfirm(payload: {
  name: string
  scope: EnvScope
  value: string
  note: string
}) {
  if (editorMode.value === 'create' && existsSame(payload.name, payload.scope)) {
    pendingSave.value = payload
    confirmOverwrite.value = true
    return
  }
  if (isPathName(payload.name) && payload.value.trim() === '') {
    pendingSave.value = payload
    confirmEmptyPath.value = true
    return
  }
  await savePayload(payload)
}

async function continueAfterOverwrite(payload: {
  name: string
  scope: EnvScope
  value: string
  note: string
}) {
  confirmOverwrite.value = false
  if (isPathName(payload.name) && payload.value.trim() === '') {
    pendingSave.value = payload
    confirmEmptyPath.value = true
    return
  }
  await savePayload(payload)
}

async function savePayload(payload: {
  name: string
  scope: EnvScope
  value: string
  note: string
}) {
  editorLoading.value = true
  try {
    await env.upsert(payload.name, payload.scope, payload.value)
    await env.setNote(payload.scope, payload.name, payload.note)
    editorOpen.value = false
    confirmEmptyPath.value = false
    confirmOverwrite.value = false
    pendingSave.value = null
  } catch {
    /* store.error */
  } finally {
    editorLoading.value = false
  }
}

async function onNoteBlur(row: EnvVarRow, e: Event) {
  const text = (e.target as HTMLInputElement).value
  await env.setNote(row.scope, row.name, text)
}

function showRowMenu(e: MouseEvent, row: EnvVarRow) {
  e.preventDefault()
  ctxItems.value = [
    {
      label: '复制值',
      icon: 'i-carbon-copy',
      onClick: () => copyValue(row)
    },
    {
      label: '复制名称',
      icon: 'i-carbon-copy',
      onClick: () => copyName(row)
    },
    { divider: true, label: '' },
    {
      label: '编辑',
      icon: 'i-carbon-edit',
      onClick: () => openEdit(row)
    },
    {
      label: '删除',
      icon: 'i-carbon-trash-can',
      danger: true,
      onClick: () => askDelete(row)
    }
  ]
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxOpen.value = true
}

function showBlankMenu(e: MouseEvent) {
  e.preventDefault()
  ctxItems.value = [
    {
      label: env.loading ? '正在刷新…' : '立即刷新',
      icon: 'i-carbon-renew',
      disabled: env.loading,
      onClick: () => env.refresh()
    },
    {
      label: '新增变量',
      icon: 'i-carbon-add',
      onClick: () => openCreate()
    }
  ]
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxOpen.value = true
}
</script>

<template>
  <div class="page">
    <PageHeader title="环境变量" :subtitle="subtitle">
      <template #actions>
        <div class="search">
          <span class="i-carbon-search" />
          <input v-model="env.keyword" placeholder="搜索 名称 / 备注…" />
        </div>
        <div class="chips">
          <button
            class="chip"
            :class="{ on: env.scopes.includes('user') }"
            @click="env.toggleScope('user')"
          >
            用户
          </button>
          <button
            class="chip"
            :class="{ on: env.scopes.includes('machine') }"
            @click="env.toggleScope('machine')"
          >
            系统
          </button>
          <button
            class="chip"
            :class="{ on: env.scopes.includes('process') }"
            @click="env.toggleScope('process')"
          >
            进程
          </button>
        </div>
        <button class="btn-ghost" :disabled="env.loading" @click="env.refresh()">
          <span class="i-carbon-renew" :class="{ spin: env.loading }" />
          <span class="btn-text">刷新</span>
        </button>
        <button class="btn-primary" @click="openCreate">
          <span class="i-carbon-add" />
          <span class="btn-text">新增</span>
        </button>
      </template>
    </PageHeader>

    <div class="body scrollbar-thin" @contextmenu="showBlankMenu($event)">
      <div v-if="env.error" class="error">
        <span class="i-carbon-warning" /> {{ env.error }}
        <button class="retry" @click="env.refresh()">重试</button>
      </div>

      <div class="table">
        <div class="thead">
          <div class="th name">变量名称</div>
          <div class="th note">备注</div>
          <div class="th act">操作</div>
        </div>
        <div class="tbody">
          <div
            v-for="row in env.filtered"
            :key="`${row.scope}:${row.name}`"
            class="tr"
            @contextmenu.stop="showRowMenu($event, row)"
          >
            <div class="td name">
              <span class="var-name mono" :title="row.name">{{ row.name }}</span>
              <span class="badge" :class="row.scope">{{ scopeLabel(row.scope) }}</span>
            </div>
            <div class="td note">
              <input
                class="note-input"
                :value="env.notes[noteKey(row.scope, row.name)] ?? ''"
                maxlength="200"
                placeholder="点击填写备注"
                @blur="onNoteBlur(row, $event)"
                @keydown.enter="($event.target as HTMLInputElement).blur()"
              />
            </div>
            <div class="td act">
              <button
                class="icon-btn"
                :title="env.copiedKey === noteKey(row.scope, row.name) ? '已复制' : '复制值'"
                @click="copyValue(row)"
              >
                <span
                  :class="
                    env.copiedKey === noteKey(row.scope, row.name)
                      ? 'i-carbon-checkmark'
                      : 'i-carbon-copy'
                  "
                />
              </button>
              <button class="icon-btn" title="编辑" @click="openEdit(row)">
                <span class="i-carbon-edit" />
              </button>
              <button class="icon-btn danger" title="删除" @click="askDelete(row)">
                <span class="i-carbon-trash-can" />
              </button>
            </div>
          </div>
          <div v-if="env.filtered.length === 0" class="empty">
            <span class="i-carbon-data-set" />
            <p>{{ env.loading ? '正在读取环境变量…' : '当前过滤条件下没有环境变量' }}</p>
          </div>
        </div>
      </div>
    </div>

    <ContextMenu
      :open="ctxOpen"
      :x="ctxPos.x"
      :y="ctxPos.y"
      :items="ctxItems"
      @close="ctxOpen = false"
    />

    <EnvEditorDialog
      :open="editorOpen"
      :mode="editorMode"
      :name="editorName"
      :scope="editorScope"
      :value="editorValue"
      :note="editorNote"
      :loading="editorLoading || env.saving"
      @close="editorOpen = false"
      @confirm="onEditorConfirm"
    />

    <transition name="fade">
      <div v-if="confirmDelete" class="modal-mask" @click.self="confirmDelete = null">
        <div class="modal">
          <header>
            <h2>删除环境变量</h2>
          </header>
          <div class="modal-body">
            <p>
              将删除
              <strong>{{ scopeLabel(confirmDelete.scope) }}</strong>
              范围的
              <strong class="mono">{{ confirmDelete.name }}</strong>
              ，确认继续？
            </p>
            <p v-if="isCriticalName(confirmDelete.name)" class="warn">
              <span class="i-carbon-warning-alt" />
              这是系统关键变量，删除后新开的终端或程序可能找不到命令。
            </p>
            <p v-else-if="confirmDelete.scope === 'process'" class="warn">
              仅从凌台当前进程移除，不影响 Windows 持久变量。
            </p>
          </div>
          <footer>
            <button class="btn-ghost" @click="confirmDelete = null">取消</button>
            <button class="btn-danger" :disabled="env.saving" @click="doDelete">
              <span class="i-carbon-trash-can" /> 确认删除
            </button>
          </footer>
        </div>
      </div>
    </transition>

    <transition name="fade">
      <div v-if="confirmOverwrite" class="modal-mask" @click.self="confirmOverwrite = false">
        <div class="modal">
          <header>
            <h2>覆盖已有变量</h2>
          </header>
          <div class="modal-body">
            <p>
              <strong class="mono">{{ pendingSave?.name }}</strong>
              已存在于{{ pendingSave ? scopeLabel(pendingSave.scope) : '' }}范围，保存将覆盖原值。
            </p>
          </div>
          <footer>
            <button class="btn-ghost" @click="confirmOverwrite = false">取消</button>
            <button class="btn-danger" :disabled="!pendingSave" @click="pendingSave && continueAfterOverwrite(pendingSave)">
              确认覆盖
            </button>
          </footer>
        </div>
      </div>
    </transition>

    <transition name="fade">
      <div v-if="confirmEmptyPath" class="modal-mask" @click.self="confirmEmptyPath = false">
        <div class="modal">
          <header>
            <h2>将 Path 写成空</h2>
          </header>
          <div class="modal-body">
            <p class="warn">
              <span class="i-carbon-warning-alt" />
              Path 被清空后，新开的终端可能找不到系统命令。确定继续保存？
            </p>
          </div>
          <footer>
            <button class="btn-ghost" @click="confirmEmptyPath = false">取消</button>
            <button
              class="btn-danger"
              :disabled="!pendingSave"
              @click="pendingSave && savePayload(pendingSave)"
            >
              确认保存空 Path
            </button>
          </footer>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.search {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 0 10px;
  height: 36px;
  color: var(--text-muted);
  min-width: 200px;
}
.search input {
  background: transparent;
  border: none;
  outline: none;
  color: var(--text);
  flex: 1;
}
.chips {
  display: flex;
  gap: 6px;
}
.chip {
  height: 36px;
  padding: 0 10px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-muted);
  font-size: 12.5px;
}
.chip.on {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-soft);
}
.btn-ghost,
.btn-primary {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 36px;
  padding: 0 12px;
  border-radius: var(--radius-md);
  font-size: 13px;
}
.btn-ghost {
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-muted);
}
.btn-ghost:hover {
  color: var(--text);
  border-color: var(--accent);
}
.btn-primary {
  background: var(--accent);
  color: #fff;
}
.body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 24px 24px;
}
.error {
  padding: 10px 14px;
  background: rgba(248, 113, 113, 0.12);
  color: var(--danger);
  border-radius: var(--radius-md);
  font-size: 12.5px;
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
}
.retry {
  margin-left: auto;
  color: var(--accent);
  font-size: 12.5px;
}
.table {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
}
.thead,
.tr {
  display: grid;
  grid-template-columns: minmax(220px, 1.2fr) minmax(180px, 1.6fr) 118px;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
}
.thead {
  background: var(--bg-soft);
  font-size: 11.5px;
  color: var(--text-muted);
  font-weight: 600;
  letter-spacing: 0.5px;
  text-transform: uppercase;
  position: sticky;
  top: 0;
  z-index: 1;
  border-bottom: 1px solid var(--border);
}
.tbody {
  display: flex;
  flex-direction: column;
}
.tr {
  font-size: 12.5px;
  border-bottom: 1px solid var(--border);
}
.tr:last-child {
  border-bottom: none;
}
.tr:hover {
  background: var(--bg-elev);
}
.td {
  min-width: 0;
}
.name {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.var-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
}
.mono {
  font-family: var(--font-mono);
}
.badge {
  flex-shrink: 0;
  padding: 1px 7px;
  border-radius: 999px;
  font-size: 10.5px;
  font-weight: 600;
}
.badge.user {
  background: var(--accent-soft);
  color: var(--accent);
}
.badge.machine {
  background: rgba(251, 191, 36, 0.16);
  color: var(--warning);
}
.badge.process {
  background: rgba(74, 222, 128, 0.14);
  color: var(--success);
}
.note-input {
  width: 100%;
  height: 30px;
  padding: 0 8px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: var(--text);
  outline: none;
}
.note-input:hover,
.note-input:focus {
  border-color: var(--border);
  background: var(--bg-soft);
}
.act {
  display: flex;
  gap: 4px;
  justify-content: flex-end;
}
.icon-btn {
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  border-radius: 6px;
  color: var(--text-muted);
  background: var(--bg-elev);
}
.icon-btn:hover {
  background: var(--accent);
  color: #fff;
}
.icon-btn.danger:hover {
  background: var(--danger);
}
.spin {
  animation: spin 1s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 50px 20px;
  color: var(--text-muted);
}
.empty span {
  font-size: 38px;
  opacity: 0.4;
}
.modal-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(4px);
  display: grid;
  place-items: center;
  z-index: 200;
}
.modal {
  width: 420px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-xl);
  overflow: hidden;
}
.modal header {
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
}
.modal h2 {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
}
.modal-body {
  padding: 16px 18px;
  font-size: 13px;
}
.modal-body p {
  margin: 0 0 10px;
}
.warn {
  display: inline-flex;
  align-items: flex-start;
  gap: 6px;
  color: var(--warning);
  font-size: 12px;
}
.modal footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 18px;
  border-top: 1px solid var(--border);
}
.btn-danger {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 14px;
  background: var(--danger);
  color: #fff;
  border-radius: var(--radius-md);
  font-size: 13px;
  font-weight: 500;
}
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

:global(html[data-ui-mode='mini'] .body) {
  padding: 8px 10px 16px;
}
:global(html[data-ui-mode='mini'] .page-header .right) {
  flex-wrap: wrap;
}
:global(html[data-ui-mode='mini'] .thead),
:global(html[data-ui-mode='mini'] .tr) {
  grid-template-columns: minmax(140px, 1.1fr) minmax(120px, 1fr) 96px;
  padding: 8px 10px;
  gap: 8px;
}
:global(html[data-ui-mode='mini'] .chips) {
  order: 3;
  width: 100%;
}
:global(html[data-ui-mode='mini'] .chip) {
  height: 28px;
  flex: 1;
}
:global(html[data-ui-mode='mini'] .btn-ghost),
:global(html[data-ui-mode='mini'] .btn-primary) {
  height: 32px;
  padding: 0 10px;
}
</style>
