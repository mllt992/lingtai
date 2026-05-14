import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { getCurrentWindow } from '@tauri-apps/api/window'
import App from './App.vue'
import { router } from './router'
import 'virtual:uno.css'
import './styles/themes.css'
import './styles/main.css'

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

const app = createApp(App)
app.use(createPinia())

// HUD 窗口不需要路由（只渲染 HudView），跳过 router 减少开销
if (!isHud) app.use(router)

app.mount('#app')
