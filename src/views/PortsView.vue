<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { usePortsStore } from '@/stores/ports'
import { useSettingsStore } from '@/stores/settings'
import { invoke } from '@tauri-apps/api/core'
import PageHeader from '@/components/PageHeader.vue'
import type { PortEntry } from '@/types'

const ports = usePortsStore()
const settings = useSettingsStore()
const confirmKill = ref<PortEntry | null>(null)

onMounted(() => {
  ports.includeSystem = settings.ports.includeSystem
  ports.start(settings.ports.refreshMs)
})
onBeforeUnmount(() => ports.stop())

function protoColor(p: string) {
  return p === 'TCP' ? 'var(--accent)' : 'var(--success)'
}

async function openProcessFolder(entry: PortEntry) {
  if (!entry.process_path) return
  await invoke('reveal_in_explorer', { path: entry.process_path })
}

async function doKill() {
  if (!confirmKill.value?.pid) return
  await ports.kill(confirmKill.value.pid)
  confirmKill.value = null
}
</script>

<template>
  <div class="page">
    <PageHeader
      title="端口监控"
      :subtitle="`默认隐藏系统进程 · 每 ${(ports.intervalMs / 1000).toFixed(0)} 秒刷新`"
    >
      <template #actions>
        <div class="search">
          <span class="i-carbon-search" />
          <input v-model="ports.keyword" placeholder="搜索 端口 / 进程 / 协议…" />
        </div>
        <label class="toggle">
          <input
            type="checkbox"
            :checked="ports.includeSystem"
            @change="ports.setIncludeSystem(($event.target as HTMLInputElement).checked)"
          />
          <span>显示系统进程</span>
        </label>
        <button class="btn-ghost" :disabled="ports.loading" @click="ports.refresh()">
          <span class="i-carbon-renew" :class="{ spin: ports.loading }" />
          <span>刷新</span>
        </button>
      </template>
    </PageHeader>

    <div class="body scrollbar-thin">
      <div v-if="ports.error" class="error">
        <span class="i-carbon-warning" /> {{ ports.error }}
      </div>

      <div class="table">
        <div class="thead">
          <div class="th proto">协议</div>
          <div class="th port">端口</div>
          <div class="th addr">本地地址</div>
          <div class="th proc">进程</div>
          <div class="th pid">PID</div>
          <div class="th state">状态</div>
          <div class="th act">操作</div>
        </div>
        <div class="tbody">
          <div
            v-for="(e, idx) in ports.filtered"
            :key="`${e.protocol}-${e.local_port}-${idx}`"
            class="tr"
            :class="{ sys: e.is_system }"
          >
            <div class="td proto">
              <span class="badge" :style="{ background: protoColor(e.protocol) }">
                {{ e.protocol }}
              </span>
            </div>
            <div class="td port mono">{{ e.local_port }}</div>
            <div class="td addr mono">{{ e.local_addr }}</div>
            <div class="td proc">
              <div class="proc-name">{{ e.process_name || '—' }}</div>
              <div v-if="e.process_path" class="proc-path">{{ e.process_path }}</div>
            </div>
            <div class="td pid mono">{{ e.pid ?? '—' }}</div>
            <div class="td state">{{ e.state }}</div>
            <div class="td act">
              <button
                v-if="e.process_path"
                class="icon-btn"
                title="打开所在目录"
                @click="openProcessFolder(e)"
              >
                <span class="i-carbon-folder-open" />
              </button>
              <button
                v-if="e.pid && !e.is_system"
                class="icon-btn danger"
                title="结束进程"
                @click="confirmKill = e"
              >
                <span class="i-carbon-stop" />
              </button>
            </div>
          </div>
          <div v-if="ports.filtered.length === 0" class="empty">
            <span class="i-carbon-network-1" />
            <p>{{ ports.loading ? '正在读取端口…' : '当前没有匹配的用户进程端口' }}</p>
          </div>
        </div>
      </div>
    </div>

    <transition name="fade">
      <div v-if="confirmKill" class="modal-mask" @click.self="confirmKill = null">
        <div class="modal">
          <header>
            <h2>结束进程</h2>
          </header>
          <div class="body">
            <p>
              将要结束进程
              <strong>{{ confirmKill.process_name }}</strong>
              (PID <strong>{{ confirmKill.pid }}</strong>)，确认继续？
            </p>
            <p class="warn">
              <span class="i-carbon-warning-alt" /> 操作不可撤销，进程内未保存数据可能会丢失。
            </p>
          </div>
          <footer>
            <button class="btn-ghost" @click="confirmKill = null">取消</button>
            <button class="btn-danger" @click="doKill">
              <span class="i-carbon-stop" /> 确认结束
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
.toggle {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--text-muted);
  font-size: 12.5px;
  cursor: pointer;
}
.toggle input {
  accent-color: var(--accent);
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
  grid-template-columns: 70px 80px 1.4fr 1.8fr 70px 110px 80px;
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
  transition: background 0.15s;
}
.tr:last-child {
  border-bottom: none;
}
.tr:hover {
  background: var(--bg-elev);
}
.tr.sys {
  opacity: 0.6;
}
.td {
  min-width: 0;
}
.mono {
  font-family: var(--font-mono);
}
.badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  color: #fff;
  font-size: 11px;
  font-weight: 600;
}
.proc-name {
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.proc-path {
  font-size: 10.5px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: var(--font-mono);
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
  transition: all 0.15s;
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
  to { transform: rotate(360deg); }
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
  background: rgba(0,0,0,0.5);
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
.modal .body {
  padding: 16px 18px;
  font-size: 13px;
  overflow: visible;
}
.modal .body p {
  margin: 0 0 10px;
}
.modal .body .warn {
  display: inline-flex;
  align-items: center;
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
</style>
