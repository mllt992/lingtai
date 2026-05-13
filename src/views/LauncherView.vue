<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useLauncherStore } from '@/stores/launcher'
import { useDrop } from '@/composables/useDrop'
import PageHeader from '@/components/PageHeader.vue'
import AppIcon from '@/components/AppIcon.vue'
import AddItemsDialog from '@/components/AddItemsDialog.vue'
import ContextMenu, { type MenuItem } from '@/components/ContextMenu.vue'
import PromptDialog from '@/components/PromptDialog.vue'
import type { LauncherGroup, LauncherItem } from '@/types'

const launcher = useLauncherStore()

const showAdd = ref(false)
const addTargetGroupId = ref('')
const dropHint = ref(false)

// 右键菜单
const ctxOpen = ref(false)
const ctxPos = ref({ x: 0, y: 0 })
const ctxItems = ref<MenuItem[]>([])

// 输入框 dialog
type PromptKind =
  | { kind: 'add-group' }
  | { kind: 'rename-group'; group: LauncherGroup }
  | { kind: 'rename-item'; item: LauncherItem }
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
  // 后台扫描已装应用，供选择器使用
  if (launcher.autoApps.length === 0) launcher.scanApps()
})

// 文件拖入：作为快捷启动项加入第一个分组
useDrop(({ paths }) => {
  const targetGroup = launcher.sortedGroups[0]
  if (!targetGroup) return
  for (const p of paths) {
    const name = p.split(/[\\\/]/).pop()?.replace(/\.(exe|lnk|url|bat|cmd)$/i, '') || p
    launcher.addItem(targetGroup.id, { name, path: p })
  }
  dropHint.value = true
  setTimeout(() => (dropHint.value = false), 1500)
})

function openAddDialog(groupId?: string) {
  addTargetGroupId.value = groupId ?? launcher.sortedGroups[0]?.id ?? ''
  showAdd.value = true
}

// ====== 右键菜单 ======
function showItemMenu(e: MouseEvent, item: LauncherItem) {
  e.preventDefault()
  const moveSubmenu: MenuItem[] = launcher.sortedGroups
    .filter((g) => g.id !== item.groupId)
    .map((g) => ({
      label: g.name,
      icon: 'i-carbon-folder',
      onClick: () => launcher.moveItem(item.id, g.id)
    }))
  ctxItems.value = [
    { label: '启动', icon: 'i-carbon-play-filled-alt', onClick: () => launcher.launchItem(item) },
    {
      label: '重命名',
      icon: 'i-carbon-edit',
      onClick: () => openPrompt({
        kind: 'rename-item',
        item
      } as any, { title: '重命名', label: '新名称', initial: item.name })
    },
    {
      label: '刷新图标',
      icon: 'i-carbon-renew',
      onClick: () => launcher.refreshIcon(item.id)
    },
    {
      label: '移到分组',
      icon: 'i-carbon-move',
      disabled: moveSubmenu.length === 0,
      submenu: moveSubmenu.length ? moveSubmenu : undefined
    },
    { divider: true, label: '' },
    {
      label: '复制路径',
      icon: 'i-carbon-copy',
      onClick: () => navigator.clipboard.writeText(item.path)
    },
    { divider: true, label: '' },
    {
      label: '从启动器移除',
      icon: 'i-carbon-trash-can',
      danger: true,
      onClick: () => launcher.removeItem(item.id)
    }
  ]
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxOpen.value = true
}

function showGroupMenu(e: MouseEvent, group: LauncherGroup) {
  e.preventDefault()
  e.stopPropagation()
  ctxItems.value = [
    {
      label: '添加应用…',
      icon: 'i-carbon-add',
      onClick: () => openAddDialog(group.id)
    },
    {
      label: '重命名',
      icon: 'i-carbon-edit',
      onClick: () => openPrompt({ kind: 'rename-group', group } as any, {
        title: '重命名分组',
        label: '分组名称',
        initial: group.name
      })
    },
    {
      label: group.collapsed ? '展开' : '折叠',
      icon: group.collapsed ? 'i-carbon-chevron-down' : 'i-carbon-chevron-up',
      onClick: () => launcher.toggleGroupCollapsed(group.id)
    },
    { divider: true, label: '' },
    {
      label: '删除分组',
      icon: 'i-carbon-trash-can',
      danger: true,
      disabled: launcher.groups.length <= 1,
      onClick: () => removeGroupWithConfirm(group)
    }
  ]
  ctxPos.value = { x: e.clientX, y: e.clientY }
  ctxOpen.value = true
}

async function removeGroupWithConfirm(group: LauncherGroup) {
  const items = launcher.itemsByGroup[group.id] ?? []
  if (items.length === 0) {
    await launcher.removeGroup(group.id)
    return
  }
  // 若有条目，移到第一个其它分组
  const fallback = launcher.sortedGroups.find((g) => g.id !== group.id)
  if (fallback) {
    if (confirm(`分组 "${group.name}" 内有 ${items.length} 项，移动到 "${fallback.name}" 后删除？`)) {
      await launcher.removeGroup(group.id, fallback.id)
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
      await launcher.addGroup(value)
      break
    case 'rename-group':
      await launcher.renameGroup(p.group.id, value)
      break
    case 'rename-item':
      await launcher.renameItem(p.item.id, value)
      break
  }
}

// ====== 拖拽 ======
function onDragStart(e: DragEvent, item: LauncherItem) {
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
  // 若已有更精细 itemId，保留；否则默认追加末尾
  if (!dropTarget.value || dropTarget.value.groupId !== groupId) {
    dropTarget.value = { groupId, itemId: null }
  }
}
async function onDrop(e: DragEvent, groupId: string, itemId: string | null) {
  e.preventDefault()
  const src = dragId.value
  if (!src) return
  await launcher.dropItem(src, groupId, itemId)
  dragId.value = null
  dropTarget.value = null
}

const totalItems = computed(() => launcher.items.length)
const empty = computed(() => totalItems.value === 0)
</script>

<template>
  <div class="page">
    <PageHeader title="启动器" subtitle="分组管理 · 拖拽排序 · 右键操作 · 拖入文件即收纳">
      <template #actions>
        <div class="search">
          <span class="i-carbon-search" />
          <input v-model="launcher.keyword" placeholder="搜索 应用 / 路径…" />
        </div>
        <button
          class="btn-ghost"
          @click="openPrompt({ kind: 'add-group' } as any, {
            title: '新建分组',
            label: '分组名称',
            placeholder: '比如 开发 / 办公 / 娱乐',
            confirmText: '新建'
          })"
        >
          <span class="i-carbon-folder-add" /> 新分组
        </button>
        <button class="btn-primary" @click="openAddDialog()">
          <span class="i-carbon-add" /> 添加应用
        </button>
      </template>
    </PageHeader>

    <div class="body scrollbar-thin" :class="{ 'drop-active': dropHint }">
      <section
        v-for="group in launcher.sortedGroups"
        :key="group.id"
        class="group"
        :class="{ collapsed: group.collapsed, 'is-drop': dropTarget?.groupId === group.id }"
        @dragover="onGroupDragOver($event, group.id)"
        @drop="onDrop($event, group.id, null)"
      >
        <header class="group-head" @contextmenu="showGroupMenu($event, group)">
          <button class="caret" @click="launcher.toggleGroupCollapsed(group.id)">
            <span
              :class="group.collapsed ? 'i-carbon-chevron-right' : 'i-carbon-chevron-down'"
            />
          </button>
          <h3>{{ group.name }}</h3>
          <em>{{ (launcher.itemsByGroup[group.id] ?? []).length }}</em>
          <div class="head-actions">
            <button class="head-btn" title="添加" @click="openAddDialog(group.id)">
              <span class="i-carbon-add" />
            </button>
            <button class="head-btn" title="更多" @click="showGroupMenu($event, group)">
              <span class="i-carbon-overflow-menu-horizontal" />
            </button>
          </div>
        </header>

        <transition name="slide">
          <div v-if="!group.collapsed" class="grid">
            <div
              v-for="item in launcher.itemsByGroup[group.id] ?? []"
              :key="item.id"
              class="item-card"
              :class="{
                dragging: dragId === item.id,
                'drop-before':
                  dropTarget?.groupId === group.id && dropTarget?.itemId === item.id && dragId !== item.id
              }"
              draggable="true"
              :title="item.target || item.path"
              @click="launcher.launchItem(item)"
              @contextmenu="showItemMenu($event, item)"
              @dragstart="onDragStart($event, item)"
              @dragend="onDragEnd"
              @dragover="onCardDragOver($event, group.id, item.id)"
              @drop="onDrop($event, group.id, item.id)"
            >
              <AppIcon :name="item.name" :icon-data="item.iconData" :size="44" :rounded="12" />
              <div class="name">{{ item.name }}</div>
            </div>
            <button
              v-if="(launcher.itemsByGroup[group.id] ?? []).length === 0"
              class="empty-slot"
              @click="openAddDialog(group.id)"
            >
              <span class="i-carbon-add" />
              <span>添加到此分组</span>
            </button>
          </div>
        </transition>
      </section>

      <div v-if="empty && launcher.groups.length === 1" class="empty-state">
        <div class="empty-logo">
          <span class="i-carbon-grid" />
        </div>
        <h2>启动器还是空的</h2>
        <p>点击右上角 <strong>"添加应用"</strong>，从已装软件里挑选，或者直接把 .exe / .lnk 拖到这里</p>
        <button class="btn-primary big" @click="openAddDialog()">
          <span class="i-carbon-add" /> 添加应用
        </button>
      </div>

      <transition name="fade">
        <div v-if="dropHint" class="drop-banner">
          <span class="i-carbon-add-large" /> 已收入快速启动
        </div>
      </transition>
    </div>

    <AddItemsDialog
      :open="showAdd"
      :default-group-id="addTargetGroupId"
      @close="showAdd = false"
      @added="showAdd = false"
    />
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
  min-width: 240px;
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
.btn-primary.big {
  height: 40px;
  padding: 0 22px;
  font-size: 14px;
  margin-top: 14px;
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
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
  gap: 8px;
  padding: 4px 0;
}

.item-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 10px 6px 8px;
  background: transparent;
  border: 1.5px solid transparent;
  border-radius: 12px;
  cursor: pointer;
  user-select: none;
  position: relative;
  transition: all 0.15s;
}
.item-card:hover {
  background: var(--bg-elev);
  transform: translateY(-2px);
}
.item-card.dragging {
  opacity: 0.4;
}
.item-card.drop-before::before {
  content: '';
  position: absolute;
  left: -4px;
  top: 8px;
  bottom: 8px;
  width: 3px;
  border-radius: 2px;
  background: var(--accent);
}
.item-card .name {
  font-size: 11.5px;
  color: var(--text);
  text-align: center;
  width: 100%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.empty-slot {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 88px;
  padding: 12px;
  background: transparent;
  border: 1.5px dashed var(--border);
  border-radius: 12px;
  color: var(--text-muted);
  font-size: 11.5px;
  transition: all 0.15s;
}
.empty-slot:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}
.empty-slot > span:first-child {
  font-size: 22px;
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
