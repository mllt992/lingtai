<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useLauncherStore } from '@/stores/launcher'
import { useDrop } from '@/composables/useDrop'
import PageHeader from '@/components/PageHeader.vue'
import ContextMenu, { type MenuItem } from '@/components/ContextMenu.vue'
import PromptDialog from '@/components/PromptDialog.vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'
import type { ResourceGroup, ResourceItem, ResourceKind } from '@/types'

const launcher = useLauncherStore()

// 添加资源对话框
const showAddDialog = ref(false)
const addKind = ref<ResourceKind>('folder')
const addTargetGroupId = ref('')
const newName = ref('')
const newPath = ref('')
const dropHint = ref(false)

// 右键菜单
const ctxOpen = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxItems = ref<MenuItem[]>([])

// 输入框 dialog（新建分组 / 重命名分组 / 重命名条目）
type PromptKind =
  | { kind: 'add-group' }
  | { kind: 'rename-group'; group: ResourceGroup }
  | { kind: 'rename-item'; item: ResourceItem }
const prompt = ref<{
  open: boolean
  title: string
  label: string
  placeholder: string
  initial: string
  confirmText: string
  danger?: boolean
  payload?: PromptKind
}>({
  open: false,
  title: '',
  label: '',
  placeholder: '',
  initial: '',
  confirmText: '确定'
})

// 拖拽状态
const dragId = ref<string | null>(null)
const dropTarget = ref<{ groupId: string; itemId: string | null } | null>(null)

onMounted(async () => {
  await launcher.load()
})

useDrop(({ paths }) => {
  const target = launcher.sortedResourceGroups[0]
  if (!target) return
  for (const p of paths) {
    const isUrl = /^https?:\/\//i.test(p)
    const kind: ResourceKind = isUrl
      ? 'url'
      : /\.[a-z0-9]{1,5}$/i.test(p)
        ? 'file'
        : 'folder'
    const name = isUrl ? p : p.split(/[\\\/]/).pop() || p
    launcher.addResource(target.id, name, p, kind)
  }
  dropHint.value = true
  setTimeout(() => (dropHint.value = false), 1500)
})

function openAddDialog(kind: ResourceKind, groupId?: string) {
  addKind.value = kind
  addTargetGroupId.value = groupId ?? launcher.sortedResourceGroups[0]?.id ?? ''
  newName.value = ''
  newPath.value = ''
  showAddDialog.value = true
}

async function browse() {
  if (addKind.value === 'url') return
  const selected = await openDialog({
    multiple: false,
    directory: addKind.value === 'folder'
  })
  if (typeof selected === 'string') {
    newPath.value = selected
    if (!newName.value) newName.value = selected.split(/[\\\/]/).pop() || ''
  }
}

async function confirmAdd() {
  if (!newPath.value.trim() || !addTargetGroupId.value) return
  let p = newPath.value
  if (addKind.value === 'url' && !/^https?:\/\//i.test(p)) p = 'https://' + p
  await launcher.addResource(addTargetGroupId.value, newName.value, p, addKind.value)
  showAddDialog.value = false
}

function iconFor(kind: ResourceKind) {
  if (kind === 'folder') return 'i-carbon-folder'
  if (kind === 'file') return 'i-carbon-document'
  return 'i-carbon-link'
}

function colorFor(kind: ResourceKind) {
  if (kind === 'folder') return 'var(--warning)'
  if (kind === 'file') return 'var(--accent)'
  return 'var(--success)'
}

// ===== 右键菜单 =====
function showItemMenu(e: MouseEvent, item: ResourceItem) {
  e.preventDefault()
  e.stopPropagation()
  const items: MenuItem[] = [
    {
      label: item.kind === 'url' ? '打开网址' : '打开',
      icon: 'i-carbon-launch',
      onClick: () => launcher.openResource(item)
    }
  ]
  if (item.kind !== 'url') {
    items.push({
      label: '在资源管理器中显示',
      icon: 'i-carbon-folder-open',
      onClick: () => invoke('reveal_in_explorer', { path: item.path }).catch(() => {})
    })
  }
  const moveSubmenu: MenuItem[] = launcher.sortedResourceGroups
    .filter((g) => g.id !== item.groupId)
    .map((g) => ({
      label: g.name,
      icon: 'i-carbon-folder',
      onClick: () => launcher.moveResource(item.id, g.id)
    }))
  items.push(
    {
      label: '移到分组',
      icon: 'i-carbon-move',
      disabled: moveSubmenu.length === 0,
      submenu: moveSubmenu.length ? moveSubmenu : undefined
    },
    {
      label: '重命名',
      icon: 'i-carbon-edit',
      onClick: () =>
        openPrompt(
          { kind: 'rename-item', item } as PromptKind,
          { title: '重命名', label: '新名称', initial: item.name }
        )
    },
    { divider: true, label: '' },
    {
      label: item.kind === 'url' ? '复制网址' : '复制路径',
      icon: 'i-carbon-copy',
      onClick: () => navigator.clipboard.writeText(item.path)
    },
    { divider: true, label: '' },
    {
      label: '移除',
      icon: 'i-carbon-trash-can',
      danger: true,
      onClick: () => launcher.removeResource(item.id)
    }
  )
  ctxItems.value = items
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxOpen.value = true
}

function showGroupMenu(e: MouseEvent, group: ResourceGroup) {
  e.preventDefault()
  e.stopPropagation()
  ctxItems.value = [
    {
      label: '添加文件夹',
      icon: 'i-carbon-folder-add',
      onClick: () => openAddDialog('folder', group.id)
    },
    {
      label: '添加文件',
      icon: 'i-carbon-document-add',
      onClick: () => openAddDialog('file', group.id)
    },
    {
      label: '添加网址',
      icon: 'i-carbon-link',
      onClick: () => openAddDialog('url', group.id)
    },
    { divider: true, label: '' },
    {
      label: '重命名',
      icon: 'i-carbon-edit',
      onClick: () =>
        openPrompt(
          { kind: 'rename-group', group } as PromptKind,
          { title: '重命名分组', label: '分组名称', initial: group.name }
        )
    },
    {
      label: group.collapsed ? '展开' : '折叠',
      icon: group.collapsed ? 'i-carbon-chevron-down' : 'i-carbon-chevron-up',
      onClick: () => launcher.toggleResourceGroupCollapsed(group.id)
    },
    { divider: true, label: '' },
    {
      label: '删除分组',
      icon: 'i-carbon-trash-can',
      danger: true,
      disabled: launcher.resourceGroups.length <= 1,
      onClick: () => removeGroupWithConfirm(group)
    }
  ]
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxOpen.value = true
}

async function removeGroupWithConfirm(group: ResourceGroup) {
  const items = launcher.resourcesByGroup[group.id] ?? []
  if (items.length === 0) {
    await launcher.removeResourceGroup(group.id)
    return
  }
  const fallback = launcher.sortedResourceGroups.find((g) => g.id !== group.id)
  if (fallback) {
    if (confirm(`分组 "${group.name}" 内有 ${items.length} 项，移动到 "${fallback.name}" 后删除？`)) {
      await launcher.removeResourceGroup(group.id, fallback.id)
    }
  }
}

function openPrompt(payload: PromptKind, opts: Partial<typeof prompt.value>) {
  prompt.value = {
    open: true,
    title: opts.title ?? '',
    label: opts.label ?? '',
    placeholder: opts.placeholder ?? '',
    initial: opts.initial ?? '',
    confirmText: opts.confirmText ?? '确定',
    danger: opts.danger,
    payload
  }
}

async function onPromptConfirm(value: string) {
  const p = prompt.value.payload
  prompt.value.open = false
  if (!p || !value.trim()) return
  switch (p.kind) {
    case 'add-group':
      await launcher.addResourceGroup(value)
      break
    case 'rename-group':
      await launcher.renameResourceGroup(p.group.id, value)
      break
    case 'rename-item':
      await launcher.renameResource(p.item.id, value)
      break
  }
}

// ===== 拖拽 =====
function onDragStart(e: DragEvent, item: ResourceItem) {
  if (!e.dataTransfer) return
  dragId.value = item.id
  e.dataTransfer.effectAllowed = 'move'
  e.dataTransfer.setData('text/plain', item.id)
}
function onDragEnd() {
  dragId.value = null
  dropTarget.value = null
}
function onCardDragOver(e: DragEvent, groupId: string, itemId: string) {
  if (!dragId.value || dragId.value === itemId) return
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  dropTarget.value = { groupId, itemId }
}
function onGroupDragOver(e: DragEvent, groupId: string) {
  if (!dragId.value) return
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  if (!dropTarget.value || dropTarget.value.groupId !== groupId) {
    dropTarget.value = { groupId, itemId: null }
  }
}
async function onDrop(e: DragEvent, groupId: string, itemId: string | null) {
  e.preventDefault()
  const src = dragId.value
  if (!src) return
  await launcher.dropResource(src, groupId, itemId)
  dragId.value = null
  dropTarget.value = null
}

const totalItems = computed(() => launcher.resources.length)
const empty = computed(() => totalItems.value === 0)
</script>

<template>
  <div class="page">
    <PageHeader title="资源归纳" subtitle="分组管理 · 拖拽排序 · 右键操作 · 拖入文件即收纳">
      <template #actions>
        <div class="search">
          <span class="i-carbon-search" />
          <input v-model="launcher.keyword" placeholder="搜索 名称 / 路径…" />
        </div>
        <button
          class="btn-ghost"
          @click="
            openPrompt({ kind: 'add-group' } as PromptKind, {
              title: '新建分组',
              label: '分组名称',
              placeholder: '比如 项目 / 文档 / 常用网站',
              confirmText: '新建'
            })
          "
        >
          <span class="i-carbon-folder-add" /> 新分组
        </button>
        <button class="btn-ghost" @click="openAddDialog('folder')">
          <span class="i-carbon-folder-add" /> 文件夹
        </button>
        <button class="btn-ghost" @click="openAddDialog('file')">
          <span class="i-carbon-document-add" /> 文件
        </button>
        <button class="btn-primary" @click="openAddDialog('url')">
          <span class="i-carbon-link" /> 网址
        </button>
      </template>
    </PageHeader>

    <div class="body scrollbar-thin" :class="{ 'drop-active': dropHint }">
      <section
        v-for="group in launcher.sortedResourceGroups"
        :key="group.id"
        class="group"
        :class="{ collapsed: group.collapsed, 'is-drop': dropTarget?.groupId === group.id }"
        @dragover="onGroupDragOver($event, group.id)"
        @drop="onDrop($event, group.id, null)"
      >
        <header class="group-head" @contextmenu="showGroupMenu($event, group)">
          <button class="caret" @click="launcher.toggleResourceGroupCollapsed(group.id)">
            <span
              :class="group.collapsed ? 'i-carbon-chevron-right' : 'i-carbon-chevron-down'"
            />
          </button>
          <h3>{{ group.name }}</h3>
          <em>{{ (launcher.resourcesByGroup[group.id] ?? []).length }}</em>
          <div class="head-actions">
            <button class="head-btn" title="添加文件夹" @click="openAddDialog('folder', group.id)">
              <span class="i-carbon-folder-add" />
            </button>
            <button class="head-btn" title="添加文件" @click="openAddDialog('file', group.id)">
              <span class="i-carbon-document-add" />
            </button>
            <button class="head-btn" title="添加网址" @click="openAddDialog('url', group.id)">
              <span class="i-carbon-link" />
            </button>
            <button class="head-btn" title="更多" @click="showGroupMenu($event, group)">
              <span class="i-carbon-overflow-menu-horizontal" />
            </button>
          </div>
        </header>

        <transition name="slide">
          <div v-if="!group.collapsed" class="grid">
            <div
              v-for="item in launcher.resourcesByGroup[group.id] ?? []"
              :key="item.id"
              class="res-card"
              :class="{
                dragging: dragId === item.id,
                'drop-before':
                  dropTarget?.groupId === group.id &&
                  dropTarget?.itemId === item.id &&
                  dragId !== item.id
              }"
              draggable="true"
              :title="item.path"
              @dblclick="launcher.openResource(item)"
              @contextmenu="showItemMenu($event, item)"
              @dragstart="onDragStart($event, item)"
              @dragend="onDragEnd"
              @dragover="onCardDragOver($event, group.id, item.id)"
              @drop="onDrop($event, group.id, item.id)"
            >
              <div class="icon" :style="{ color: colorFor(item.kind) }">
                <span :class="iconFor(item.kind)" />
              </div>
              <div class="info">
                <div class="name">{{ item.name }}</div>
                <div class="path">{{ item.path }}</div>
              </div>
            </div>
            <button
              v-if="(launcher.resourcesByGroup[group.id] ?? []).length === 0"
              class="empty-slot"
              @click="openAddDialog('folder', group.id)"
            >
              <span class="i-carbon-add" />
              <span>添加到此分组</span>
            </button>
          </div>
        </transition>
      </section>

      <div v-if="empty && launcher.resourceGroups.length === 1" class="empty-state">
        <div class="empty-logo">
          <span class="i-carbon-folder-shared" />
        </div>
        <h2>资源库还是空的</h2>
        <p>点击右上角添加，或直接把 <strong>文件 / 文件夹</strong> 拖到这里</p>
        <div class="empty-actions">
          <button class="btn-ghost big" @click="openAddDialog('folder')">
            <span class="i-carbon-folder-add" /> 文件夹
          </button>
          <button class="btn-ghost big" @click="openAddDialog('file')">
            <span class="i-carbon-document-add" /> 文件
          </button>
          <button class="btn-primary big" @click="openAddDialog('url')">
            <span class="i-carbon-link" /> 网址
          </button>
        </div>
      </div>

      <transition name="fade">
        <div v-if="dropHint" class="drop-banner">
          <span class="i-carbon-add-large" /> 已收入资源库
        </div>
      </transition>
    </div>

    <!-- 添加资源对话框 -->
    <transition name="fade">
      <div v-if="showAddDialog" class="modal-mask" @click.self="showAddDialog = false">
        <div class="modal">
          <header>
            <h2>
              <span v-if="addKind === 'folder'">添加文件夹</span>
              <span v-else-if="addKind === 'file'">添加文件</span>
              <span v-else>添加网址</span>
            </h2>
            <button class="icon-btn" @click="showAddDialog = false">
              <span class="i-carbon-close" />
            </button>
          </header>
          <div class="modal-body">
            <label class="field">
              <span>添加到分组</span>
              <select v-model="addTargetGroupId" class="input-base">
                <option v-for="g in launcher.sortedResourceGroups" :key="g.id" :value="g.id">
                  {{ g.name }}
                </option>
              </select>
            </label>
            <label class="field">
              <span>名称（可选）</span>
              <input
                v-model="newName"
                :placeholder="addKind === 'url' ? '比如 GitHub' : '默认使用文件名'"
                class="input-base"
              />
            </label>
            <label class="field">
              <span>{{ addKind === 'url' ? '网址' : '路径' }}</span>
              <div class="row">
                <input
                  v-model="newPath"
                  :placeholder="
                    addKind === 'url' ? 'https://github.com' :
                    addKind === 'folder' ? 'D:\\Projects' :
                    'D:\\Projects\\readme.md'
                  "
                  class="input-base flex-1"
                />
                <button v-if="addKind !== 'url'" class="btn-ghost" @click="browse">
                  <span class="i-carbon-folder-open" /> 浏览
                </button>
              </div>
            </label>
          </div>
          <footer>
            <button class="btn-ghost" @click="showAddDialog = false">取消</button>
            <button class="btn-primary" :disabled="!newPath.trim()" @click="confirmAdd">
              确认添加
            </button>
          </footer>
        </div>
      </div>
    </transition>

    <ContextMenu
      :open="ctxOpen"
      :x="ctxPos.x"
      :y="ctxPos.y"
      :items="ctxItems"
      @close="ctxOpen = false"
    />
    <PromptDialog
      :open="prompt.open"
      :title="prompt.title"
      :label="prompt.label"
      :placeholder="prompt.placeholder"
      :initial="prompt.initial"
      :confirm-text="prompt.confirmText"
      :danger="prompt.danger"
      @close="prompt.open = false"
      @confirm="onPromptConfirm"
    />
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
  border-radius: 10px;
  padding: 0 10px;
  height: 36px;
  color: var(--text-muted);
  min-width: 220px;
}
.search input {
  background: transparent;
  border: none;
  outline: none;
  color: var(--text);
  flex: 1;
}
.btn-ghost {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 36px;
  padding: 0 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  color: var(--text-muted);
  font-size: 13px;
  transition: all 0.15s;
}
.btn-ghost:hover {
  color: var(--text);
  border-color: var(--accent);
}
.btn-primary {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 36px;
  padding: 0 14px;
  background: var(--accent);
  color: #fff;
  border-radius: 10px;
  font-weight: 500;
  font-size: 13px;
  transition: opacity 0.15s;
}
.btn-primary:hover {
  opacity: 0.9;
}
.btn-primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.btn-primary.big,
.btn-ghost.big {
  height: 40px;
  padding: 0 18px;
  font-size: 14px;
}

.body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 20px 24px;
  position: relative;
}
.body.drop-active {
  outline: 2px dashed var(--accent);
  outline-offset: -8px;
}

.group {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  margin-bottom: 12px;
  padding: 4px 14px 12px;
  transition: border-color 0.15s, background 0.15s;
}
.group.is-drop {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.group.collapsed {
  padding-bottom: 4px;
}

.group-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0 8px;
  color: var(--text);
  cursor: pointer;
}
.caret {
  width: 22px;
  height: 22px;
  display: grid;
  place-items: center;
  color: var(--text-muted);
  border-radius: 5px;
}
.caret:hover {
  background: var(--bg-elev);
  color: var(--text);
}
.group-head h3 {
  margin: 0;
  font-size: 13.5px;
  font-weight: 600;
  letter-spacing: 0.3px;
}
.group-head em {
  font-style: normal;
  font-size: 11px;
  color: var(--text-muted);
  background: var(--bg-elev);
  padding: 1px 7px;
  border-radius: 99px;
}
.head-actions {
  margin-left: auto;
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.15s;
}
.group:hover .head-actions {
  opacity: 1;
}
.head-btn {
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  border-radius: 6px;
  color: var(--text-muted);
}
.head-btn:hover {
  background: var(--bg-elev);
  color: var(--text);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 8px;
  padding: 4px 0;
}

.res-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--bg-elev);
  border: 1.5px solid transparent;
  border-radius: 10px;
  cursor: pointer;
  user-select: none;
  position: relative;
  transition: all 0.15s;
}
.res-card:hover {
  border-color: var(--accent);
  transform: translateY(-1px);
}
.res-card.dragging {
  opacity: 0.4;
}
.res-card.drop-before::before {
  content: '';
  position: absolute;
  left: -4px;
  top: 8px;
  bottom: 8px;
  width: 3px;
  border-radius: 2px;
  background: var(--accent);
}
.res-card .icon {
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  font-size: 22px;
  flex-shrink: 0;
  background: var(--bg-card);
  border-radius: 8px;
}
.res-card .info {
  flex: 1;
  min-width: 0;
}
.res-card .name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.res-card .path {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: var(--font-mono);
  margin-top: 2px;
}

.empty-slot {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 60px;
  padding: 12px;
  background: transparent;
  border: 1.5px dashed var(--border);
  border-radius: 10px;
  color: var(--text-muted);
  font-size: 12px;
  transition: all 0.15s;
}
.empty-slot:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}
.empty-slot > span:first-child {
  font-size: 18px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 70px 20px;
  text-align: center;
}
.empty-logo {
  width: 80px;
  height: 80px;
  border-radius: 22px;
  background: var(--accent-soft);
  display: grid;
  place-items: center;
  font-size: 36px;
  color: var(--accent);
  margin-bottom: 8px;
}
.empty-state h2 {
  margin: 6px 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
}
.empty-state p {
  margin: 0;
  font-size: 13px;
  color: var(--text-muted);
  max-width: 480px;
}
.empty-state strong {
  color: var(--accent);
}
.empty-actions {
  display: flex;
  gap: 8px;
  margin-top: 14px;
}

.drop-banner {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--accent);
  color: #fff;
  padding: 10px 18px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  z-index: 150;
}

/* 添加资源对话框 */
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
  width: 460px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
}
.modal header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 18px;
  border-bottom: 1px solid var(--border);
}
.modal h2 {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
}
.icon-btn {
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border-radius: 6px;
  color: var(--text-muted);
}
.icon-btn:hover {
  background: var(--bg-elev);
  color: var(--text);
}
.modal-body {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 14px;
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
.modal footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 14px 18px;
  border-top: 1px solid var(--border);
}

.slide-enter-active, .slide-leave-active {
  transition: opacity 0.18s ease, max-height 0.22s ease, padding 0.18s ease;
  overflow: hidden;
}
.slide-enter-from, .slide-leave-to {
  opacity: 0;
  max-height: 0;
  padding-top: 0;
  padding-bottom: 0;
}
.slide-enter-to, .slide-leave-from {
  opacity: 1;
  max-height: 1500px;
}
</style>
