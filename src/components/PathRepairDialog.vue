<script setup lang="ts">
import { computed } from 'vue'
import { useLauncherStore } from '@/stores/launcher'

const launcher = useLauncherStore()
const open = computed(() => !!launcher.pendingPathRepair)
const req = computed(() => launcher.pendingPathRepair)

function choose(choice: 'repair-open' | 'open-once' | 'cancel') {
  launcher.resolvePendingPathRepair(choice)
}
</script>

<template>
  <transition name="dlg">
    <div v-if="open && req" class="mask" @click.self="choose('cancel')">
      <div class="dialog">
        <header>
          <h3>修复路径？</h3>
        </header>
        <div class="body">
          <p class="lead">
            「{{ req.name }}」的绝对路径已失效，但相对路径可用。
          </p>
          <div class="paths">
            <div class="row">
              <span class="lbl">原路径</span>
              <code class="mono old">{{ req.oldPath }}</code>
            </div>
            <div class="row">
              <span class="lbl">可用路径</span>
              <code class="mono ok">{{ req.newPath }}</code>
            </div>
          </div>
          <p class="hint">修复将更新条目的绝对路径（不改写已缓存图标）。</p>
        </div>
        <footer>
          <button class="btn-ghost" @click="choose('cancel')">取消</button>
          <button class="btn-ghost" @click="choose('open-once')">仅本次打开</button>
          <button class="btn-primary" @click="choose('repair-open')">修复并打开</button>
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
  z-index: 320;
}
.dialog {
  width: min(440px, calc(100vw - 32px));
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  overflow: hidden;
}
header {
  padding: 14px 16px 0;
}
header h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}
.body {
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.lead {
  margin: 0;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.5;
}
.paths {
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px;
}
.row {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.lbl {
  font-size: 11px;
  color: var(--text-muted);
}
.mono {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 12px;
  word-break: break-all;
  line-height: 1.4;
}
.old {
  color: var(--text-muted);
  text-decoration: line-through;
}
.ok {
  color: var(--success, var(--accent));
}
.hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-muted);
}
footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 0 16px 14px;
}
.btn-ghost,
.btn-primary {
  height: 32px;
  padding: 0 12px;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
}
.btn-ghost {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-muted);
}
.btn-ghost:hover {
  color: var(--text);
  border-color: var(--accent);
}
.btn-primary {
  background: var(--accent);
  border: 1px solid var(--accent);
  color: #fff;
}
.dlg-enter-active,
.dlg-leave-active {
  transition: opacity 0.15s ease;
}
.dlg-enter-from,
.dlg-leave-to {
  opacity: 0;
}
</style>
