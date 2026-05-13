<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useLauncherStore } from '@/stores/launcher'
import { useDrop } from '@/composables/useDrop'
import PageHeader from '@/components/PageHeader.vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import type { ResourceItem } from '@/types'

const launcher = useLauncherStore()

const showAddDialog = ref(false)
const addKind = ref<ResourceItem['kind']>('folder')
const newName = ref('')
const newPath = ref('')
const dropHint = ref(false)

onMounted(async () => {
  await launcher.load()
})

useDrop(({ paths }) => {
  for (const p of paths) {
    const isUrl = /^https?:\/\//i.test(p)
    const kind: ResourceItem['kind'] = isUrl
      ? 'url'
      : /\.[a-z0-9]{1,5}$/i.test(p)
        ? 'file'
        : 'folder'
    const name = isUrl ? p : p.split(/[\\\/]/).pop() || p
    launcher.addResource(name, p, kind)
  }
  dropHint.value = true
  setTimeout(() => (dropHint.value = false), 1500)
})

function openAdd(kind: ResourceItem['kind']) {
  addKind.value = kind
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
    if (!newName.value) {
      newName.value = selected.split(/[\\\/]/).pop() || ''
    }
  }
}

async function confirmAdd() {
  if (!newPath.value.trim()) return
  if (addKind.value === 'url' && !/^https?:\/\//i.test(newPath.value)) {
    newPath.value = 'https://' + newPath.value
  }
  await launcher.addResource(newName.value, newPath.value, addKind.value)
  showAddDialog.value = false
}

const groupOrder = ['文件夹', '文件', '网址'] as const

function iconFor(kind: ResourceItem['kind']) {
  if (kind === 'folder') return 'i-carbon-folder'
  if (kind === 'file') return 'i-carbon-document'
  return 'i-carbon-link'
}

function colorFor(kind: ResourceItem['kind']) {
  if (kind === 'folder') return 'var(--warning)'
  if (kind === 'file') return 'var(--accent)'
  return 'var(--success)'
}

const sortedGroups = computed(() => {
  const groups = launcher.resourceGroups
  return groupOrder
    .filter((k) => groups[k]?.length)
    .map((k) => ({ name: k, items: groups[k] }))
})
</script>

<template>
  <div class="page">
    <PageHeader
      title="资源归纳"
      subtitle="文件夹 / 文件 / 网址 一站式收纳 · 双击打开 · 直接拖入"
    >
      <template #actions>
        <div class="search">
          <span class="i-carbon-search" />
          <input v-model="launcher.keyword" placeholder="搜索 名称 / 路径…" />
        </div>
        <button class="btn-ghost" @click="openAdd('folder')">
          <span class="i-carbon-folder-add" /> 添加文件夹
        </button>
        <button class="btn-ghost" @click="openAdd('file')">
          <span class="i-carbon-document-add" /> 添加文件
        </button>
        <button class="btn-primary" @click="openAdd('url')">
          <span class="i-carbon-link" /> 添加网址
        </button>
      </template>
    </PageHeader>

    <div class="body scrollbar-thin" :class="{ 'drop-active': dropHint }">
      <section v-for="g in sortedGroups" :key="g.name" class="section">
        <h3>{{ g.name }} <em>{{ g.items.length }}</em></h3>
        <div class="grid">
          <div
            v-for="item in g.items"
            :key="item.id"
            class="card-hover res-card"
            @dblclick="launcher.openResource(item)"
            :title="item.path"
          >
            <div class="icon" :style="{ background: 'var(--accent-soft)', color: colorFor(item.kind) }">
              <span :class="iconFor(item.kind)" />
            </div>
            <div class="info">
              <div class="name">{{ item.name }}</div>
              <div class="path">{{ item.path }}</div>
            </div>
            <div class="actions">
              <button
                class="icon-btn"
                title="打开"
                @click.stop="launcher.openResource(item)"
              >
                <span class="i-carbon-launch" />
              </button>
              <button
                class="icon-btn danger"
                title="移除"
                @click.stop="launcher.removeResource(item.id)"
              >
                <span class="i-carbon-trash-can" />
              </button>
            </div>
          </div>
        </div>
      </section>

      <div v-if="sortedGroups.length === 0" class="empty">
        <div class="empty-icon i-carbon-folder-shared" />
        <p>暂无资源，点击右上角添加，或直接把文件 / 文件夹拖到这里。</p>
        <p class="small">网址支持手动添加 https://…</p>
      </div>

      <transition name="fade">
        <div v-if="dropHint" class="drop-banner">
          <span class="i-carbon-add-large" /> 已收入资源库
        </div>
      </transition>
    </div>

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
          <div class="body">
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
  border-radius: var(--radius-md);
  color: var(--text-muted);
  font-size: 13px;
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
  border-radius: var(--radius-md);
  font-weight: 500;
  font-size: 13px;
}
.btn-primary:hover {
  opacity: 0.9;
}
.btn-primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 24px 24px;
}
.body.drop-active {
  outline: 2px dashed var(--accent);
  outline-offset: -8px;
}

.section + .section {
  margin-top: 22px;
}
.section h3 {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-muted);
  font-weight: 500;
  margin: 4px 0 12px;
}
.section h3 em {
  font-style: normal;
  font-size: 11px;
  background: var(--bg-elev);
  padding: 2px 7px;
  border-radius: 99px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 10px;
}

.res-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.18s;
}
.res-card:hover {
  border-color: var(--accent);
  transform: translateY(-1px);
  box-shadow: var(--shadow-card);
}
.icon {
  width: 38px;
  height: 38px;
  border-radius: var(--radius-md);
  display: grid;
  place-items: center;
  font-size: 20px;
  flex-shrink: 0;
}
.info {
  min-width: 0;
  flex: 1;
}
.name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.path {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: var(--font-mono);
}
.actions {
  display: none;
  gap: 4px;
}
.res-card:hover .actions {
  display: flex;
}
.icon-btn {
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border-radius: 6px;
  color: var(--text-muted);
  background: var(--bg-elev);
  transition: all 0.15s;
}
.icon-btn:hover {
  background: var(--accent);
  color: #fff;
}
.icon-btn.danger:hover {
  background: var(--danger);
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 60px 20px;
  color: var(--text-muted);
}
.empty-icon {
  font-size: 48px;
  opacity: 0.4;
}
.empty .small {
  font-size: 11.5px;
  opacity: 0.7;
}

.drop-banner {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--accent);
  color: #fff;
  padding: 10px 18px;
  border-radius: var(--radius-md);
  font-size: 13px;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  box-shadow: var(--shadow-elev);
  z-index: 100;
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
  width: 460px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-elev);
  overflow: hidden;
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
.modal .body {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  overflow: visible;
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
</style>
