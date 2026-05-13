<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useLauncherStore } from '@/stores/launcher'
import { useSettingsStore } from '@/stores/settings'
import { useDrop } from '@/composables/useDrop'
import PageHeader from '@/components/PageHeader.vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'

const launcher = useLauncherStore()
const settings = useSettingsStore()

const showAddDialog = ref(false)
const newName = ref('')
const newPath = ref('')
const dropHint = ref(false)

onMounted(async () => {
  await launcher.load()
  if (settings.launcher.autoScan && launcher.autoApps.length === 0) {
    launcher.scanApps()
  }
})

useDrop(({ paths }) => {
  // Treat each dropped path as a manual launcher item
  for (const p of paths) {
    const name = p.split(/[\\\/]/).pop()?.replace(/\.(exe|lnk|url)$/i, '') || p
    launcher.addLauncherItem(name, p)
  }
  flashDrop()
})

function flashDrop() {
  dropHint.value = true
  setTimeout(() => (dropHint.value = false), 1500)
}

async function browseFile() {
  const selected = await openDialog({
    multiple: false,
    directory: false,
    filters: [
      { name: '可执行 / 快捷方式', extensions: ['exe', 'lnk', 'bat', 'cmd', 'url'] },
      { name: '所有文件', extensions: ['*'] }
    ]
  })
  if (typeof selected === 'string') {
    newPath.value = selected
    if (!newName.value) {
      newName.value =
        selected.split(/[\\\/]/).pop()?.replace(/\.(exe|lnk|url)$/i, '') || ''
    }
  }
}

async function confirmAdd() {
  if (!newPath.value.trim()) return
  await launcher.addLauncherItem(newName.value, newPath.value)
  newName.value = ''
  newPath.value = ''
  showAddDialog.value = false
}

function initials(name: string) {
  return (name || '?').trim().charAt(0).toUpperCase()
}

const empty = computed(
  () => launcher.filteredApps.length === 0 && launcher.filteredManual.length === 0
)
</script>

<template>
  <div class="page">
    <PageHeader title="启动器" subtitle="自动扫描开始菜单 · 手动添加 · 拖拽即收纳">
      <template #actions>
        <div class="search">
          <span class="i-carbon-search" />
          <input v-model="launcher.keyword" placeholder="搜索 应用 / 别名…" />
        </div>
        <button class="btn-ghost" @click="launcher.scanApps" :disabled="launcher.scanning">
          <span class="i-carbon-renew" />
          <span>{{ launcher.scanning ? '扫描中…' : '重新扫描' }}</span>
        </button>
        <button class="btn-primary" @click="showAddDialog = true">
          <span class="i-carbon-add" />
          <span>添加</span>
        </button>
      </template>
    </PageHeader>

    <div class="body scrollbar-thin" :class="{ 'drop-active': dropHint }">
      <section v-if="launcher.filteredManual.length" class="section">
        <h3>
          <span class="i-carbon-star-filled" /> 我的常用
          <em>{{ launcher.filteredManual.length }}</em>
        </h3>
        <div class="grid">
          <div
            v-for="item in launcher.filteredManual"
            :key="item.id"
            class="card-hover app-card"
            @dblclick="launcher.launchApp(item)"
            :title="item.path"
          >
            <div class="logo" :style="{ background: 'var(--accent-soft)', color: 'var(--accent)' }">
              {{ initials(item.name) }}
            </div>
            <div class="info">
              <div class="name">{{ item.name }}</div>
              <div class="path">{{ item.path }}</div>
            </div>
            <div class="actions">
              <button
                class="icon-btn"
                title="启动"
                @click.stop="launcher.launchApp(item)"
              >
                <span class="i-carbon-play-filled-alt" />
              </button>
              <button
                class="icon-btn danger"
                title="移除"
                @click.stop="launcher.removeLauncherItem(item.id)"
              >
                <span class="i-carbon-trash-can" />
              </button>
            </div>
          </div>
        </div>
      </section>

      <section class="section">
        <h3>
          <span class="i-carbon-application" /> 已安装应用
          <em>{{ launcher.filteredApps.length }}</em>
          <span v-if="launcher.scanning" class="hint">扫描中…</span>
        </h3>
        <div v-if="launcher.scanError" class="error">{{ launcher.scanError }}</div>
        <div class="grid">
          <div
            v-for="app in launcher.filteredApps"
            :key="app.path"
            class="card-hover app-card"
            @dblclick="launcher.launchApp(app)"
            :title="app.target || app.path"
          >
            <div class="logo gradient">{{ initials(app.name) }}</div>
            <div class="info">
              <div class="name">{{ app.name }}</div>
              <div class="path">{{ app.target || app.path }}</div>
            </div>
            <div class="actions">
              <button
                class="icon-btn"
                title="启动"
                @click.stop="launcher.launchApp(app)"
              >
                <span class="i-carbon-play-filled-alt" />
              </button>
              <button
                class="icon-btn"
                title="收藏到常用"
                @click.stop="launcher.addLauncherItem(app.name, app.path)"
              >
                <span class="i-carbon-star" />
              </button>
            </div>
          </div>
        </div>
        <div v-if="empty && !launcher.scanning" class="empty">
          <div class="empty-icon i-carbon-folder-add" />
          <p>暂无数据，点击 "重新扫描" 或 "添加"，也可直接把 .exe / .lnk 拖到这里。</p>
        </div>
      </section>

      <transition name="fade">
        <div v-if="dropHint" class="drop-banner">
          <span class="i-carbon-add-large" /> 已收入快速启动
        </div>
      </transition>
    </div>

    <!-- Add Dialog -->
    <transition name="fade">
      <div v-if="showAddDialog" class="modal-mask" @click.self="showAddDialog = false">
        <div class="modal">
          <header>
            <h2>添加启动项</h2>
            <button class="icon-btn" @click="showAddDialog = false">
              <span class="i-carbon-close" />
            </button>
          </header>
          <div class="body">
            <label class="field">
              <span>名称（可选）</span>
              <input v-model="newName" placeholder="比如 VSCode" class="input-base" />
            </label>
            <label class="field">
              <span>路径</span>
              <div class="row">
                <input
                  v-model="newPath"
                  placeholder="C:\Program Files\…\app.exe"
                  class="input-base flex-1"
                />
                <button class="btn-ghost" @click="browseFile">
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
  border-radius: var(--radius-md);
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
  border-radius: var(--radius-md);
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

.body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 24px 24px;
  position: relative;
}
.body.drop-active {
  outline: 2px dashed var(--accent);
  outline-offset: -8px;
}

.section + .section {
  margin-top: 24px;
}
.section h3 {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-muted);
  font-weight: 500;
  margin: 8px 0 12px;
  letter-spacing: 0.4px;
}
.section h3 em {
  font-style: normal;
  font-size: 11px;
  color: var(--text-muted);
  background: var(--bg-elev);
  padding: 2px 7px;
  border-radius: 99px;
}
.section h3 .hint {
  margin-left: auto;
  font-size: 11px;
  color: var(--accent);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 10px;
}

.app-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.18s;
  position: relative;
  overflow: hidden;
}
.app-card:hover {
  border-color: var(--accent);
  transform: translateY(-1px);
  box-shadow: var(--shadow-card);
}
.logo {
  width: 38px;
  height: 38px;
  border-radius: var(--radius-md);
  display: grid;
  place-items: center;
  font-weight: 600;
  font-size: 16px;
  flex-shrink: 0;
}
.logo.gradient {
  background: linear-gradient(135deg, var(--accent), var(--accent-soft));
  color: #fff;
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
.app-card:hover .actions {
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
  color: var(--text);
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
  gap: 12px;
  padding: 60px 20px;
  color: var(--text-muted);
}
.empty-icon {
  font-size: 48px;
  opacity: 0.4;
}
.error {
  padding: 10px 14px;
  background: rgba(248, 113, 113, 0.12);
  color: var(--danger);
  border-radius: var(--radius-md);
  font-size: 12.5px;
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

/* Modal */
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
.field input {
  width: 100%;
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
