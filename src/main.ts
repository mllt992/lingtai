import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { getCurrentWindow } from '@tauri-apps/api/window'
import App from './App.vue'
import { router } from './router'
import 'virtual:uno.css'
import './styles/themes.css'
import './styles/main.css'

const hasTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

function showFatal(message: string) {
  const root = document.getElementById('app')
  if (!root) return
  const box = document.createElement('pre')
  box.style.cssText = [
    'margin:16px',
    'padding:12px',
    'white-space:pre-wrap',
    'word-break:break-word',
    'font:12px/1.5 Consolas,monospace',
    'color:#5c1a1a',
    'background:#fff4f4',
    'border:1px solid #f0b4b4',
    'border-radius:8px'
  ].join(';')
  box.textContent = `Loft 前端启动失败\n\n${message}`
  root.replaceChildren(box)
}

window.addEventListener('error', (e) => {
  console.error('[loft] window error', e)
})
window.addEventListener('unhandledrejection', (e) => {
  console.error('[loft] unhandled rejection', e.reason)
})

// 通过 Tauri window label 判定当前是 HUD 窗口还是主窗口。
// HUD 窗口需要透明 body —— 在 Vue 挂载之前就标记好，避免主题色一闪。
let isHud = false
try {
  isHud = getCurrentWindow().label === 'hud'
} catch {
  // 不在 Tauri 上下文（比如纯浏览器 dev），按主窗口处理
}

if (isHud) {
  document.documentElement.classList.add('hud')
  document.body.classList.add('hud')
}

if (!hasTauri && !isHud) {
  showFatal(
    [
      'window.__TAURI_INTERNALS__ 不存在。',
      '当前地址: ' + location.href,
      '这通常表示页面在普通浏览器里打开，而不是 Loft 主窗口。',
      '请用 pnpm tauri:dev 启动，或从托盘打开主窗口。'
    ].join('\n')
  )
} else {
  try {
    const app = createApp(App)
    app.use(createPinia())

    // HUD 窗口不需要路由（只渲染 HudView），跳过 router 减少开销
    if (!isHud) app.use(router)

    app.config.errorHandler = (err, _instance, info) => {
      console.error('[loft] vue error', info, err)
    }

    app.mount('#app')
  } catch (err) {
    console.error('[loft] mount failed', err)
    showFatal(err instanceof Error ? `${err.name}: ${err.message}\n${err.stack ?? ''}` : String(err))
  }
}
