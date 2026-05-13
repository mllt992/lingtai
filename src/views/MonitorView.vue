<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { useMonitorStore } from '@/stores/monitor'
import { useSettingsStore } from '@/stores/settings'
import PageHeader from '@/components/PageHeader.vue'
import StatCard from '@/components/StatCard.vue'
import SparkLine from '@/components/SparkLine.vue'

const monitor = useMonitorStore()
const settings = useSettingsStore()

onMounted(() => {
  monitor.start(settings.monitor.refreshMs)
})
onBeforeUnmount(() => {
  monitor.stop()
})

function fmtBytes(n: number) {
  if (n <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.min(units.length - 1, Math.floor(Math.log10(n) / 3))
  return `${(n / Math.pow(1000, i)).toFixed(i >= 2 ? 1 : 0)} ${units[i]}`
}
function fmtPct(used: number, total: number) {
  if (!total) return '—'
  return `${((used / total) * 100).toFixed(1)}%`
}

const cpu = computed(() => monitor.snapshot?.cpu)
const mem = computed(() => monitor.snapshot?.mem)
const gpus = computed(() => monitor.snapshot?.gpus ?? [])
const memPct = computed(() => (mem.value ? (mem.value.used / mem.value.total) * 100 : 0))

const uptime = computed(() => {
  const s = monitor.snapshot?.uptime_secs ?? 0
  const d = Math.floor(s / 86400)
  const h = Math.floor((s % 86400) / 3600)
  const m = Math.floor((s % 3600) / 60)
  return d > 0 ? `${d}天 ${h}时 ${m}分` : `${h}时 ${m}分`
})
</script>

<template>
  <div class="page">
    <PageHeader
      title="性能监控"
      :subtitle="cpu ? `${cpu.brand} · ${cpu.cores} 核 · ${(cpu.frequency_mhz / 1000).toFixed(2)} GHz` : '读取中…'"
    >
      <template #actions>
        <div class="status">
          <span class="dot" :class="{ live: monitor.polling }" />
          {{ monitor.polling ? '实时' : '已暂停' }}
        </div>
        <button class="btn-ghost" @click="monitor.refreshDrives()">
          <span class="i-carbon-renew" /> 刷新磁盘
        </button>
      </template>
    </PageHeader>

    <div class="body scrollbar-thin">
      <div class="stats">
        <StatCard
          label="CPU 总占用"
          :value="cpu ? cpu.total.toFixed(1) + '%' : '—'"
          :hint="`系统运行 ${uptime}`"
          :percent="cpu?.total ?? 0"
          icon="i-carbon-chip"
        />
        <StatCard
          label="内存"
          :value="mem ? fmtBytes(mem.used) + ' / ' + fmtBytes(mem.total) : '—'"
          :hint="mem ? fmtPct(mem.used, mem.total) + ' 已使用' : ''"
          :percent="memPct"
          :accent="'var(--success)'"
          icon="i-carbon-memory"
        />
        <StatCard
          label="交换空间"
          :value="mem ? fmtBytes(mem.swap_used) + ' / ' + fmtBytes(mem.swap_total) : '—'"
          :hint="mem && mem.swap_total ? fmtPct(mem.swap_used, mem.swap_total) + ' 已使用' : '未启用'"
          :percent="mem && mem.swap_total ? (mem.swap_used / mem.swap_total) * 100 : 0"
          :accent="'var(--warning)'"
          icon="i-carbon-data-base"
        />
        <StatCard
          label="GPU"
          :value="gpus.length ? gpus[0].name : '未检测'"
          :hint="gpus.length && gpus[0].utilization != null ? gpus[0].utilization.toFixed(0) + '% 使用率' : (gpus.length ? '驱动 / NVML 不可用' : '—')"
          :percent="gpus[0]?.utilization ?? undefined"
          :accent="'var(--danger)'"
          icon="i-carbon-gpu"
        />
      </div>

      <div class="charts">
        <div class="chart-card">
          <header>
            <h3><span class="i-carbon-chip" /> CPU 折线</h3>
            <span class="kpi">{{ cpu ? cpu.total.toFixed(1) + '%' : '—' }}</span>
          </header>
          <SparkLine :data="monitor.cpuSeries" :max="100" color="var(--accent)" :height="100" />
          <div v-if="cpu && cpu.per_core.length" class="cores">
            <div v-for="(v, i) in cpu.per_core" :key="i" class="core" :title="`Core ${i}: ${v.toFixed(1)}%`">
              <div class="core-bar" :style="{ height: v + '%' }" />
              <span>{{ i }}</span>
            </div>
          </div>
        </div>
        <div class="chart-card">
          <header>
            <h3><span class="i-carbon-memory" /> 内存折线</h3>
            <span class="kpi">{{ memPct.toFixed(1) }}%</span>
          </header>
          <SparkLine :data="monitor.memSeries" :max="100" color="var(--success)" :height="100" />
          <div v-if="mem" class="mem-detail">
            <div>已用: <strong>{{ fmtBytes(mem.used) }}</strong></div>
            <div>剩余: <strong>{{ fmtBytes(mem.total - mem.used) }}</strong></div>
            <div>总量: <strong>{{ fmtBytes(mem.total) }}</strong></div>
          </div>
        </div>
      </div>

      <div class="drives">
        <header>
          <h3><span class="i-carbon-data-volume" /> 磁盘卷</h3>
        </header>
        <div class="drive-grid">
          <div v-for="d in monitor.drives" :key="d.mount" class="drive">
            <div class="drive-head">
              <span class="i-carbon-data-volume drive-ic" />
              <strong>{{ d.mount }}</strong>
              <em>{{ d.file_system }} · {{ d.kind }}</em>
            </div>
            <div class="drive-bar">
              <div
                class="drive-fill"
                :style="{
                  width: ((d.total - d.available) / Math.max(1, d.total) * 100) + '%'
                }"
              />
            </div>
            <div class="drive-foot">
              <span>{{ fmtBytes(d.total - d.available) }} 已用</span>
              <span>{{ fmtBytes(d.available) }} 可用</span>
              <span>{{ fmtBytes(d.total) }} 总计</span>
            </div>
          </div>
        </div>
      </div>

      <div v-if="gpus.length > 0" class="gpus">
        <header>
          <h3><span class="i-carbon-gpu" /> 显卡</h3>
        </header>
        <div class="gpu-grid">
          <div v-for="(g, i) in gpus" :key="i" class="gpu">
            <div class="gpu-head">
              <strong>{{ g.name }}</strong>
              <em>{{ g.vendor }}</em>
            </div>
            <div class="gpu-row">
              <span>使用率</span>
              <strong>{{ g.utilization != null ? g.utilization.toFixed(0) + '%' : '—' }}</strong>
            </div>
            <div class="gpu-row" v-if="g.mem_total">
              <span>显存</span>
              <strong>
                {{ g.mem_used != null ? fmtBytes(g.mem_used) : '—' }} /
                {{ fmtBytes(g.mem_total) }}
              </strong>
            </div>
          </div>
        </div>
      </div>

      <div v-if="monitor.error" class="error">
        <span class="i-carbon-warning" /> {{ monitor.error }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--text-muted);
  font-size: 12.5px;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
}
.dot.live {
  background: var(--success);
  box-shadow: 0 0 0 0 var(--success);
  animation: pulse 1.4s infinite;
}
@keyframes pulse {
  0% { box-shadow: 0 0 0 0 rgba(74,222,128,0.4); }
  70% { box-shadow: 0 0 0 8px rgba(74,222,128,0); }
  100% { box-shadow: 0 0 0 0 rgba(74,222,128,0); }
}
.btn-ghost {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  color: var(--text-muted);
  font-size: 12.5px;
}
.btn-ghost:hover {
  color: var(--text);
  border-color: var(--accent);
}

.body {
  flex: 1;
  overflow-y: auto;
  padding: 16px 24px 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
}

.charts {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
  gap: 12px;
}
.chart-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.chart-card header,
.drives header,
.gpus header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.chart-card h3,
.drives h3,
.gpus h3 {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  margin: 0;
}
.kpi {
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: 600;
  color: var(--accent);
}
.cores {
  display: flex;
  align-items: flex-end;
  gap: 4px;
  height: 50px;
  margin-top: 4px;
}
.core {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-end;
  height: 100%;
  gap: 2px;
}
.core-bar {
  width: 100%;
  background: var(--accent);
  border-radius: 2px 2px 0 0;
  min-height: 1px;
  transition: height 0.3s ease;
}
.core span {
  font-size: 9px;
  color: var(--text-muted);
}
.mem-detail {
  display: flex;
  gap: 16px;
  font-size: 12px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}
.mem-detail strong {
  color: var(--text);
}

.drives,
.gpus {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.drive-grid,
.gpu-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 12px;
}
.drive,
.gpu {
  background: var(--bg-elev);
  border-radius: var(--radius-md);
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.drive-head,
.gpu-head {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}
.drive-ic {
  color: var(--accent);
  font-size: 18px;
}
.drive-head em,
.gpu-head em {
  font-style: normal;
  font-size: 11px;
  color: var(--text-muted);
  margin-left: auto;
}
.drive-bar {
  height: 6px;
  background: var(--bg);
  border-radius: 3px;
  overflow: hidden;
}
.drive-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), var(--success));
  border-radius: 3px;
  transition: width 0.4s;
}
.drive-foot {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}
.gpu-row {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
}
.gpu-row span {
  color: var(--text-muted);
}

.error {
  padding: 10px 14px;
  background: rgba(248, 113, 113, 0.12);
  color: var(--danger);
  border-radius: var(--radius-md);
  font-size: 12.5px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
</style>
