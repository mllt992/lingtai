import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router'
import 'virtual:uno.css'
import './styles/themes.css'
import './styles/main.css'

// HUD 窗口：在 Vue 挂载之前把 html/body 标成 .hud
// 这样 main.css 里的透明规则立即生效，避免显示主题底色一闪
if (typeof window !== 'undefined' && window.location.search.includes('hud=1')) {
  document.documentElement.classList.add('hud')
  document.body.classList.add('hud')
}

const app = createApp(App)
app.use(createPinia())

// HUD 窗口不需要路由（只渲染 HudView），跳过 router 减少开销
if (!window.location.search.includes('hud=1')) {
  app.use(router)
}

app.mount('#app')
